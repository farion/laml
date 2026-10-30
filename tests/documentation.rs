use laml::{
    export::png, geometry::place, layout::validate_ordering, parser::parse, route::route,
    styles::load, svg::render,
};
use std::fs;
use std::path::Path;

#[test]
fn every_published_example_checks_and_builds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples = root.join("examples");
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    for entry in fs::read_dir(examples).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        if laml::sequence::is_sequence_source(&source) {
            let diagram = laml::sequence::parse(&source)
                .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
            let sheet = load(
                &laml::syntax::Diagram {
                    styling: diagram.styling.clone(),
                    ..Default::default()
                },
                &path,
            )
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
            let svg = laml::sequence::render::render_with_styles(
                &diagram,
                &sheet,
                laml::svg::TextMode::Embedded,
            )
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(svg.starts_with("<svg "), "{}", path.display());
            let output = png(&svg).unwrap();
            assert!(output.starts_with(b"\x89PNG"));
            let name = path.file_stem().unwrap().to_str().unwrap();
            let preview = root.join("docs/images").join(format!("{name}.png"));
            if preview.exists() {
                let expected =
                    resvg::tiny_skia::Pixmap::decode_png(&fs::read(&preview).unwrap()).unwrap();
                let actual = resvg::tiny_skia::Pixmap::decode_png(&output).unwrap();
                assert_eq!(
                    (expected.width(), expected.height()),
                    (actual.width(), actual.height())
                );
                let changed = expected
                    .pixels()
                    .iter()
                    .zip(actual.pixels())
                    .filter(|(a, b)| {
                        [
                            a.red().abs_diff(b.red()),
                            a.green().abs_diff(b.green()),
                            a.blue().abs_diff(b.blue()),
                            a.alpha().abs_diff(b.alpha()),
                        ]
                        .into_iter()
                        .any(|difference| difference > 16)
                    })
                    .count();
                assert!(
                    changed * 1000 <= expected.pixels().len() * 3,
                    "{} is out of sync with its sequence source",
                    preview.display()
                );
                assert!(readme.contains(&format!("docs/images/{name}.png")));
                assert!(readme.contains(&format!("examples/{name}.laml")));
            }
            continue;
        }
        let diagram = parse(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let sheet = load(&diagram, &path).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        validate_ordering(&diagram).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let boxes = place(&diagram, &sheet).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let routes =
            route(&diagram, &boxes, &sheet).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let svg = render(&boxes, &routes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(svg.starts_with("<svg "), "{}", path.display());
        assert!(svg.ends_with("</svg>\n"), "{}", path.display());
        let name = path.file_stem().unwrap().to_str().unwrap();
        let preview_path = root.join("docs/images").join(format!("{name}.png"));
        let preview =
            fs::read(&preview_path).unwrap_or_else(|e| panic!("{}: {e}", preview_path.display()));
        let expected = resvg::tiny_skia::Pixmap::decode_png(&preview).unwrap();
        let actual = resvg::tiny_skia::Pixmap::decode_png(&png(&svg).unwrap()).unwrap();
        assert_eq!(
            (expected.width(), expected.height()),
            (actual.width(), actual.height())
        );
        // Cross-platform rasterizers can round a few antialiased edge pixels
        // differently; large visible changes must still update the screenshot.
        let changed = expected
            .pixels()
            .iter()
            .zip(actual.pixels())
            .filter(|(a, b)| {
                [
                    a.red().abs_diff(b.red()),
                    a.green().abs_diff(b.green()),
                    a.blue().abs_diff(b.blue()),
                    a.alpha().abs_diff(b.alpha()),
                ]
                .into_iter()
                .any(|difference| difference > 16)
            })
            .count();
        assert!(
            changed * 1000 <= expected.pixels().len() * 3,
            "{} is out of sync with its LAML source ({changed} changed pixels)",
            preview_path.display()
        );
        assert!(
            readme.contains(&format!("docs/images/{name}.png")),
            "{name}: image is missing from README"
        );
        assert!(
            readme.contains(&format!("examples/{name}.laml")),
            "{name}: source is missing from README"
        );
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
