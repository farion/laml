use laml::syntax::NodeKind;
use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const ORDER: &str = include_str!("../examples/usecase-order.laml");
const QUICKSTART: &str = include_str!("../examples/usecase-quickstart.laml");

#[test]
fn quickstart_draws_straight_association_from_proportional_actor_to_ellipse() {
    let diagram = parse(QUICKSTART).unwrap();
    let mut sheet = load(&diagram, Path::new("examples/usecase-quickstart.laml")).unwrap();
    sheet.theme = laml::theme::Theme::ModernDark;
    let boxes = place(&diagram, &sheet).unwrap();
    let actor = boxes.iter().find(|node| node.id == "Customer").unwrap();
    let system = boxes.iter().find(|node| node.id == "Shop").unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(
        routes[0].points.len(),
        2,
        "simple associations must be straight"
    );
    let start = routes[0].points[0];
    assert!((start.x - actor.x - actor.width / 2.0 - 50.0).abs() < 0.01);
    assert!((start.y - actor.y - 47.5).abs() < 0.01);
    assert!(start.x > actor.x + actor.width / 2.0 + 26.0);
    // The route starts at the invisible contour, not at the actor's arm.
    assert!((routes[0].points[1].y - start.y).abs() < 0.01);
    assert_eq!(system.radius, 8.0);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"8.00\"",
        system.x, system.y, system.width, system.height
    )));
    assert!(svg.contains("r=\"14.00\""));
    let x = actor.x + actor.width / 2.0;
    assert!(svg.contains(&format!(
        "M{x:.2},{:.2} V{:.2} M{:.2},{:.2} H{:.2} M{x:.2},{:.2} L{:.2},{:.2} M{x:.2},{:.2} L{:.2},{:.2}",
        actor.y + 29.0, actor.y + 60.0,
        x - 26.0, actor.y + 37.0, x + 26.0,
        actor.y + 60.0, x - 24.0, actor.y + 91.0,
        actor.y + 60.0, x + 24.0, actor.y + 91.0,
    )));
    // Arms: 26 per side, legs: sqrt(31² + 24²) ≈ 39.2 each.
    let leg_length = 31.0_f64.hypot(24.0);
    assert!((26.0 / leg_length - 2.0 / 3.0).abs() < 0.01);
    let opening = 2.0 * (24.0_f64 / 31.0).atan().to_degrees();
    assert!((opening - 75.0).abs() < 1.0);
    // Neck: 8, abdomen: 23; the arms remain above the figure's middle.
    assert_eq!(37.0 - (15.0 + 14.0), 8.0);
    assert_eq!(60.0 - 37.0, 23.0);
    for (x, y) in [(14.0_f64, 1.0_f64), (26.0, 37.0), (24.0, 91.0)] {
        assert!((x / 50.0).powi(2) + ((y - 47.5) / 50.0).powi(2) < 1.0);
    }
}

fn source(body: &str) -> String {
    format!("settings {{\ntype: usecase\n}}\nsemantics {{\n{body}\n}}")
}

