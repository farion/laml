//! Opaque recovery address on web; native adapters use a private snapshot path.
use std::path::PathBuf;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub file: PathBuf,
    pub path: Option<PathBuf>,
    pub source: String,
}
