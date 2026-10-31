use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const SOURCE: &str = "settings {\ntype: class\n}\nsemantics {\nclass Client\nclass Worker\ninterface API\n@client Client ..|> API\n@worker Worker ..|> API\n}\nstyling {\nrelation[client] {\ninterface-notation: lollipop\n}\nrelation[worker] {\ninterface-notation: lollipop\n}\n}\n";

#[test]
fn realization_lollipop_replaces_unused_interface_box_once_per_provider() {
    let diagram = parse(SOURCE).unwrap();
    let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!(boxes.len(), 4);
    assert!(!boxes.iter().any(|node| node.id == "API"));
    let routed = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routed.len(), 2);
    let svg = render(&boxes, &routed).unwrap();
    assert_eq!(svg.matches("<circle ").count(), 2);
    assert_eq!(svg.matches(">API</text>").count(), 2);
    assert!(!svg.contains("«interface»"));
    assert!(svg.contains("data-node-id=\"_laml_lollipop_0\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn lollipop_fails_when_interface_box_is_used_or_realization_is_not_plain() {
    let cases = [
        SOURCE.replace("interface API", "interface API {\n+run()\n}"),
        SOURCE.replace("@worker Worker ..|> API", "@worker Worker --> API"),
        SOURCE.replace(
            "@worker Worker ..|> API",
            "@worker Worker ..|> API : implements",
        ),
        SOURCE.replace(
            "interface API",
            "interface API\nclass Report\nAPI -- Report",
        ),
        SOURCE.replace("}\nstyling {", "}\nlayout {\nClient < API\n}\nstyling {"),
    ];
    for source in cases {
        let diagram = parse(&source).unwrap();
        let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
        assert!(place(&diagram, &styles).is_err());
    }
    let hidden_members = SOURCE
        .replace("interface API", "interface API {\n+run()\n}")
        .replace(
            "relation[client] {",
            "node[API] compartment[operations] {\ndisplay: none\n}\nrelation[client] {",
        );
    let diagram = parse(&hidden_members).unwrap();
    let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
    assert!(place(&diagram, &styles).is_err());
    let wrong_kind = SOURCE.replace("type: class", "type: object");
    assert!(parse(&wrong_kind).is_err());
    let bad_value = SOURCE.replace("lollipop", "socket");
    let diagram = parse(&bad_value).unwrap();
    assert!(load(&diagram, Path::new("examples/architecture.laml")).is_err());
}
