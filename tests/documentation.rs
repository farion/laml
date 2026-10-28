use laml::{
    geometry::place, layout::validate_ordering, parser::parse, route::route, styles::load,
    svg::render,
};
use std::fs;
use std::path::Path;

#[test]
fn every_published_example_checks_and_builds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples = root.join("examples");
    for entry in fs::read_dir(examples).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let diagram = parse(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let sheet = load(&diagram, &path).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        validate_ordering(&diagram).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let boxes = place(&diagram, &sheet).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let routes =
            route(&diagram, &boxes, &sheet).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let svg = render(&boxes, &routes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(svg.starts_with("<svg "), "{}", path.display());
        assert!(svg.ends_with("</svg>\n"), "{}", path.display());
    }
}

#[test]
fn user_documentation_contains_no_speculative_status_notes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in [
        "README.md",
        "docs/getting-started.md",
        "docs/language-reference.md",
        "docs/language-definition.md",
        "examples/architecture.laml",
    ] {
        let text = fs::read_to_string(root.join(file)).unwrap().to_lowercase();
        for marker in [
            "current implementation",
            "not yet implemented",
            "planned",
            "provisional",
            "draft",
            "preview",
            "future work",
            "early subset",
        ] {
            assert!(
                !text.contains(marker),
                "{file} contains speculative phrase {marker:?}"
            );
        }
    }
}
