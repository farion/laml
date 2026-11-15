use super::*;
use crate::{document::Document, files::FileSession, output::Format};

const SOURCE: &str = "settings {\n theme: plain\n}\nsemantics {\n A\n B\n A -> B : call\n}\n";

#[test]
fn presentation_is_explicit_and_exports_keep_the_document_theme() {
    let backend = LocalBackend;
    let path = Path::new("diagram.laml");
    let editor = backend.compile(SOURCE, path, Presentation::Editor).unwrap();
    let document = backend
        .compile(SOURCE, path, Presentation::Document)
        .unwrap();
    assert_eq!(
        editor.diagram.settings.theme,
        laml::theme::Theme::ModernDark
    );
    assert!(editor.diagram.settings.transparent);
    assert_eq!(document.diagram.settings.theme, laml::theme::Theme::Plain);
    assert_eq!(
        backend.render_export(SOURCE, path, Format::Svg).unwrap(),
        document.svg.as_bytes()
    );
    assert_eq!(
        backend
            .without_route(&editor, SOURCE, path, editor.routes[0].source_line)
            .unwrap(),
        crate::preview::without_route(&editor, SOURCE, path, editor.routes[0].source_line).unwrap()
    );
    assert_eq!(
        backend
            .route_overlay(&editor, SOURCE, path, &editor.routes[0], editor.bounds)
            .unwrap(),
        crate::gesture_preview::render(&editor, SOURCE, path, &editor.routes[0], editor.bounds)
            .unwrap()
    );
}

#[test]
fn checked_file_and_output_services_preserve_conflicts_and_no_clobber() {
    let backend = local();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.laml");
    assert_eq!(backend.source_snapshot(&path).unwrap(), None);
    backend.write_source(&path, SOURCE, None).unwrap();
    assert!(backend.write_source(&path, "bad", None).is_err());
    let (mut file, source) = FileSession::open_with_backend(&path, backend.clone()).unwrap();
    let mut document = Document::new(source);
    document.replace_text("mine".into()).unwrap();
    backend
        .write_source(&path, "external", Some(SOURCE))
        .unwrap();
    assert!(file.save(&mut document).is_err());
    assert!(document.is_dirty());
    assert_eq!(backend.read_source(&path).unwrap(), "external");
    let export = backend
        .export_target(
            &dir.path().join("out.svg"),
            Format::Svg,
            std::slice::from_ref(&path),
        )
        .unwrap();
    backend.write_output(&export, b"old", None).unwrap();
    let expected = backend.output_snapshot(&export).unwrap();
    backend
        .write_output(&export, b"external", expected.as_deref())
        .unwrap();
    assert!(
        backend
            .write_output(&export, b"mine", expected.as_deref())
            .is_err()
    );
    assert_eq!(
        backend.output_snapshot(&export).unwrap().unwrap(),
        b"external"
    );
    assert!(
        backend
            .export_target(&export, Format::Svg, std::slice::from_ref(&export))
            .is_err()
    );
    assert!(backend.source_snapshot(dir.path()).is_err());
}

#[test]
fn dependency_watch_is_owned_by_the_backend_and_tracks_missing_imports() {
    let backend = LocalBackend;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diagram.laml");
    let source = "semantics {\n A\n}\nstyling {\n import \"new.lamlstyle\"\n}\n";
    let watch = backend.watch_dependencies(source, &path);
    assert_eq!(
        watch.paths(),
        &[dir.path().canonicalize().unwrap().join("new.lamlstyle")]
    );
    assert!(!watch.changed().unwrap());
    std::fs::write(dir.path().join("new.lamlstyle"), "styling {\n}\n").unwrap();
    assert!(watch.changed().unwrap());
}
