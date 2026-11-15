use super::*;
use crate::backend::{Backend, Presentation};
use std::path::{Path, PathBuf};
const SOURCE: &str = "semantics {\n A\n B\n A -> B\n}\n";
fn backend() -> (tempfile::TempDir, scoped::Scoped) {
    let root = tempfile::tempdir().unwrap();
    let workspace = Arc::new(
        workspace::Workspace::open(root.path())
            .unwrap()
            .with_limits(true),
    );
    (root, scoped::Scoped::new(workspace))
}
#[test]
fn paths_are_capability_relative_and_file_types_are_role_checked() {
    let (root, backend) = backend();
    std::fs::write(root.path().join("source.laml"), SOURCE).unwrap();
    assert_eq!(
        backend.read_source(Path::new("./source.laml")).unwrap(),
        SOURCE
    );
    for path in [
        "../source.laml",
        "/etc/passwd",
        "C:\\outside.laml",
        ".git/config",
        "source\n.laml",
    ] {
        assert!(backend.read_source(Path::new(path)).is_err());
    }
    assert!(
        backend
            .write_source(Path::new("code.rs"), SOURCE, None)
            .is_err()
    );
    assert!(backend.output_snapshot(Path::new("source.laml")).is_err());
    assert!(
        backend
            .export_target(Path::new("source.laml"), crate::output::Format::Svg, &[])
            .is_err()
    );
    assert!(
        backend
            .export_target(
                Path::new("out.svg"),
                crate::output::Format::Svg,
                &[PathBuf::from("out.svg")]
            )
            .is_err()
    );
}
#[test]
fn checked_writes_preserve_conflicts_read_only_flags_and_permissions() {
    let (root, backend) = backend();
    let path = Path::new("source.laml");
    backend.write_source(path, SOURCE, None).unwrap();
    assert!(backend.write_source(path, "other", None).is_err());
    assert!(backend.write_source(path, "other", Some("stale")).is_err());
    assert_eq!(backend.read_source(path).unwrap(), SOURCE);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            root.path().join(path),
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
    }
    backend.write_source(path, "new", Some(SOURCE)).unwrap();
    let original = std::fs::metadata(root.path().join(path))
        .unwrap()
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(original.mode() & 0o777, 0o640);
    }
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(root.path().join(path), readonly).unwrap();
    assert!(
        backend
            .write_source(path, "not applied", Some("new"))
            .is_err()
    );
    std::fs::set_permissions(root.path().join(path), original).unwrap();
    assert_eq!(backend.read_source(path).unwrap(), "new");
}
#[test]
fn staged_compilation_translates_origins_and_keeps_unrelated_backslashes() {
    let (root, backend) = backend();
    std::fs::write(
        root.path().join("theme.lamlstyle"),
        "styling {\n node {\n fill: #ff0000\n }\n}\n",
    )
    .unwrap();
    let source = format!("{SOURCE}styling {{\n import \"theme.lamlstyle\"\n}}\n");
    let preview = backend
        .compile(&source, Path::new("source.laml"), Presentation::Editor)
        .unwrap();
    assert!(
        preview
            .styles
            .rules
            .iter()
            .flat_map(|r| &r.declarations)
            .all(|d| d.path.is_relative())
    );
    let encoded = serde_json::to_string(&preview).unwrap();
    assert!(!encoded.contains(&root.path().display().to_string()));
    let stage = stage::Stage::new(&backend.workspace, SOURCE, Path::new("source.laml")).unwrap();
    assert_eq!(
        stage.translate("keep\\slashes".to_owned()).unwrap(),
        "keep\\slashes"
    );
    let error = backend
        .compile(
            "semantics {\n A -> Missing\n}\n",
            Path::new("source.laml"),
            Presentation::Editor,
        )
        .unwrap_err();
    assert_eq!(error.path, Some(PathBuf::from("source.laml")));
    assert_eq!(error.line, Some(2));
}
#[test]
fn export_uses_the_exact_frozen_dependency_snapshot() {
    let (root, backend) = backend();
    let style = root.path().join("theme.lamlstyle");
    std::fs::write(&style, "styling {\n node {\n fill: #ff0000\n }\n}\n").unwrap();
    let source = format!("{SOURCE}styling {{\n import \"theme.lamlstyle\"\n}}\n");
    let watch = backend.watch_dependencies(&source, Path::new("source.laml"));
    std::fs::write(&style, "styling {\n node {\n fill: #0000ff\n }\n}\n").unwrap();
    let svg = String::from_utf8(
        backend
            .render_export(
                &source,
                Path::new("source.laml"),
                crate::output::Format::Svg,
            )
            .unwrap(),
    )
    .unwrap();
    assert!(svg.contains("#ff0000"));
    assert!(watch.changed().unwrap());
}
#[test]
fn unsafe_nested_styles_fonts_and_oversized_dependencies_fail_before_compiler_io() {
    let (root, backend) = backend();
    for source in [
        format!("{SOURCE}styling {{\n import \"../outside.lamlstyle\"\n}}\n"),
        format!("settings {{\n font-file: \"/outside.ttf\"\n}}\n{SOURCE}"),
    ] {
        assert!(
            backend
                .compile(&source, Path::new("source.laml"), Presentation::Editor)
                .is_err()
        );
    }
    std::fs::write(
        root.path().join("outer.lamlstyle"),
        "styling {\n import \"../outside.lamlstyle\"\n}\n",
    )
    .unwrap();
    let source = format!("{SOURCE}styling {{\n import \"outer.lamlstyle\"\n}}\n");
    assert!(
        backend
            .compile(&source, Path::new("source.laml"), Presentation::Editor)
            .is_err()
    );
    let font = std::fs::File::create(root.path().join("large.ttf")).unwrap();
    font.set_len(workspace::FILE_LIMIT as u64 + 1).unwrap();
    let source = format!("settings {{\n font-file: \"large.ttf\"\n}}\n{SOURCE}");
    assert!(
        backend
            .compile(&source, Path::new("source.laml"), Presentation::Editor)
            .is_err()
    );
}
#[test]
#[cfg(unix)]
fn symlinked_files_directories_styles_and_output_cannot_escape_the_workspace() {
    use std::os::unix::fs::symlink;
    let (root, backend) = backend();
    let outside = tempfile::tempdir().unwrap();
    let external = outside.path().join("outside.laml");
    std::fs::write(&external, SOURCE).unwrap();
    symlink(&external, root.path().join("escape.laml")).unwrap();
    symlink(outside.path(), root.path().join("directory")).unwrap();
    for path in ["escape.laml", "directory/outside.laml"] {
        assert!(backend.read_source(Path::new(path)).is_err());
        assert!(
            backend
                .write_source(Path::new(path), "not applied", Some(SOURCE))
                .is_err()
        );
    }
    symlink(&external, root.path().join("out.svg")).unwrap();
    assert!(
        backend
            .write_output(
                Path::new("out.svg"),
                b"not applied",
                Some(SOURCE.as_bytes())
            )
            .is_err()
    );
    std::fs::write(outside.path().join("style.lamlstyle"), "styling {\n}\n").unwrap();
    symlink(
        outside.path().join("style.lamlstyle"),
        root.path().join("style.lamlstyle"),
    )
    .unwrap();
    let source = format!("{SOURCE}styling {{\n import \"style.lamlstyle\"\n}}\n");
    assert!(
        backend
            .compile(&source, Path::new("source.laml"), Presentation::Editor)
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&external).unwrap(), SOURCE);
}
#[test]
fn element_and_source_limits_are_opt_in() {
    let root = tempfile::tempdir().unwrap();
    let source = format!(
        "semantics {{\n{}}}\n",
        (0..513).map(|i| format!(" N{i}\n")).collect::<String>()
    );
    let unlimited = Arc::new(workspace::Workspace::open(root.path()).unwrap());
    assert!(stage::Stage::new(&unlimited, &source, Path::new("source.laml")).is_ok());
    let limited = Arc::new(
        workspace::Workspace::open(root.path())
            .unwrap()
            .with_limits(true),
    );
    assert!(stage::Stage::new(&limited, &source, Path::new("source.laml")).is_err());
    let data = "x".repeat(workspace::SOURCE_LIMIT + 1);
    let open = scoped::Scoped::new(unlimited);
    open.write_source(Path::new("large.laml"), &data, None)
        .unwrap();
    assert_eq!(
        open.read_source(Path::new("large.laml")).unwrap().len(),
        data.len()
    );
    assert!(
        scoped::Scoped::new(limited)
            .read_source(Path::new("large.laml"))
            .is_err()
    );
}
#[test]
fn public_origin_accepts_http_unless_https_is_explicitly_required() {
    let mut config = Config {
        browser: false,
        max_users: 10,
        workspace: "workspace".into(),
        assets: "assets".into(),
        state: "state".into(),
        bind: "0.0.0.0:7878".parse().unwrap(),
        origin: None,
        limits: false,
        parallelism: None,
        http_checks: false,
        require_https: false,
    };
    let local = config.bind;
    assert!(config.public_origin(local).is_ok());
    config.origin = Some("http://diagrams.example".into());
    assert!(config.public_origin(local).is_ok());
    config.require_https = true;
    assert!(config.public_origin(local).is_err());
    config.origin = Some("https://diagrams.example".into());
    assert!(config.public_origin(local).is_ok());
    config.origin = Some("https://diagrams.example/path".into());
    assert!(config.public_origin(local).is_err());
}
