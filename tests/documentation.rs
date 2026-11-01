use laml::{
    export::png, geometry::place, layout::validate_ordering, parser::parse, route::route,
    styles::load, svg::render,
};
use std::fs;
use std::path::Path;

fn text_blocks(guide: &str) -> Vec<String> {
    guide
        .replace("\r\n", "\n")
        .split("```text\n")
        .skip(1)
        .map(|block| block.split("```").next().unwrap().to_owned())
        .collect()
}

fn assert_preview_matches(root: &Path, path: &Path, svg: &str) {
    let name = path.file_stem().unwrap().to_str().unwrap();
    let preview = root.join("docs/images").join(format!("{name}.png"));
    let expected = resvg::tiny_skia::Pixmap::decode_png(&fs::read(&preview).unwrap()).unwrap();
    let actual = resvg::tiny_skia::Pixmap::decode_png(&png(svg).unwrap()).unwrap();
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
            .any(|d| d > 16)
        })
        .count();
    assert!(
        changed * 1000 <= expected.pixels().len() * 3,
        "{} is out of sync",
        preview.display()
    );
}

#[test]
fn guide_examples_are_extracted_from_lf_and_crlf_files() {
    let source = "intro\n```text\nsemantics {\nA\n}\n```\n";
    let crlf = source.replace('\n', "\r\n");
    assert_eq!(text_blocks(source), text_blocks(&crlf));
    assert_eq!(text_blocks(&crlf).len(), 1);
}

#[test]
fn readme_showcases_include_generic_class_and_sequence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    for (kind, minimum_nodes, minimum_relations) in [("generic", 9, 6), ("class", 5, 3)] {
        let name = format!("showcase-{kind}");
        assert!(readme.contains(&format!("examples/{name}.laml")));
        assert!(readme.contains(&format!("docs/images/{name}.png")));
        let source = fs::read_to_string(root.join(format!("examples/{name}.laml"))).unwrap();
        let diagram = parse(&source).unwrap();
        assert!(diagram.nodes.len() >= minimum_nodes, "{kind} nodes");
        assert!(
            diagram.relations.len() >= minimum_relations,
            "{kind} relations"
        );
    }
    let source = fs::read_to_string(root.join("examples/showcase-sequence.laml")).unwrap();
    let sequence = laml::sequence::parse(&source).unwrap();
    assert!(sequence.participants.len() >= 4);
    assert!(sequence.events.len() >= 18);
    assert!(readme.contains("examples/showcase-sequence.laml"));
    assert!(readme.contains("docs/images/showcase-sequence.png"));
}

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
        if laml::interaction_overview::is_overview_source(&source) {
            let doc = laml::interaction_overview::parse(&source).unwrap();
            let styles = load(&doc.graph, &path).unwrap();
            validate_ordering(&doc.graph).unwrap();
            let boxes = place(&doc.graph, &styles).unwrap();
            let routes = route(&doc.graph, &boxes, &styles).unwrap();
            let svg = laml::interaction_overview::render(
                &doc,
                &boxes,
                &routes,
                laml::svg::TextMode::Embedded,
            )
            .unwrap();
            assert_preview_matches(root, &path, &svg);
            continue;
        }
        if laml::communication::is_communication_source(&source) {
            let document = laml::communication::parse(&source)
                .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
            let sheet = load(&document.graph, &path).unwrap();
            validate_ordering(&document.graph).unwrap();
            let boxes = place(&document.graph, &sheet).unwrap();
            let routes = route(&document.graph, &boxes, &sheet).unwrap();
            let svg = laml::communication::render(
                &document,
                &boxes,
                &routes,
                laml::svg::TextMode::Embedded,
            )
            .unwrap();
            assert_preview_matches(root, &path, &svg);
            continue;
        }
        if laml::timing::is_timing_source(&source) {
            let doc = laml::timing::parse(&source).unwrap();
            let input = laml::syntax::Diagram {
                styling: doc.styling.clone(),
                ..Default::default()
            };
            let sheet = load(&input, &path).unwrap();
            let svg = laml::timing::render(&doc, &sheet, laml::svg::TextMode::Embedded).unwrap();
            assert_preview_matches(root, &path, &svg);
            continue;
        }
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
            }
            continue;
        }
        let diagram = parse(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        if path.file_stem().unwrap() == "architecture" {
            assert!(
                diagram.nodes.len() >= 10,
                "first README example must show at least ten nodes"
            );
        }
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
            (actual.width(), actual.height()),
            "{}",
            path.display()
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
    }
}

