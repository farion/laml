use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const EXAMPLE: &str = include_str!("../examples/component-basic.laml");

#[test]
fn component_overview_renders_and_retains_required_interface_box() {
    let diagram = parse(EXAMPLE).unwrap();
    let styles = load(&diagram, Path::new("examples/component-basic.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert!(boxes.iter().any(|node| node.label.contains("«component»")));
    assert!(boxes.iter().any(|node| node.id == "OrderAPI"));
    assert!(!boxes.iter().any(|node| node.id == "HealthAPI"));
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("«component»"));
    assert!(svg.contains("«interface»"));
    assert!(svg.contains("<circle "));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn component_relations_do_not_imply_assembly_or_fake_interface_semantics() {
    for (source, reason) in [
        ("Gateway -- Orders", "operator"),
        ("Gateway ..|> Orders", "component overview"),
        ("OrderAPI ..> Gateway", "component overview"),
        ("Gateway \"1\" ..> Orders", "component overview"),
        ("Gateway::HTTP ..> Orders", "unknown"),
    ] {
        let diagram = format!(
            "settings {{\ntype: component\n}}\nsemantics {{\ncomponent Gateway\ncomponent Orders\ninterface OrderAPI\n{source}\n}}\n"
        );
        let error = parse(&diagram).unwrap_err();
        assert!(error.message.contains(reason), "{source}: {error:?}");
    }
    let wrong_type = EXAMPLE.replace("type: component", "type: class");
    assert!(parse(&wrong_type).is_err());
}

#[test]
fn required_interface_can_use_a_compact_socket() {
    let source = "settings {\ntype: component\n}\nsemantics {\ncomponent Gateway\ninterface API\n@required Gateway ..> API\n}\nstyling {\nrelation[required] {\ninterface-notation: socket\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/component-basic.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert!(
        boxes
            .iter()
            .any(|node| node.kind == laml::syntax::NodeKind::Socket)
    );
    assert!(!boxes.iter().any(|node| node.id == "API"));
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("A10,10"));
    assert!(!svg.contains("«interface»"));
}

#[test]
fn component_ports_require_explicit_declared_interface_contracts() {
    let source = "settings {\ntype: component\n}\nsemantics {\ninterface API\ncomponent Gateway {\n@http port HTTP(requires=API)\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/component-basic.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let gateway = boxes.iter().find(|node| node.id == "Gateway").unwrap();
    assert!(gateway.ports[0].label.contains("requires API"));
    let svg = render(&boxes, &[]).unwrap();
    assert!(svg.contains("requires API"));
    for bad in [
        source.replace("requires=API", "provides=Missing"),
        source.replace("requires=API", "bogus=API"),
        source.replace("requires=API", ""),
        source.replace("requires=API", "provides=API; requires=API"),
    ] {
        assert!(parse(&bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn component_lollipop_rejects_box_still_used_as_required_interface() {
    let source = EXAMPLE.replace("Gateway ..> OrderAPI", "Gateway ..> HealthAPI");
    let diagram = parse(&source).unwrap();
    let styles = load(&diagram, Path::new("examples/component-basic.laml")).unwrap();
    let error = place(&diagram, &styles).unwrap_err();
    assert!(error.message.contains("still referenced"), "{error:?}");
}
