//! Source-linked geometry emitted by the renderers for authoring tools.
//! Regions describe the current compilation, never persistent layout state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Region {
    pub id: String,
    pub kind: String,
    pub line: usize,
    /// Closing source event for an execution; absent for an open execution.
    #[serde(default)]
    pub end_line: Option<usize>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub path: Vec<(f64, f64)>,
}
impl Region {
    pub fn rect(id: impl Into<String>, kind: &str, line: usize, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { id: id.into(), kind: kind.into(), line, end_line: None, x, y, width, height, path: vec![] }
    }
    pub fn stroke(id: impl Into<String>, kind: &str, line: usize, path: Vec<(f64, f64)>) -> Self {
        let (mut x, mut y, mut r, mut b) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &(px, py) in &path { x = x.min(px); y = y.min(py); r = r.max(px); b = b.max(py); }
        Self { id: id.into(), kind: kind.into(), line, end_line: None, x, y, width: r - x, height: b - y, path }
    }
}