#[test]
fn sequence_guide_complete_examples_build() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/sequence-diagrams.md")).unwrap();
    let example_source = root.join("examples/sequence-basic.laml");
    let blocks = text_blocks(&guide);
    assert!(blocks.len() >= 2, "missing sequence guide examples");
    for (index, source) in blocks.iter().take(2).enumerate() {
        let diagram = laml::sequence::parse(source)
            .unwrap_or_else(|e| panic!("sequence guide block {index}: {e:?}"));
        let sheet = load(
            &laml::syntax::Diagram {
                styling: diagram.styling.clone(),
                ..Default::default()
            },
            &example_source,
        )
        .unwrap();
        laml::sequence::render::render_with_styles(&diagram, &sheet, laml::svg::TextMode::System)
            .unwrap_or_else(|e| panic!("sequence guide block {index}: {e}"));
    }
}

#[test]
fn class_guide_complete_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/class-object-diagrams.md")).unwrap();
    let blocks = text_blocks(&guide);
    let source = blocks.first().expect("missing class guide example");
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, &root.join("examples/class-basic.laml")).unwrap();
    let nodes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &nodes, &styles).unwrap();
    let image = render(&nodes, &routes).unwrap();
    assert!(png(&image).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&image).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn component_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/component-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing component guide example");
    let diagram = parse(&source).unwrap();
    let styles = load(&diagram, &root.join("examples/component-internal.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn deployment_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/deployment-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing deployment guide example");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, &root.join("examples/deployment-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn state_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/state-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing state guide example");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, &root.join("examples/state-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn usecase_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/usecase-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing use-case guide example");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, &root.join("examples/usecase-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn package_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/package-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing package guide example");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, &root.join("examples/package-architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn timing_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/timing-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing timing guide example");
    let doc = laml::timing::parse(&source).unwrap();
    let input = laml::syntax::Diagram {
        styling: doc.styling.clone(),
        ..Default::default()
    };
    let sheet = load(&input, &root.join("examples/timing-access.laml")).unwrap();
    let svg = laml::timing::render(&doc, &sheet, laml::svg::TextMode::Embedded).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn interaction_overview_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/interaction-overview-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing overview guide example");
    let doc = laml::interaction_overview::parse(&source).unwrap();
    let sheet = load(
        &doc.graph,
        &root.join("examples/interaction-overview-purchase.laml"),
    )
    .unwrap();
    let boxes = place(&doc.graph, &sheet).unwrap();
    let routes = route(&doc.graph, &boxes, &sheet).unwrap();
    let svg =
        laml::interaction_overview::render(&doc, &boxes, &routes, laml::svg::TextMode::Embedded)
            .unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn communication_guide_example_builds_all_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = fs::read_to_string(root.join("docs/communication-diagrams.md")).unwrap();
    let source = text_blocks(&guide)
        .into_iter()
        .next()
        .expect("missing communication guide example");
    let doc = laml::communication::parse(&source).unwrap();
    let sheet = load(&doc.graph, &root.join("examples/communication-order.laml")).unwrap();
    let boxes = place(&doc.graph, &sheet).unwrap();
    let routes = route(&doc.graph, &boxes, &sheet).unwrap();
    let svg =
        laml::communication::render(&doc, &boxes, &routes, laml::svg::TextMode::Embedded).unwrap();
    assert!(png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
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
