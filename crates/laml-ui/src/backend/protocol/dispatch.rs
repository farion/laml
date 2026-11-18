//! Trusted in-process dispatch. This is NOT an authorized network handler.
use super::*;
use crate::backend::{DependencyWatch, Handle};
use std::{collections::BTreeMap, path::Path};

type Owner = (u64, u64);
struct Watch {
    id: u64,
    state: Box<dyn DependencyWatch>,
}
pub struct Dispatcher {
    backend: Handle,
    watches: BTreeMap<Owner, Watch>,
    next_watch: u64,
    exports: super::tickets::Tickets,
    gestures: super::gesture::Cache,
    compiled: super::compiled::Cache,
    limits: bool,
}
impl Dispatcher {
    pub fn new(backend: Handle) -> Self {
        Self {
            backend,
            watches: BTreeMap::new(),
            next_watch: 1,
            exports: Default::default(),
            gestures: Default::default(),
            compiled: Default::default(),
            limits: false,
        }
    }
    pub fn with_limits(mut self, limits: bool) -> Self {
        self.limits = limits;
        self.exports.set_limits(limits);
        self.gestures.set_limits(limits);
        self
    }
    pub fn execute(&mut self, request: Request) -> Response {
        let stamp = request.stamp;
        let result =
            codec::validate_request(&request).and_then(|()| self.run(stamp, request.operation));
        Response { stamp, result }
    }
    fn run(&mut self, stamp: Stamp, operation: Operation) -> Result<Reply, Failure> {
        let owner = (stamp.session, stamp.document);
        let backend = &self.backend;
        match operation {
            Operation::RecoverySync { .. }
            | Operation::RecoveryList
            | Operation::RecoveryDiscard { .. } => {
                Err(Failure::protocol("Recovery requires a host backend"))
            }
            Operation::SequenceOptions {
                source,
                address,
                presentation,
                queries,
                references,
            } => Ok(Reply::SequenceOptions(backend.sequence_options(
                &source,
                Path::new(&address),
                presentation,
                &queries,
                references,
            ))),
            Operation::SequenceTargets {
                source,
                address,
                presentation,
                action,
                targets,
            } => Ok(Reply::SequenceTargets(backend.sequence_targets(
                &source,
                Path::new(&address),
                presentation,
                &action,
                &targets,
            ))),
            Operation::GestureRender {
                presentation,
                source,
                address,
                route,
                bounds,
                remove_line,
            } => self.gestures.render(
                backend,
                stamp,
                source,
                address,
                route,
                bounds,
                remove_line,
                presentation,
            ),
            Operation::Compile {
                source,
                address,
                presentation,
            } => Ok(self
                .compiled
                .compile(backend, stamp, source, address, presentation)),

            Operation::RenderExport {
                source,
                address,
                format,
            } => backend
                .render_export(&source, Path::new(&address), format)
                .map(Reply::Export)
                .map_err(Failure::backend),
            Operation::OpenSource { address: input } => {
                let path = backend
                    .resolve_existing(Path::new(&input))
                    .map_err(Failure::backend)?;
                let source = backend.read_source(&path).map_err(Failure::backend)?;
                Ok(Reply::OpenedSource {
                    address: address(&path)?,
                    source,
                })
            }
            Operation::InspectSource { address: input } => {
                let path = backend
                    .resolve_target(Path::new(&input))
                    .map_err(Failure::backend)?;
                let source = backend.source_snapshot(&path).map_err(Failure::backend)?;
                Ok(Reply::SourceSnapshot {
                    address: address(&path)?,
                    source,
                })
            }
            Operation::SaveSource {
                address: input,
                source,
                expected,
            } => {
                let path = backend
                    .resolve_target(Path::new(&input))
                    .map_err(Failure::backend)?;
                let output = address(&path)?;
                backend
                    .write_source(&path, &source, expected.as_deref())
                    .map_err(Failure::backend)?;
                Ok(Reply::Address(output))
            }
            Operation::PrepareExport {
                address: input,
                source_address,
                source,
                format,
            } => {
                // Derive dependency protection here, never accept a caller-supplied
                // list of files that are supposedly safe to overwrite.
                let source_path = Path::new(&source_address);
                let watch = backend.watch_dependencies(&source, source_path);
                let mut protected = watch.paths().to_vec();
                protected.push(
                    backend
                        .resolve_existing(source_path)
                        .or_else(|_| backend.resolve_target(source_path))
                        .map_err(Failure::backend)?,
                );
                let path = backend
                    .export_target(Path::new(&input), format, &protected)
                    .map_err(Failure::backend)?;
                Ok(Reply::Address(address(&path)?))
            }
            Operation::BeginExport {
                address,
                source_address,
                source,
                format,
            } => self
                .exports
                .begin(backend, stamp, address, source_address, source, format),
            Operation::CommitExport { ticket } => self.exports.commit(backend, stamp, ticket),
            Operation::CancelExport { ticket } => self.exports.cancel(stamp, ticket),
            Operation::InspectOutput { address } => backend
                .output_snapshot(Path::new(&address))
                .map(Reply::OutputSnapshot)
                .map_err(Failure::backend),
            Operation::WriteOutput {
                address,
                data,
                expected,
            } => backend
                .write_output(Path::new(&address), &data, expected.as_deref())
                .map(|()| Reply::Unit)
                .map_err(Failure::backend),
            Operation::WatchDependencies {
                source,
                address: input,
            } => {
                if self.limits && !self.watches.contains_key(&owner) && self.watches.len() >= 256 {
                    return Err(Failure::new(
                        FailureCode::Limit,
                        "Too many dependency watches",
                    ));
                }
                let id = self.next_watch;
                self.next_watch = id.checked_add(1).ok_or_else(|| {
                    Failure::new(FailureCode::Limit, "Dependency watch IDs exhausted")
                })?;
                let state = backend.watch_dependencies(&source, Path::new(&input));
                let paths = state
                    .paths()
                    .iter()
                    .map(|p| address(p))
                    .collect::<Result<_, _>>()?;
                self.watches.insert(owner, Watch { id, state });
                Ok(Reply::DependencyWatch { watch: id, paths })
            }
            Operation::DependenciesChanged { watch } => {
                let state = self
                    .watches
                    .get(&owner)
                    .filter(|w| w.id == watch)
                    .ok_or_else(|| {
                        Failure::protocol("Dependency watch expired or belongs to another document")
                    })?;
                state
                    .state
                    .changed()
                    .map(Reply::DependenciesChanged)
                    .map_err(Failure::backend)
            }
            Operation::CloseDocument => {
                self.watches.remove(&owner);
                self.exports.close(stamp);
                self.gestures.close(stamp);
                self.compiled.close(stamp);
                Ok(Reply::Unit)
            }
        }
    }
}
