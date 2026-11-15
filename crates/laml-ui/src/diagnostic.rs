//! Compiler-reported locations, never inferred by scraping error-message strings.
use std::{
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
}
impl Diagnostic {
    pub fn at(path: &Path, line: usize, message: impl fmt::Display) -> Self {
        Self {
            message: format!("Line {line}: {message}"),
            path: Some(path.into()),
            line: (line > 0).then_some(line),
        }
    }
    pub fn in_file(path: PathBuf, line: usize, message: impl fmt::Display) -> Self {
        Self {
            message: format!("{}:{line}: {message}", path.display()),
            path: Some(path),
            line: (line > 0).then_some(line),
        }
    }
    /// The compiler currently supplies lines, not token spans. Highlight precisely
    /// that whole source line, and never a same-numbered line in an imported file.
    pub fn range(&self, source: &str, active: &Path) -> Option<Range<usize>> {
        let path = self.path.as_ref()?;
        let same = path == active
            || path
                .canonicalize()
                .ok()
                .zip(active.canonicalize().ok())
                .is_some_and(|(a, b)| a == b);
        if !same {
            return None;
        }
        crate::source::SourceIndex::new(source).line(self.line?)
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl From<String> for Diagnostic {
    fn from(message: String) -> Self {
        Self {
            message,
            path: None,
            line: None,
        }
    }
}
impl From<&str> for Diagnostic {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_real_locations_in_the_active_document_are_marked() {
        let active = Path::new("test.laml");
        let source = "ä\r\nB\r\n";
        assert_eq!(
            Diagnostic::at(active, 2, "bad").range(source, active),
            Some(4..5)
        );
        assert_eq!(
            Diagnostic::at(active, 0, "global").range(source, active),
            None
        );
        assert_eq!(
            Diagnostic::at(active, 9, "outside").range(source, active),
            None
        );
        assert_eq!(
            Diagnostic::in_file("external.lamlstyle".into(), 2, "bad").range(source, active),
            None
        );
        assert_eq!(
            Diagnostic::from("Line 2: unlocated message").range(source, active),
            None
        );
    }
    #[test]
    fn preview_preserves_parse_and_import_locations() {
        let dir = tempfile::tempdir().unwrap();
        let active = dir.path().join("test.laml");
        let source = "semantics {\n A -> Missing\n}\n";
        let error = crate::preview::build_diagnostic(source, &active).unwrap_err();
        assert_eq!(error.range(source, &active), Some(12..25));
        std::fs::write(
            dir.path().join("bad.lamlstyle"),
            "styling {\n node {\n fill: broken\n }\n}\n",
        )
        .unwrap();
        let source = "semantics {\n A\n}\nstyling {\n import \"bad.lamlstyle\"\n}\n";
        let error = crate::preview::build_diagnostic(source, &active).unwrap_err();
        assert!(error.path.unwrap().ends_with("bad.lamlstyle"));
    }
}
