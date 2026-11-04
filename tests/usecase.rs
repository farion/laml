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
    assert!((start.x - actor.x - actor.width / 2.0 - 22.0).abs() < 0.01);
    assert!((start.y - actor.y - 64.0).abs() < 0.01);
    assert!((routes[0].points[1].y - start.y).abs() < 0.01);
    assert_eq!(system.radius, 8.0);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"8.00\"",
        system.x, system.y, system.width, system.height
    )));
    assert!(svg.contains("r=\"11\""));
    // Arms: 22 per side, legs: sqrt(32² + 8.5²) ≈ 33.1 each.
    let leg_length = 32.0_f64.hypot(8.5);
    assert!((22.0 / leg_length - 2.0 / 3.0).abs() < 0.01);
    assert!(((32.0_f64 / 8.5).atan().to_degrees() - 75.0).abs() < 1.0);
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
fn actor_association_falls_back_to_a_single_diagonal_when_horizontal_is_impossible() {
    let diagram = parse(QUICKSTART).unwrap();
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
