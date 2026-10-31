use laml::{geometry::place, parser::parse, route::route, styles::load};
use std::{fs, path::Path};

#[test]
fn dense_class_showcase_keeps_composition_title_and_both_end_labels() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/showcase-class.laml");
    let source = fs::read_to_string(&path).unwrap();
    let old = "@owns Domain.Order \"1\" *-- \"1..*\" Domain.Line";
    assert!(source.contains(old));
    let dense = source.replace(old, &format!("{old} : contains"));
    let diagram = parse(&dense).unwrap();
    let sheet = load(&diagram, &path).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    let index = diagram
        .relations
        .iter()
        .position(|relation| relation.name.as_deref() == Some("owns"))
        .unwrap();
    let owns = &routed[index];
    assert_eq!(owns.label.as_ref().unwrap().text, "contains");
    assert!(owns.end_labels.iter().any(|label| label.text == "1"));
    assert!(owns.end_labels.iter().any(|label| label.text == "1..*"));
}

#[test]
fn compact_class_association_can_put_end_text_on_either_side() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\n@r A \"1\" -- \"1..*\" B : contains\n}\nlayout {\nA < B : align=center; gap=115\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = laml::styles::StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routes[0].label.as_ref().unwrap().text, "contains");
    assert_eq!(routes[0].end_labels.len(), 2);
}

#[test]
fn mandatory_attachment_sides_can_detour_around_an_intervening_class() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass Blocker\nclass C\n@route A --> C\n}\nlayout {\nA < Blocker : align=center; gap=90\nBlocker < C : align=center; gap=90\nrelation[route].source : side=top\nrelation[route].target : side=left\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = laml::styles::StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    let points = &routed[0].points;
    assert!((points[0].y - boxes[0].y).abs() < 1e-6);
    assert!((points.last().unwrap().x - boxes[2].x).abs() < 1e-6);
    assert!(points.len() >= 4);
}
