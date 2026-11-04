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
fn architecture_dependencies_route_without_changing_original_node_layout() {
    assert!(
        !ARCHITECTURE.contains("layout {"),
        "the routing test must not prescribe geometry"
    );
    for theme in ["modern", "modern-dark"] {
        let source =
            ARCHITECTURE.replace("type: package", &format!("type: package\ntheme: {theme}"));
        let diagram = parse(&source).unwrap();
        let sheet = load(&diagram, Path::new("examples/package-architecture.laml")).unwrap();
        let boxes = place(&diagram, &sheet).unwrap();
        let routes = route(&diagram, &boxes, &sheet).unwrap();
        let node = |id: &str| boxes.iter().find(|box_node| box_node.id == id).unwrap();
        let api = node("Shop.API");
        let orders = node("Shop.Orders");
        let pricing = node("Shop.Pricing");
        let payments = node("Payments");
        let common = node("Common");
        assert!(
            orders.y >= api.y + api.height + 36.0 - 1e-6,
            "{theme}: keep breathing room below API"
        );
        assert!(
            common.y >= payments.y + payments.height + 36.0 - 1e-6,
            "{theme}: keep breathing room below Payments"
        );
        let import = &routes[1].points;
        let access = &routes[2].points;
        let merge = &routes[3].points;
        assert_eq!(import.len(), 2, "{theme}: import must be straight");
        assert!((import[0].x - api.x - api.width).abs() < 1e-6);
        assert!((import[1].x - payments.x).abs() < 1e-6);
        assert!((import[0].y - import[1].y).abs() < 1e-6);
        assert!(
            api.y + api.height / 2.0 - payments.y - payments.height / 2.0 >= 10.0,
            "{theme}: automatic placement should lift Payments clear of merge"
        );
        assert!(
            import[1].y >= payments.y + 14.0 + 6.0
                && import[1].y <= payments.y + payments.height - 6.0,
            "{theme}: the straight import must still dock on Payments' visible body"
        );
        assert_eq!(access.len(), 2, "{theme}: access must be straight");
        assert!((access[0].x - orders.x - orders.width).abs() < 1e-6);
        assert!((access[1].x - common.x).abs() < 1e-6);
        assert!((access[0].y - access[1].y).abs() < 1e-6);
        assert!(
            access[0].y - pricing.y - pricing.height >= 18.0,
            "{theme}: access needs visible clearance below Pricing"
        );
        assert_eq!(merge.len(), 3, "{theme}: merge needs only one bend");
        assert!(merge[1].y > payments.y + payments.height + 4.0);
        let distance_to_payments = merge[0].y - payments.y - payments.height;
        let distance_to_common = common.y + 14.0 - merge[0].y;
        assert!(
            distance_to_payments > 1.7 * distance_to_common,
            "{theme}: merge should visibly belong to Common, not Payments"
        );
        assert!((merge[0].x - node("Shop.Pricing").x - node("Shop.Pricing").width).abs() < 1e-6);
        assert!(
            merge[2].x >= common.x + 32.0 && merge[2].x <= common.x + common.width - 6.0,
            "{theme}: merge docks on Common's visible top edge"
        );
        assert!(
            (merge[2].y - common.y - 14.0).abs() < 1e-6,
            "{theme}: merge should meet Common's visible body, not the tab's bounding box"
        );
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        let touches_outline = |n: &laml::geometry::BoxNode, p: laml::route::Point| {
            let body_top = n.y + 14.0;
            (near(p.x, n.x) && p.y >= n.y && p.y <= n.y + n.height)
                || (near(p.x, n.x + n.width) && p.y >= body_top && p.y <= n.y + n.height)
                || (near(p.y, n.y) && p.x >= n.x && p.x <= n.x + 32.0)
                || (near(p.y, body_top) && p.x >= n.x + 32.0 && p.x <= n.x + n.width)
                || (near(p.y, n.y + n.height) && p.x >= n.x && p.x <= n.x + n.width)
                || (near(p.x, n.x + 32.0) && p.y >= n.y && p.y <= body_top)
        };
        for relation in &routes {
            assert!(
                touches_outline(&boxes[relation.from], relation.points[0]),
                "{theme}: source '{}' has a visible gap",
                boxes[relation.from].id
            );
            assert!(
                touches_outline(&boxes[relation.to], *relation.points.last().unwrap()),
                "{theme}: target '{}' has a visible gap",
                boxes[relation.to].id
            );
        }
        let label = routes[3].label.as_ref().unwrap();
        assert!(
            label.y > merge[0].y,
            "{theme}: merge label belongs below the route, near Common"
        );
        assert!(
            label.y > payments.y + payments.height + 20.0,
            "{theme}: merge label must clear the unrelated Payments box"
        );
        assert!(
            label.x + label.width + 4.0 < payments.x,
            "{theme}: the rendered merge label pill must not touch Payments"
        );
        assert!(
            routes.iter().all(|route| route.bridges.is_empty()),
            "{theme}: dependencies must not cross"
        );
    }
}

#[test]
fn quickstart_dependency_leaves_nested_package_horizontally() {
    let source = include_str!("../examples/package-quickstart.laml");
    for theme in ["modern", "modern-dark"] {
        let source = source.replace("type: package", &format!("type: package\ntheme: {theme}"));
        let diagram = parse(&source).unwrap();
        let sheet = load(&diagram, Path::new("examples/package-quickstart.laml")).unwrap();
        let boxes = place(&diagram, &sheet).unwrap();
        let routes = route(&diagram, &boxes, &sheet).unwrap();
        let points = &routes[0].points;
        assert_eq!(
            points.len(),
            2,
            "{theme}: dependency should run directly right"
        );
        assert!(points[1].x > points[0].x);
        assert!(
            (points[1].y - points[0].y).abs() < 1e-6,
            "{theme}: sloped dependency"
        );
    }
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
