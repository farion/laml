use laml::{
    geometry::place,
    parser::parse,
    route::route,
    styles::load,
    svg::{TextMode, render_with_theme},
};
use std::path::Path;

fn image(
    source: &str,
) -> (
    String,
    Vec<laml::geometry::BoxNode>,
    Vec<laml::route::Route>,
) {
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/showcase-generic.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg =
        render_with_theme(&boxes, &routes, TextMode::Embedded, diagram.settings.theme).unwrap();
    (svg, boxes, routes)
}

#[test]
fn modern_arrows_end_at_circle_edge_instead_of_its_center() {
    let source = "settings {\ntheme: modern\n}\nsemantics {\nA {\nport In\n}\nB\nB --> A::In\n}\nlayout {\nB < A : gap=140\nport[A::In] : side=left\n}\n";
    let (svg, boxes, routes) = image(source);
    let port = &boxes[0].ports[0];
    let route = &routes[0];
    assert!((route.points.last().unwrap().x - port.x).abs() < 1e-6); // routing still meets port center
    let arrow = svg
        .lines()
        .find(|line| line.contains("marker-end=\"url(#arrow)\""))
        .unwrap();
    assert!(
        arrow.contains(&format!("{:.2},{:.2}", port.x - 6.0, port.y)),
        "{arrow}"
    );
    assert!(
        !arrow.contains(&format!("{:.2},{:.2}\"", port.x, port.y)),
        "{arrow}"
    );
    assert!(svg.contains("markerHeight=\"10\""));
    assert!(svg.contains("data-port-id=\"A::In\""));
    assert!(laml::export::png(&svg).is_ok());
    assert!(laml::export::pdf(&svg).is_ok());
}

#[test]
fn plain_arrows_end_at_square_border_in_light_and_dark() {
    for theme in ["plain", "plain-dark"] {
        let source = format!(
            "settings {{\ntheme: {theme}\n}}\nsemantics {{\nA {{\nport In\n}}\nB\nB --> A::In\n}}\nlayout {{\nB < A : gap=140\nport[A::In] : side=left\n}}\n"
        );
        let (svg, boxes, routes) = image(&source);
        let port = &boxes[0].ports[0];
        assert!((routes[0].points.last().unwrap().x - port.x).abs() < 1e-6);
        let arrow = svg
            .lines()
            .find(|line| line.contains("marker-end=\"url(#arrow)\""))
            .unwrap();
        let edge = port.x - 5.0 - boxes[0].border_width / 2.0;
        assert!(
            arrow.contains(&format!("{edge:.2},{:.2}", port.y)),
            "{theme}: {arrow}"
        );
        assert!(
            !arrow.contains(&format!("{:.2},{:.2}\"", port.x, port.y)),
            "{theme}: arrow still meets the port center: {arrow}"
        );
        assert!(svg.contains("data-port-id=\"A::In\""));
        assert!(laml::export::png(&svg).is_ok());
        assert!(laml::export::pdf(&svg).is_ok());
    }
}

#[test]
fn modern_generic_note_is_rounded_without_fold_and_has_flush_full_height_accent() {
    let source = "semantics {\nnote Memo {\nImportant\n}\n}\n";
    let (svg, boxes, _) = image(source);
    let note = &boxes[0];
    assert!(note.note_card);
    assert!(svg.contains("data-note-card=\"true\""));
    assert!(svg.contains("data-note-accent=\"true\""));
    assert!(svg.contains(&format!(
        "x=\"{:.2}\" y=\"{:.2}\" width=\"6\" height=\"{:.2}\"",
        note.x, note.y, note.height
    )));
    assert!(svg.contains("clip-path=\"url(#laml-note-accent-0)\""));
    assert!(!svg.contains("stroke-linecap=\"round\""));
    assert!(laml::export::png(&svg).is_ok());
    assert!(laml::export::pdf(&svg).is_ok());
}

#[test]
fn classifiers_and_sequence_parts_share_modern_radii_without_changing_plain() {
    let class_source = "settings {\ntype: class\n}\nsemantics {\nclass A {\n+name: String\n}\n}\n";
    let (svg, boxes, _) = image(class_source);
    assert_eq!(boxes[0].radius, 8.0);
    assert!(svg.contains("rx=\"8.00\""));
    let (_, plain_boxes, _) =
        image(&class_source.replace("type: class", "type: class\ntheme: plain"));
    assert_eq!(plain_boxes[0].radius, 0.0);

    let source = include_str!("../examples/showcase-sequence.laml");
    let modern = laml::sequence::parse(source).unwrap();
    let rendered = laml::sequence::render::render(&modern, TextMode::System).unwrap();
    let part = rendered
        .lines()
        .find(|line| line.contains("data-participant=\"Browser\""))
        .unwrap();
    assert!(part.contains("rx=\"8.00\""));
    let activation = rendered
        .lines()
        .find(|line| line.contains("data-activation-owner=\"API\""))
        .unwrap();
    assert!(activation.contains("rx=\"3.00\""));
    assert!(rendered.contains("data-sequence-note-accent="));
    assert!(rendered.contains("data-message-pill=\"true\""));
    assert!(rendered.lines().any(
        |line| line.contains("data-fragment-operator=\"alt\"") && line.contains("rx=\"7.00\"")
    ));

    let plain_source = source.replacen("type: sequence", "type: sequence\ntheme: plain", 1);
    let plain = laml::sequence::parse(&plain_source).unwrap();
    let rendered = laml::sequence::render::render(&plain, TextMode::System).unwrap();
    assert!(!rendered.contains("data-sequence-note-accent="));
    assert!(!rendered.contains("data-message-pill=\"true\""));
    assert!(
        rendered
            .lines()
            .any(|line| line.contains("data-participant=\"Browser\"")
                && line.contains("rx=\"0.00\""))
    );
}

#[test]
fn timing_frame_rounds_only_in_modern_theme() {
    let source = include_str!("../examples/timing-basic.laml");
    for (theme, radius) in [("modern", "8.00"), ("plain", "0.00")] {
        let source = source.replacen("type: timing", &format!("type: timing\ntheme: {theme}"), 1);
        let diagram = laml::timing::parse(&source).unwrap();
        let mut stylesheet = laml::styles::StyleSheet::default();
        stylesheet.theme = diagram.theme;
        let svg = laml::timing::render(&diagram, &stylesheet, TextMode::System).unwrap();
        assert!(svg.contains(&format!("rx=\"{radius}\" fill=\"none\"")));
    }
}

#[test]
fn plain_notes_keep_the_fold_and_modern_uml_notes_use_the_same_card() {
    for diagram_type in ["", "type: class\n", "type: object\n"] {
        let plain = format!(
            "settings {{\n{diagram_type}theme: plain\n}}\nsemantics {{\nnote Memo {{\nMessage\n}}\n}}\n"
        );
        let (svg, boxes, _) = image(&plain);
        assert!(!boxes[0].note_card);
        assert!(!svg.contains("data-note-card=\"true\""));
        assert!(svg.contains("markerWidth=\"12\" markerHeight=\"8\" refX=\"10\""));
        let modern = plain.replace("theme: plain", "theme: modern");
        let (svg, boxes, _) = image(&modern);
        assert!(boxes[0].note_card);
        assert!(svg.contains("data-note-card=\"true\""));
        assert!(svg.contains("data-note-accent=\"true\""));
    }
}