#[test]
fn usecase_system_has_scoped_ellipses_and_external_actors() {
    let diagram = parse(ORDER).unwrap();
    assert!(
        diagram
            .nodes
            .iter()
            .any(|node| node.id == "Shop.Order" && node.kind == NodeKind::UseCase)
    );
    assert!(
        diagram
            .nodes
            .iter()
            .any(|node| node.id == "ReturningCustomer" && node.kind == NodeKind::UseCaseActor)
    );
    let sheet = load(&diagram, Path::new("examples/usecase-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let shop = boxes.iter().find(|node| node.id == "Shop").unwrap();
    let customer = boxes.iter().find(|node| node.id == "Customer").unwrap();
    let order = boxes.iter().find(|node| node.id == "Shop.Order").unwrap();
    assert!(customer.x + customer.width < shop.x);
    assert!(order.x > shop.x && order.x + order.width < shop.x + shop.width);
    assert!(order.y > shop.y && order.y + order.height < shop.y + shop.height);
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    for text in ["«include»", "«extend»", "Place order", "ReturningCustomer"] {
        assert!(svg.contains(text));
    }
    assert!(svg.contains("<ellipse"));
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn actor_association_falls_back_to_a_single_diagonal_when_enabled() {
    let diagram =
        parse(&QUICKSTART.replace("type: usecase", "type: usecase\nallow-diagonal: true")).unwrap();
    let sheet = load(&diagram, Path::new("examples/usecase-quickstart.laml")).unwrap();
    let mut boxes = place(&diagram, &sheet).unwrap();
    let actor_y = boxes.iter().find(|node| node.id == "Customer").unwrap().y;
    let order = boxes
        .iter_mut()
        .find(|node| node.id == "Shop.Order")
        .unwrap();
    order.y = actor_y + 110.0; // hand lies above the ellipse's vertical extent
    let shop = boxes.iter_mut().find(|node| node.id == "Shop").unwrap();
    shop.y = actor_y + 30.0;
    shop.height = 190.0;
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routes[0].points.len(), 2);
    assert!((routes[0].points[0].y - routes[0].points[1].y).abs() > 1.0);
}

#[test]
fn order_actors_align_and_lateral_association_avoids_the_actor_head() {
    let diagram = parse(ORDER).unwrap();
    let sheet = load(&diagram, Path::new("examples/usecase-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let actor = boxes.iter().find(|n| n.id == "Customer").unwrap();
    let returning = boxes.iter().find(|n| n.id == "ReturningCustomer").unwrap();
    let shop = boxes.iter().find(|n| n.id == "Shop").unwrap();
    let order = boxes.iter().find(|n| n.id == "Shop.Order").unwrap();
    assert!(
        (actor.y - returning.y).abs() < 0.01,
        "align the figures, not their captions"
    );
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let generalization = &routes[diagram
        .relations
        .iter()
        .position(|r| r.from == "ReturningCustomer")
        .unwrap()];
    assert_eq!(generalization.points.len(), 2);
    assert!((generalization.points[0].y - generalization.points[1].y).abs() < 0.01);
    let association = &routes[diagram
        .relations
        .iter()
        .position(|r| r.from == "Customer")
        .unwrap()];
    assert_eq!(association.points.len(), 5);
    let p = &association.points;
    assert!((p[0].y - actor.y - 47.5).abs() < 0.01);
    assert!((p[0].y - p[1].y).abs() < 0.01);
    assert!(
        p[1].x < shop.x - 8.0,
        "the first elbow must clear the subject frame"
    );
    assert!(p[2].y > order.y + order.height + 8.0);
    assert!((p[4].y - order.y - order.height).abs() < 0.01);
}

#[test]
fn usecase_subject_corner_radius_follows_theme_and_can_be_overridden() {
    let diagram = parse(QUICKSTART).unwrap();
    let mut sheet = load(&diagram, Path::new("examples/usecase-quickstart.laml")).unwrap();
    sheet.theme = laml::theme::Theme::Plain;
    let boxes = place(&diagram, &sheet).unwrap();
    let shop = boxes.iter().find(|node| node.id == "Shop").unwrap();
    assert_eq!(shop.radius, 0.0);

    let text = format!("{QUICKSTART}\nstyling {{\nnode[Shop] {{\ncorner-radius: 12\n}}\n}}");
    let diagram = parse(&text).unwrap();
    let sheet = load(&diagram, Path::new("examples/usecase-quickstart.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert_eq!(
        boxes.iter().find(|node| node.id == "Shop").unwrap().radius,
        12.0
    );
}

#[test]
fn usecase_relations_reject_false_uml_meanings() {
    let prefix =
        "actor Customer\nactor Guest\nsystem Shop {\nusecase Order\nusecase Payment\nRELATION\n}\n";
    for (line, reason) in [
        ("Order -- Payment", "use-case relation"),
        ("Order ..> Payment", "use-case relation"),
        ("Order ..> Payment <<approve>>", "use-case relation"),
        ("Order --|> Customer", "use-case relation"),
        ("Order --|> Payment <<include>>", "use-case relation"),
        ("Order ..> Order <<include>>", "use-case relation"),
        ("Customer ..> Payment <<include>>", "use-case relation"),
        ("Customer -- Guest", "use-case relation"),
        (
            "Order --|> Payment : misleading",
            "use-case relations require plain",
        ),
    ] {
        let text = source(&prefix.replace("RELATION", line));
        let err = parse(&text).unwrap_err();
        assert!(err.message.contains(reason), "{line}: {err:?}");
    }
}

#[test]
fn usecase_containment_and_generalization_cycles_are_checked() {
    for body in [
        "actor A\nusecase Outside\nsystem Shop {\nusecase Inside\n}",
        "system Shop {\nactor Wrong\nusecase Inside\n}",
        "system Shop {\nusecase One\n}\nsystem Other {\nusecase Two\n}",
        "actor A\nsystem Shop {\nusecase One\n}\nA --|> A",
        "actor A\nactor B\nsystem Shop {\nusecase One\n}\nA --|> B\nB --|> A",
        "system Shop {\nusecase A\nusecase B\nA --|> B\nB --|> A\n}",
        "system Shop {\nactor Child\nusecase One\n}",
    ] {
        assert!(
            parse(&source(body)).is_err(),
            "unexpectedly accepted: {body}"
        );
    }
}

#[test]
fn actor_connection_cannot_request_a_nonexistent_top_or_bottom_hand() {
    let text =
        source("actor Customer\nsystem Shop {\nusecase Order\n}\n@link Customer -- Shop.Order");
    let text = format!("{text}\nlayout {{\nrelation[link].source : side=top\n}}");
    assert!(
        parse(&text)
            .unwrap_err()
            .message
            .contains("only left or right")
    );
}

#[test]
fn ellipse_and_actor_cannot_be_restyled_to_false_uml_forms() {
    for (selector, property, reason) in [
        ("node[Shop.Order]", "shape: rectangle", "UML node form"),
        ("node[Shop]", "shape: ellipse", "UML node form"),
        ("node[Customer]", "shape: diamond", "UML node form"),
        ("node[Customer]", "fill: none", "no fillable box"),
        ("node[Customer]", "border-color: none", "visible outline"),
        (
            "node[Customer]",
            "corner-radius: 20",
            "custom corner radius",
        ),
    ] {
        let text = format!("{ORDER}\nstyling {{\n{selector} {{\n{property}\n}}\n}}");
        let diagram = parse(&text).unwrap();
        let sheet = load(&diagram, Path::new("examples/usecase-order.laml")).unwrap();
        assert!(
            place(&diagram, &sheet)
                .unwrap_err()
                .message
                .contains(reason)
        );
    }
}
