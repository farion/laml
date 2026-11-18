//! Isolated compiler process. Parent enforces a wall deadline and kills on timeout.
use super::{
    scoped::Scoped,
    workspace::{SOURCE_LIMIT, Workspace, relative},
};
use crate::{
    backend::protocol::{
        self, Failure, Operation, Reply, Request, Response, codec, dispatch::Dispatcher,
    },
    recovery::{Record, Store},
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
struct Document {
    seen: Instant,
    mutations: BTreeMap<u8, u64>,
    revision: u64,
}
struct Worker {
    dispatcher: Dispatcher,
    workspace: Arc<Workspace>,
    recovery: PathBuf,
    documents: BTreeMap<(u64, u64), Document>,
    stores: BTreeMap<(u64, u64), Store>,
}
impl Worker {
    fn run(&mut self, request: Request) -> Response {
        let stamp = request.stamp;
        let result = self
            .authorize(&request)
            .and_then(|()| self.operation(request));
        Response { stamp, result }
    }
    fn authorize(&mut self, request: &Request) -> Result<(), Failure> {
        let stamp = request.stamp;
        fn network_path(path: &str) -> Result<(), Failure> {
            if path.contains('\\') {
                return Err(Failure::protocol("Workspace addresses use forward slashes"));
            }
            relative(Path::new(path))
                .map(|_| ())
                .map_err(Failure::backend)
        }
        match &request.operation {
            Operation::BeginExport {
                address,
                source_address,
                ..
            }
            | Operation::PrepareExport {
                address,
                source_address,
                ..
            } => {
                network_path(address)?;
                network_path(source_address)?;
            }
            Operation::Compile { address, .. }
            | Operation::RenderExport { address, .. }
            | Operation::OpenSource { address }
            | Operation::InspectSource { address }
            | Operation::SaveSource { address, .. }
            | Operation::InspectOutput { address }
            | Operation::WriteOutput { address, .. }
            | Operation::WatchDependencies { address, .. }
            | Operation::SequenceTargets { address, .. }
            | Operation::SequenceOptions { address, .. }
            | Operation::GestureRender { address, .. } => network_path(address)?,
            Operation::RecoverySync {
                address: Some(address),
                ..
            } => network_path(address)?,
            _ => {}
        }
        let owner = (stamp.session, stamp.document);
        // Browser inactivity is not a document close: background tabs can stop
        // sending requests indefinitely. Keep recovery leases for this worker's
        // lifetime, so their live snapshots never become recovery candidates.
        let expired = self
            .documents
            .iter()
            .filter(|(_, d)| self.workspace.limits && d.seen.elapsed() > Duration::from_secs(120))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for (session, document) in expired {
            self.documents.remove(&(session, document));
            self.dispatcher.execute(Request {
                stamp: protocol::Stamp {
                    session,
                    document,
                    ..stamp
                },
                operation: Operation::CloseDocument,
            });
        }
        if self.workspace.limits
            && self.documents.len() >= 256
            && !self.documents.contains_key(&owner)
        {
            return Err(Failure::backend("Too many active server documents"));
        }
        let doc = self.documents.entry(owner).or_insert(Document {
            seen: Instant::now(),
            mutations: BTreeMap::new(),
            revision: 0,
        });
        doc.seen = Instant::now();
        let family = match &request.operation {
            Operation::SaveSource { .. } => Some(1),
            Operation::BeginExport { .. }
            | Operation::CommitExport { .. }
            | Operation::CancelExport { .. } => Some(2),
            Operation::RecoverySync { .. } => Some(3),
            Operation::RecoveryDiscard { .. } => Some(5),
            Operation::WatchDependencies { .. } => Some(4),
            _ => None,
        };
        if let Some(family) = family {
            let previous = doc.mutations.entry(family).or_default();
            if stamp.request <= *previous || stamp.revision < doc.revision {
                return Err(Failure::protocol("Stale or replayed server mutation"));
            }
            *previous = stamp.request;
            doc.revision = stamp.revision;
        }
        Ok(())
    }
    fn operation(&mut self, request: Request) -> Result<Reply, Failure> {
        let owner = (request.stamp.session, request.stamp.document);
        match request.operation {
            Operation::WriteOutput { .. }
            | Operation::InspectOutput { .. }
            | Operation::PrepareExport { .. }
            | Operation::RenderExport { .. } => Err(Failure::protocol(
                "Operation is not available on the network host",
            )),
            Operation::RecoverySync {
                address,
                source,
                dirty,
            } => {
                if self.workspace.limits && source.len() > SOURCE_LIMIT {
                    return Err(Failure::backend("Recovery exceeds host size limit"));
                }
                let address = address
                    .map(|s| self.workspace.resolve(Path::new(&s), false))
                    .transpose()
                    .map_err(Failure::backend)?;
                if let Some(path) = &address {
                    super::workspace::extension(path, &["laml", "lamlstyle"])
                        .map_err(Failure::backend)?;
                }
                if dirty && self.workspace.limits {
                    let total = std::fs::read_dir(&self.recovery)
                        .map_err(|_| Failure::backend("Recovery storage unavailable"))?
                        .filter_map(Result::ok)
                        .filter_map(|e| e.metadata().ok())
                        .map(|m| m.len())
                        .sum::<u64>();
                    if total > 128 * 1024 * 1024 {
                        return Err(Failure::backend(
                            "Recovery storage is full; save source explicitly",
                        ));
                    }
                }
                self.store(owner)?
                    .sync(
                        address.as_deref().map(super::workspace::wire).as_deref(),
                        &source,
                        dirty,
                    )
                    .map_err(|_| Failure::backend("Host recovery write failed"))?;
                Ok(Reply::Unit)
            }
            Operation::RecoveryList => {
                let records = self
                    .store(owner)?
                    .pending()
                    .map_err(|_| Failure::backend("Host recovery read failed"))?;
                let mut output = vec![];
                let mut size = 0;
                for mut record in records.into_iter().take(if self.workspace.limits {
                    16
                } else {
                    usize::MAX
                }) {
                    if (self.workspace.limits && record.source.len() > SOURCE_LIMIT)
                        || record.path.as_deref().is_some_and(|p| relative(p).is_err())
                    {
                        continue;
                    }
                    size += record.source.len();
                    if self.workspace.limits && size > 16 * 1024 * 1024 {
                        break;
                    }
                    record.file = record
                        .file
                        .file_name()
                        .ok_or_else(|| Failure::protocol("Invalid recovery identity"))?
                        .into();
                    output.push(record);
                }
                Ok(Reply::RecoveryRecords(output))
            }
            Operation::RecoveryDiscard { id } => {
                if (self.workspace.limits && id.len() > 128)
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
                {
                    return Err(Failure::protocol("Invalid recovery identity"));
                }
                let store = self.store(owner)?;
                let records = store
                    .pending()
                    .map_err(|_| Failure::backend("Host recovery read failed"))?;
                let record: Record = records
                    .into_iter()
                    .find(|r| r.file.file_name().and_then(|s| s.to_str()) == Some(&id))
                    .ok_or_else(|| Failure::backend("Recovery snapshot is unavailable"))?;
                store
                    .discard(&record)
                    .map_err(|_| Failure::backend("Host recovery discard failed"))?;
                Ok(Reply::Unit)
            }
            operation => {
                self.dispatcher
                    .execute(Request {
                        operation,
                        ..request
                    })
                    .result
            }
        }
    }
    fn store(&mut self, owner: (u64, u64)) -> Result<&Store, Failure> {
        if !self.stores.contains_key(&owner) {
            if self.workspace.limits && self.stores.len() >= 128 {
                return Err(Failure::backend("Too many live recovery buffers"));
            }
            let store = Store::open(self.recovery.clone())
                .map_err(|_| Failure::backend("Host recovery storage unavailable"))?
                .with_limits(self.workspace.limits);
            self.stores.insert(owner, store);
        }
        Ok(self.stores.get(&owner).unwrap())
    }
}
pub fn main(workspace: PathBuf, recovery: PathBuf, limits: bool) -> Result<(), String> {
    let workspace = Arc::new(Workspace::open(&workspace)?.with_limits(limits));
    let mut worker = Worker {
        dispatcher: Dispatcher::new(Arc::new(Scoped::new(workspace.clone()))).with_limits(limits),
        workspace,
        recovery,
        documents: BTreeMap::new(),
        stores: BTreeMap::new(),
    };
    let (sender, requests) = std::sync::mpsc::channel();
    // Observe parent pipe closure even during blocked compiler IO. This process
    // contains no UI state; accepted filesystem writes may already have completed.
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        loop {
            let mut length = [0; 4];
            if input.read_exact(&mut length).is_err() {
                std::process::exit(0);
            }
            let length = u32::from_be_bytes(length) as usize;
            if limits && length > protocol::MAX_REQUEST_BYTES {
                std::process::exit(1);
            }
            let mut bytes = vec![0; length];
            if input.read_exact(&mut bytes).is_err() || sender.send(bytes).is_err() {
                std::process::exit(1);
            }
        }
    });
    let deadline = Arc::new(std::sync::Mutex::new(None::<Instant>));
    let watchdog = deadline.clone();
    if limits {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if watchdog.lock().is_ok_and(|start| {
                    start.is_some_and(|at| at.elapsed() > Duration::from_secs(40))
                }) {
                    std::process::exit(124);
                }
            }
        });
    }
    let mut output = std::io::stdout().lock();
    while let Ok(bytes) = requests.recv() {
        *deadline.lock().map_err(|_| "Worker clock unavailable")? = Some(Instant::now());
        if limits {
            super::limits::apply()?;
        }
        let request =
            codec::decode_request_with_limits(&bytes, limits).map_err(|e| e.to_string())?;
        let stamp = request.stamp;
        let response = worker.run(request);
        let bytes = codec::encode_response_with_limits(&response, limits)
            .or_else(|_| {
                codec::encode_response(&Response {
                    stamp,
                    result: Err(Failure::backend("Host response exceeds size limit")),
                })
            })
            .map_err(|e| e.to_string())?;
        output
            .write_all(
                &u32::try_from(bytes.len())
                    .map_err(|_| "IPC response exceeds representable length")?
                    .to_be_bytes(),
            )
            .and_then(|()| output.write_all(&bytes))
            .and_then(|()| output.flush())
            .map_err(|_| "Worker output closed")?;
        *deadline.lock().map_err(|_| "Worker clock unavailable")? = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
