use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const SOURCE: &str = include_str!("../examples/component-internal.laml");

#[test]
fn component_parts_assemble_and_delegate_only_matching_ports() {
    let diagram = parse(SOURCE).unwrap();
    let sheet = load(&diagram, Path::new("examples/component-internal.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let owner = boxes.iter().find(|node| node.id == "Storefront").unwrap();
    assert!(owner.is_container);
    let browser = boxes
        .iter()
        .find(|node| node.id == "Storefront.browser")
        .unwrap();
    let orders = boxes
        .iter()
        .find(|node| node.id == "Storefront.orders")
        .unwrap();
    assert!(browser.x >= owner.x && orders.x + orders.width <= owner.x + owner.width);
    assert!(browser.label.contains("browser:Browser"));
    assert!(
        browser
            .ports
            .iter()
            .any(|port| port.id == "Storefront.browser::Out")
    );
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routed.len(), 2);
    assert!(routed.iter().all(|line| !line.appearance.dashed));
    let svg = render(&boxes, &routed).unwrap();
    assert!(svg.contains("data-port-id=\"Storefront.browser::Out\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn component_connector_rejects_mismatch_cross_owner_and_fake_assembly() {
    for (source, reason) in [
        (
            SOURCE.replace("browser::Out == orders::In", "browser == orders"),
            "matching",
        ),
        (
            SOURCE.replace("browser::Out == orders::In", "browser::In == browser::Out"),
            "matching",
        ),
        (
            SOURCE.replace(
                "browser::Out == orders::In",
                "Storefront::Public == orders::In",
            ),
            "matching",
        ),
        (
            SOURCE.replace(
                "Storefront::Public => browser::In",
                "Storefront::Public => orders::In",
            ),
            "matching",
        ),
        (
            SOURCE.replace("part orders(of=Orders)", "part orders(of=Missing)"),
            "declared component type",
        ),
        (
            SOURCE.replace(
                "browser::Out == orders::In",
                "browser::Out == orders::In : duplicate",
            ),
            "cannot have labels",
        ),
    ] {
        let error = parse(&source).unwrap_err();
        assert!(error.message.contains(reason), "{error:?}");
    }
    let malformed = "settings {\ntype: component\n}\nsemantics {\npart orphan(of=Missing)\n}\n";
    assert!(parse(malformed).is_err());
}

#[test]
fn scoped_component_type_and_interface_references_resolve_on_cloned_ports() {
    let source = "settings {\ntype: component\n}\nsemantics {\npackage Catalog {\ninterface API\ncomponent Worker {\nport In(provides=API)\n}\ncomponent Composite {\npart worker(of=Worker)\n}\n}\n}\n";
    let diagram = parse(source).unwrap();
    let worker = diagram
        .nodes
        .iter()
        .find(|node| node.id == "Catalog.Composite.worker")
        .unwrap();
    assert_eq!(worker.part_type.as_deref(), Some("Catalog.Worker"));
    assert!(matches!(&worker.ports[0].contract,
        Some(laml::syntax::PortContract::Provides(id)) if id == "Catalog.API"));
    let sheet = load(&diagram, Path::new("examples/component-internal.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert!(
        boxes
            .iter()
            .find(|node| node.id == "Catalog.Composite")
            .unwrap()
            .is_container
    );
}

#[test]
fn part_port_keeps_its_type_scope_even_if_owner_has_a_shadowing_interface() {
    let source = "settings {\ntype: component\n}\nsemantics {\npackage Provider {\ninterface API\ncomponent Worker {\nport In(provides=API)\n}\n}\npackage Consumer {\ninterface API\ncomponent System {\npart worker(of=Provider.Worker)\n}\n}\n}\n";
    let diagram = parse(source).unwrap();
    let worker = diagram
        .nodes
        .iter()
        .find(|node| node.id == "Consumer.System.worker")
        .unwrap();
    assert!(matches!(&worker.ports[0].contract,
        Some(laml::syntax::PortContract::Provides(id)) if id == "Provider.API"));
}

#[test]
fn component_connector_never_uses_generic_graph_operator() {
    let source = SOURCE.replace("type: component", "type: generic");
    assert!(parse(&source).is_err());
    let source = SOURCE.replace("browser::Out == orders::In", "browser::Out -- orders::In");
    assert!(parse(&source).is_err());
}
