//! Backend-owned rendered export and overwrite snapshot. The frontend confirms
//! a ticket rather than uploading arbitrary bytes or stale dependency assertions.
use super::*;
use crate::backend::{DependencyWatch, Handle};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

type Owner = (u64, u64);
const MAX_TICKET_BYTES: usize = 128 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
struct Ticket {
    id: u64,
    revision: u64,
    path: PathBuf,
    data: Vec<u8>,
    expected: Option<Vec<u8>>,
    watch: Box<dyn DependencyWatch>,
}
impl Ticket {
    fn bytes(&self) -> usize {
        self.data.len() + self.expected.as_ref().map_or(0, Vec::len)
    }
}
#[derive(Default)]
pub(super) struct Tickets {
    entries: BTreeMap<Owner, Ticket>,
    next: u64,
    limits: bool,
}
impl Tickets {
    pub fn set_limits(&mut self, limits: bool) {
        self.limits = limits;
    }
    pub fn begin(
        &mut self,
        backend: &Handle,
        stamp: Stamp,
        target: String,
        source_address: String,
        source: String,
        format: Format,
    ) -> Result<Reply, Failure> {
        let owner = (stamp.session, stamp.document);
        self.entries.remove(&owner);
        let origin = Path::new(&source_address);
        let watch = backend.watch_dependencies(&source, origin);
        let mut protected = watch.paths().to_vec();
        protected.push(
            backend
                .resolve_existing(origin)
                .or_else(|_| backend.resolve_target(origin))
                .map_err(Failure::backend)?,
        );
        let path = backend
            .export_target(Path::new(&target), format, &protected)
            .map_err(Failure::backend)?;
        let output = address(&path)?;
        let expected = backend.output_snapshot(&path).map_err(Failure::backend)?;
        let data = backend
            .render_export(&source, origin, format)
            .map_err(Failure::backend)?;
        let bytes = data
            .len()
            .saturating_add(expected.as_ref().map_or(0, Vec::len));
        if self.limits
            && (bytes > MAX_TICKET_BYTES
                || self.entries.len() >= 16
                || self
                    .entries
                    .values()
                    .map(Ticket::bytes)
                    .sum::<usize>()
                    .saturating_add(bytes)
                    > MAX_TOTAL_BYTES)
        {
            return Err(Failure::new(
                FailureCode::Limit,
                "Export ticket storage limit exceeded",
            ));
        }
        if watch.changed().map_err(Failure::backend)? {
            return Err(Failure::backend(
                "Dependencies changed during rendering; export cancelled",
            ));
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| Failure::new(FailureCode::Limit, "Export ticket IDs exhausted"))?;
        let id = self.next;
        let exists = expected.is_some();
        self.entries.insert(
            owner,
            Ticket {
                id,
                revision: stamp.revision,
                path,
                data,
                expected,
                watch,
            },
        );
        Ok(Reply::ExportPrepared {
            ticket: id,
            address: output,
            exists,
        })
    }
    pub fn commit(&mut self, backend: &Handle, stamp: Stamp, id: u64) -> Result<Reply, Failure> {
        let owner = (stamp.session, stamp.document);
        let ticket = self
            .entries
            .get(&owner)
            .filter(|t| t.id == id && t.revision == stamp.revision)
            .ok_or_else(|| {
                Failure::protocol("Export ticket expired or belongs to another document/revision")
            })?;
        let output = address(&ticket.path)?;
        let ticket = self
            .entries
            .remove(&owner)
            .expect("Validated export ticket");
        if ticket.watch.changed().map_err(Failure::backend)? {
            return Err(Failure::backend(
                "Dependencies changed before export write; export cancelled",
            ));
        }
        backend
            .write_output(&ticket.path, &ticket.data, ticket.expected.as_deref())
            .map_err(Failure::backend)?;
        Ok(Reply::Address(output))
    }
    pub fn cancel(&mut self, stamp: Stamp, id: u64) -> Result<Reply, Failure> {
        let owner = (stamp.session, stamp.document);
        if !self.entries.get(&owner).is_some_and(|t| t.id == id) {
            return Err(Failure::protocol(
                "Export ticket expired or belongs to another document",
            ));
        }
        self.entries.remove(&owner);
        Ok(Reply::Unit)
    }
    pub fn close(&mut self, stamp: Stamp) {
        self.entries.remove(&(stamp.session, stamp.document));
    }
}
