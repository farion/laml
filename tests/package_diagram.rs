use laml::syntax::{DiagramType, NodeKind};
use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const ARCHITECTURE: &str = include_str!("../examples/package-architecture.laml");
fn source(body: &str) -> String {
    format!("settings {{\ntype: package\n}}\nsemantics {{\n{body}\n}}")
}

#[test]
fn package_diagram_has_nested_groups_and_distinct_dependency_keywords() {
    let diagram = parse(ARCHITECTURE).unwrap();
    assert_eq!(diagram.settings.diagram_type, DiagramType::Package);
    assert!(
        diagram
            .nodes
            .iter()
            .any(|n| n.id == "Shop.API" && n.kind == NodeKind::Package)
    );
    assert!(
        diagram
            .relations
            .iter()
            .any(|r| r.stereotypes == ["import"])
    );
    assert!(
        diagram
            .relations
            .iter()
            .any(|r| r.stereotypes == ["access"])
    );
    assert!(diagram.relations.iter().any(|r| r.stereotypes == ["merge"]));
    assert!(diagram.relations.iter().any(|r| r.stereotypes.is_empty()));
    let sheet = load(&diagram, Path::new("examples/package-architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let root = boxes.iter().find(|n| n.id == "Shop").unwrap();
    let leaf = boxes.iter().find(|n| n.id == "Shop.API").unwrap();
    assert!(root.is_container && !leaf.is_container);
    assert!(leaf.x >= root.x && leaf.x + leaf.width <= root.x + root.width);
    let paths = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &paths).unwrap();
    for keyword in ["«import»", "«access»", "«merge»"] {
        assert!(svg.contains(keyword));
    }
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn unmarked_dependency_is_not_a_package_import() {
    let diagram = parse(&source("package A\npackage B\nA ..> B")).unwrap();
    assert_eq!(diagram.relations.len(), 1);
    assert!(diagram.relations[0].stereotypes.is_empty());
    assert_eq!(diagram.relations[0].display_label(), None);
}

#[test]
fn unsupported_package_relationships_and_elements_fail() {
    for (fragment, reason) in [
        ("package A\npackage B\nA --> B", "operator"),
        ("package A\npackage B\nA -- B", "operator"),
        (
            "package A\npackage B\nA ..> B <<deploy>>",
            "package relationship",
        ),
        (
            "package A\npackage B\nA ..> B <<include>>",
            "package relationship",
        ),
        (
            "package A\npackage B\nA ..> B <<import>> : extra",
            "package relationship",
        ),
        ("package A\nA ..> A <<merge>>", "package relationship"),
        (
            "package A {\npackage B\n}\nA ..> A.B <<import>>",
            "own contained package",
        ),
        ("class A", "not supported"),
        ("+package A", "only nested packages"),
        ("package A <<custom>>", "unchecked stereotypes"),
    ] {
        let err = parse(&source(fragment)).unwrap_err();
        assert!(err.message.contains(reason), "{fragment}: {err:?}");
    }
}

#[test]
fn package_merge_cycles_and_excluded_shapes_fail() {
    let cyclic = source(
        "package A\npackage B\npackage C\nA ..> B <<merge>>\nB ..> C <<merge>>\nC ..> A <<merge>>",
    );
    assert!(parse(&cyclic).unwrap_err().message.contains("merge cycle"));
    for (selector, property, reason) in [
        ("node[Shop.API]", "shape: ellipse", "UML node form"),
        ("node[Shop.API]", "height: 50", "fixed leaf package"),
        ("node[Shop]", "width: 100", "fixed container"),
    ] {
        let source = format!("{ARCHITECTURE}\nstyling {{\n{selector} {{\n{property}\n}}\n}}");
        let diagram = parse(&source).unwrap();
        let sheet = load(&diagram, Path::new("examples/package-architecture.laml")).unwrap();
        let err = place(&diagram, &sheet).unwrap_err();
        assert!(err.message.contains(reason), "{err:?}");
    }
}

#[test]
fn nested_public_private_visibility_is_displayed_not_executed() {
    let diagram = parse(&source(
        "package Root {\n+package Public\n-package Private\n}",
    ))
    .unwrap();
    let sheet = load(&diagram, Path::new("examples/package-architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let svg = render(&boxes, &[]).unwrap();
    assert!(svg.contains("+Public"));
    assert!(svg.contains("-Private"));
}

#[test]
fn notes_are_annotations_not_implicit_package_dependencies() {
    let diagram = parse(&source(
        "package A\npackage B\nnote Hint \"not an import\"\nHint .. A",
    ))
    .unwrap();
    assert_eq!(diagram.relations[0].display_label(), None);
    assert!(parse(&source("package A\npackage B\nA .. B")).is_err());
}
