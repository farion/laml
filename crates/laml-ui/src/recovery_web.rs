//! Host-held browser recovery. Never stores source in browser storage.
use crate::backend::{
    async_client::Client,
    protocol::{Operation, Reply, Stamp},
};
pub use crate::recovery_record::Record;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};
pub fn default_directory() -> Option<PathBuf> {
    Some("host-recovery".into())
}
pub struct Store {
    state: RefCell<State>,
    pub closed: bool,
}
struct State {
    client: Client,
    document: u64,
    pending: Option<(Stamp, Snapshot)>,
    acknowledged: Option<Snapshot>,
    list: Option<Stamp>,
    listed: bool,
    records: Vec<Record>,
    error: Option<String>,
}
#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    path: Option<String>,
    source: String,
    dirty: bool,
}
impl State {
    fn poll(&mut self) {
        while let Some(response) = self.client.poll() {
            if self.list == Some(response.stamp) {
                self.list = None;
                self.listed = true;
                match response.result {
                    Ok(Reply::RecoveryRecords(records)) => self.records = records,
                    Err(e) => self.error = Some(e.to_string()),
                    _ => self.error = Some("Invalid recovery reply".into()),
                }
            } else if self
                .pending
                .as_ref()
                .is_some_and(|(s, _)| *s == response.stamp)
            {
                let (_, snapshot) = self.pending.take().unwrap();
                match response.result {
                    Ok(Reply::Unit) => self.acknowledged = Some(snapshot),
                    Err(e) => self.error = Some(e.to_string()),
                    _ => self.error = Some("Invalid recovery acknowledgement".into()),
                }
            } else if let Err(error) = response.result {
                self.error = Some(error.to_string());
            }
        }
    }
}
impl Store {
    pub fn open(_directory: PathBuf) -> Result<Self, String> {
        let services =
            crate::services::Services::browser("/api/editor".into(), crate::web::limits());
        let document = (js_sys::Math::random() * 4_503_599_627_370_496.0) as u64 + 1;
        Ok(Self {
            state: RefCell::new(State {
                client: services.client(),
                document,
                pending: None,
                acknowledged: None,
                list: None,
                listed: false,
                records: vec![],
                error: None,
            }),
            closed: false,
        })
    }
    pub fn checkpoint(
        &self,
        path: Option<&Path>,
        source: &str,
        dirty: bool,
    ) -> Result<bool, String> {
        if self.closed {
            return Ok(true);
        }
        let mut state = self.state.borrow_mut();
        state.poll();
        if let Some(error) = state.error.take() {
            return Err(error);
        }
        let snapshot = Snapshot {
            path: path.map(|p| p.to_string_lossy().into_owned()),
            source: if dirty { source.into() } else { String::new() },
            dirty,
        };
        if state.acknowledged.as_ref() == Some(&snapshot) {
            return Ok(true);
        }
        if state.pending.is_none() {
            let document = state.document;
            let stamp = state
                .client
                .submit(
                    document,
                    0,
                    Operation::RecoverySync {
                        address: snapshot.path.clone(),
                        source: snapshot.source.clone(),
                        dirty,
                    },
                )
                .map_err(|e| e.to_string())?;
            state.pending = Some((stamp, snapshot));
        }
        Ok(false)
    }
    pub fn clear(&self) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        state.poll();
        if let Some((stamp, _)) = state.pending.take() {
            state.client.forget(stamp);
        }
        let document = state.document;
        state
            .client
            .submit(
                document,
                0,
                Operation::RecoverySync {
                    address: None,
                    source: String::new(),
                    dirty: false,
                },
            )
            .map_err(|e| e.to_string())?;
        state.acknowledged = None;
        Ok(())
    }
    pub fn pending(&self) -> Result<Vec<Record>, String> {
        let mut state = self.state.borrow_mut();
        state.poll();
        if let Some(error) = state.error.take() {
            return Err(error);
        }
        // Discover recovery candidates once when this editor opens. Later
        // orphans belong to the next opening, rather than interrupting editing.
        if !state.listed && state.list.is_none() {
            let document = state.document;
            state.list = Some(
                state
                    .client
                    .submit(document, 0, Operation::RecoveryList)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(std::mem::take(&mut state.records))
    }
    pub fn discard(&self, record: &Record) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        let document = state.document;
        state
            .client
            .submit(
                document,
                0,
                Operation::RecoveryDiscard {
                    id: record.file.to_string_lossy().into_owned(),
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
