use laml::{
    geometry::{self, BoxNode},
    parser, styles,
};
use std::path::Path;

fn source(layout: &str) -> String {
    format!(
        "semantics {{\n A\n B\n C\n}}\nlayout {{\n{layout}\n}}\nstyling {{\n node[A] {{\n width: 200\n height: 140\n }}\n node[B] {{\n width: 100\n height: 80\n }}\n}}\n"
    )
}
fn build(source: &str) -> Result<Vec<BoxNode>, String> {
    let diagram = parser::parse(source).map_err(|e| format!("{}:{}", e.line, e.message))?;
    let sheet = styles::load(&diagram, Path::new("alignment.laml")).map_err(|e| e.message)?;
    geometry::place(&diagram, &sheet).map_err(|e| format!("{}:{}", e.line, e.message))
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}
fn overlaps(a: &BoxNode, b: &BoxNode) -> bool {
    a.x < b.x + b.width - 1e-6
        && b.x < a.x + a.width - 1e-6
        && a.y < b.y + b.height - 1e-6
        && b.y < a.y + a.height - 1e-6
}
#[test]
fn all_seven_independent_alignments_use_measured_boxes() {
    for name in [
        "left", "right", "top", "bottom", "center-x", "center-y", "center",
    ] {
        let text = source(&format!(" A ? B : align={name}"));
        let diagram = parser::parse(&text).unwrap();
        assert!(diagram.layout.is_empty());
        assert_eq!(diagram.alignments.len(), 1);
        let boxes = build(&text).unwrap();
        let (a, b, c) = (&boxes[0], &boxes[1], &boxes[2]);
        let (x, y) = diagram.alignments[0].align.anchors();
        if let Some(k) = x {
            near(a.x + k * a.width, b.x + k * b.width);
        }
        if let Some(k) = y {
            near(a.y + k * a.height, b.y + k * b.height);
        }
        assert_eq!(overlaps(a, b), name == "center");
        assert!(!overlaps(a, c));
        assert!(!overlaps(b, c));
    }
}
#[test]
fn shared_center_is_transitive_but_does_not_grant_unrelated_overlap() {
    let boxes = build(&source(" A ? B : align=center\n B ? C : align=center")).unwrap();
    for b in &boxes[1..] {
        near(boxes[0].x + boxes[0].width / 2.0, b.x + b.width / 2.0);
        near(boxes[0].y + boxes[0].height / 2.0, b.y + b.height / 2.0);
    }
    let boxes = build(&source(" A ? B : align=center-x\n A ? B : align=center-y")).unwrap();
    assert!(overlaps(&boxes[0], &boxes[1]));
    assert!(!overlaps(&boxes[0], &boxes[2]));
}
#[test]
fn independent_constraints_conflict_instead_of_being_discarded() {
    assert!(build(&source(" A ? B : align=center\n A < B")).is_err());
    assert!(build(&source(" A ? B : align=left\n A ? B : align=right")).is_err());
    // Coincident edges are not a blanket permission to overlap: only shared centers are.
    assert!(build(&source(" A ? B : align=left\n A ? B : align=top")).is_err());
}
#[test]
fn legacy_center_is_still_perpendicular_to_placement() {
    let boxes = build(&source(" A < B : align=center")).unwrap();
    near(
        boxes[0].y + boxes[0].height / 2.0,
        boxes[1].y + boxes[1].height / 2.0,
    );
    assert!(boxes[1].x > boxes[0].x + boxes[0].width);
    let boxes = build(&source(" A above B : align=center")).unwrap();
    near(
        boxes[0].x + boxes[0].width / 2.0,
        boxes[1].x + boxes[1].width / 2.0,
    );
    assert!(boxes[1].y > boxes[0].y + boxes[0].height);
}
#[test]
fn invalid_references_attributes_and_hidden_targets_report_errors() {
    for rule in [
        " A ? Missing : align=left",
        " A ? A : align=center",
        " A ? B : gap=20",
        " A ? B : align=left; gap=10",
        " A ? B : align=middle",
        " A ? B",
    ] {
        assert!(parser::parse(&source(rule)).is_err(), "{rule}");
    }
    let text = "semantics {\n A\n B\n}\nlayout {\n A ? B : align=left\n}\nstyling {\n node[B] {\n display: none\n }\n}\n";
    assert!(build(text).unwrap_err().contains("hidden"));
}
#[test]
fn scoped_alignment_is_applied_within_and_across_containers() {
    let text = "semantics {\n package P {\n A\n B\n }\n package Q {\n C\n }\n}\nlayout {\n P.A ? P.B : align=left\n P.A ? Q.C : align=top\n}\n";
    let boxes = build(text).unwrap();
    let node = |id| boxes.iter().find(|node| node.id == id).unwrap();
    near(node("P.A").x, node("P.B").x);
    near(node("P.A").y, node("Q.C").y);
    let centered = text.replace("P.A ? Q.C : align=top", "P.A ? Q.C : align=center");
    assert!(
        parser::parse(&centered)
            .unwrap_err()
            .message
            .contains("same container")
    );
}
