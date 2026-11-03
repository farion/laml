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
fn plain_and_uml_notes_keep_the_fold() {
    for source in [
        "settings {\ntheme: plain\n}\nsemantics {\nnote Memo {\nMessage\n}\n}\n",
        "settings {\ntype: class\ntheme: modern\n}\nsemantics {\nnote Memo {\nMessage\n}\n}\n",
    ] {
        let (svg, boxes, _) = image(source);
        assert!(!boxes[0].note_card);
        if source.contains("theme: plain") {
            assert!(svg.contains("markerWidth=\"12\" markerHeight=\"8\" refX=\"10\""));
        }
        assert!(!svg.contains("data-note-card=\"true\""));
        assert!(!svg.contains("data-note-accent=\"true\""));
    }
}
