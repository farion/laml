//! Versioned, owned editor service messages. No HTTP endpoints are exposed here.
//! Addresses belong to the backend. A future network host must translate/scope
//! them and compiler-origin paths; native absolute addresses are not a web API.
use super::Presentation;
use crate::{diagnostic::Diagnostic, output::Format, preview::Preview};
use serde::{Deserialize, Serialize};

pub mod codec;
#[cfg(not(target_arch = "wasm32"))]
mod compiled;
#[cfg(not(target_arch = "wasm32"))]
pub mod dispatch;
#[cfg(not(target_arch = "wasm32"))]
mod gesture;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ticket_tests;
#[cfg(not(target_arch = "wasm32"))]
mod tickets;

pub const VERSION: u32 = 1;
pub const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// Correlation only, never an authentication credential. Revision zero is valid.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub version: u32,
    pub session: u64,
    pub document: u64,
    pub request: u64,
    pub revision: u64,
}
impl Stamp {
    pub fn validate(self) -> Result<(), Failure> {
        if self.version != VERSION {
            return Err(Failure::protocol("Unsupported editor protocol version"));
        }
        if self.session == 0 || self.document == 0 || self.request == 0 {
            return Err(Failure::protocol("Editor correlation IDs must be nonzero"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub stamp: Stamp,
    pub operation: Operation,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub stamp: Stamp,
    pub result: Result<Reply, Failure>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Compile {
        source: String,
        address: String,
        presentation: Presentation,
    },
    RenderExport {
        source: String,
        address: String,
        format: Format,
    },
    OpenSource {
        address: String,
    },
    InspectSource {
        address: String,
    },
    SaveSource {
        address: String,
        source: String,
        expected: Option<String>,
    },
    PrepareExport {
        address: String,
        source_address: String,
        source: String,
        format: Format,
    },
    BeginExport {
        address: String,
        source_address: String,
        source: String,
        format: Format,
    },
    CommitExport {
        ticket: u64,
    },
    CancelExport {
        ticket: u64,
    },
    InspectOutput {
        address: String,
    },
    WriteOutput {
        address: String,
        data: Vec<u8>,
        expected: Option<Vec<u8>>,
    },
    WatchDependencies {
        source: String,
        address: String,
    },
    DependenciesChanged {
        watch: u64,
    },
    SequenceOptions {
        source: String,
        address: String,
        presentation: Presentation,
        queries: Vec<crate::notation_edit::sequence_options::Query>,
        references: bool,
    },
    SequenceTargets {
        source: String,
        address: String,
        presentation: Presentation,
        action: crate::notation_edit::sequence_targets::Action,
        targets: Vec<crate::notation_edit::sequence_targets::Target>,
    },
    GestureRender {
        presentation: Presentation,
        source: String,
        address: String,
        route: Option<Box<laml::route::Route>>,
        bounds: (f64, f64, f64, f64),
        remove_line: Option<usize>,
    },
    RecoverySync {
        address: Option<String>,
        source: String,
        dirty: bool,
    },
    RecoveryList,
    RecoveryDiscard {
        id: String,
    },
    CloseDocument,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Reply {
    RecoveryRecords(Vec<crate::recovery_record::Record>),
    GestureSvg {
        base: Option<String>,
        overlay: Option<String>,
    },
    Compiled(Result<Box<Preview>, Diagnostic>),
    SequenceTargets(Vec<crate::notation_edit::sequence_targets::Choice>),
    SequenceOptions(Vec<crate::notation_edit::sequence_options::Choice>),
    Export(Vec<u8>),
    OpenedSource {
        address: String,
        source: String,
    },
    SourceSnapshot {
        address: String,
        source: Option<String>,
    },
    Address(String),
    OutputSnapshot(Option<Vec<u8>>),
    ExportPrepared {
        ticket: u64,
        address: String,
        exists: bool,
    },
    Unit,
    DependencyWatch {
        watch: u64,
        paths: Vec<String>,
    },
    DependenciesChanged(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyKind {
    RecoveryRecords,
    GestureSvg,
    Compiled,
    SequenceTargets,
    SequenceOptions,
    Export,
    OpenedSource,
    SourceSnapshot,
    Address,
    OutputSnapshot,
    ExportPrepared,
    Unit,
    DependencyWatch,
    DependenciesChanged,
}
impl Operation {
    pub fn discardable(&self) -> bool {
        matches!(
            self,
            Self::Compile { .. }
                | Self::SequenceTargets { .. }
                | Self::SequenceOptions { .. }
                | Self::RenderExport { .. }
                | Self::GestureRender { .. }
        )
    }
    pub fn reply_kind(&self) -> ReplyKind {
        match self {
            Self::RecoveryList => ReplyKind::RecoveryRecords,
            Self::RecoverySync { .. } | Self::RecoveryDiscard { .. } => ReplyKind::Unit,
            Self::GestureRender { .. } => ReplyKind::GestureSvg,
            Self::Compile { .. } => ReplyKind::Compiled,
            Self::SequenceTargets { .. } => ReplyKind::SequenceTargets,
            Self::SequenceOptions { .. } => ReplyKind::SequenceOptions,
            Self::RenderExport { .. } => ReplyKind::Export,
            Self::OpenSource { .. } => ReplyKind::OpenedSource,
            Self::InspectSource { .. } => ReplyKind::SourceSnapshot,
            Self::SaveSource { .. } | Self::PrepareExport { .. } => ReplyKind::Address,
            Self::InspectOutput { .. } => ReplyKind::OutputSnapshot,
            Self::BeginExport { .. } => ReplyKind::ExportPrepared,
            Self::CommitExport { .. } => ReplyKind::Address,
            Self::CancelExport { .. } => ReplyKind::Unit,
            Self::WriteOutput { .. } | Self::CloseDocument => ReplyKind::Unit,
            Self::WatchDependencies { .. } => ReplyKind::DependencyWatch,
            Self::DependenciesChanged { .. } => ReplyKind::DependenciesChanged,
        }
    }
}
impl Reply {
    pub fn kind(&self) -> ReplyKind {
        match self {
            Self::RecoveryRecords(_) => ReplyKind::RecoveryRecords,
            Self::GestureSvg { .. } => ReplyKind::GestureSvg,
            Self::Compiled(_) => ReplyKind::Compiled,
            Self::SequenceTargets(_) => ReplyKind::SequenceTargets,
            Self::SequenceOptions(_) => ReplyKind::SequenceOptions,
            Self::Export(_) => ReplyKind::Export,
            Self::OpenedSource { .. } => ReplyKind::OpenedSource,
            Self::SourceSnapshot { .. } => ReplyKind::SourceSnapshot,
            Self::Address(_) => ReplyKind::Address,
            Self::OutputSnapshot(_) => ReplyKind::OutputSnapshot,
            Self::ExportPrepared { .. } => ReplyKind::ExportPrepared,
            Self::Unit => ReplyKind::Unit,
            Self::DependencyWatch { .. } => ReplyKind::DependencyWatch,
            Self::DependenciesChanged(_) => ReplyKind::DependenciesChanged,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    Protocol,
    Backend,
    Busy,
    Disconnected,
    Timeout,
    Limit,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub code: FailureCode,
    pub message: String,
}
impl Failure {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(FailureCode::Protocol, message)
    }
    pub fn backend(message: impl Into<String>) -> Self {
        Self::new(FailureCode::Backend, message)
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Failure {}

pub fn address(path: &std::path::Path) -> Result<String, Failure> {
    let value = path
        .to_str()
        .ok_or_else(|| Failure::protocol("Backend address is not UTF-8"))?;
    validate_address(value)?;
    Ok(value.into())
}
pub fn validate_address(value: &str) -> Result<(), Failure> {
    if value.is_empty() || value.contains('\0') {
        return Err(Failure::protocol("Invalid backend address"));
    }
    Ok(())
}
