use laml::syntax::NodeKind;
use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const ORDER: &str = include_str!("../examples/usecase-order.laml");
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
fn ellipse_and_system_boundary_cannot_be_restyled_to_false_uml_forms() {
    for (selector, property, reason) in [
        ("node[Shop.Order]", "shape: rectangle", "UML node form"),
        ("node[Shop]", "shape: ellipse", "UML node form"),
        ("node[Customer]", "shape: diamond", "UML node form"),
        ("node[Customer]", "fill: none", "no fillable box"),
        ("node[Customer]", "border-color: none", "visible outline"),
        ("node[Shop]", "corner-radius: 20", "custom corner radius"),
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
