use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const SOURCE: &str = include_str!("../examples/deployment-basic.laml");

fn image(source: &str) -> String {
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, Path::new("examples/deployment-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    render(&boxes, &routes).unwrap()
}

#[test]
fn nested_deployment_artifacts_and_named_relations_export() {
    let diagram = parse(SOURCE).unwrap();
    let sheet = load(&diagram, Path::new("examples/deployment-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let edge = boxes.iter().find(|node| node.id == "Edge").unwrap();
    let runtime = boxes.iter().find(|node| node.id == "Edge.Runtime").unwrap();
    let nested = boxes
        .iter()
        .find(|node| node.id == "Edge.Runtime.WorkerJar")
        .unwrap();
    assert!(edge.is_container && runtime.is_container);
    assert!(nested.x > runtime.x && nested.x + nested.width < runtime.x + runtime.width);
    let svg = image(SOURCE);
    assert_eq!(svg.matches("data-deployment-cube=").count(), 3);
    assert!(svg.contains("«deploy»"));
    assert!(svg.contains("«manifest»"));
    assert!(svg.contains("«artifact»"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn node_kinds_and_dotted_note_attachments_use_distinct_notation() {
    let source = "settings {\ntype: deployment\n}\nsemantics {\nnode Cluster\ndevice Server\nexecutionEnvironment JVM\nnote Hint \"This is a note\"\nHint .. Cluster\n}\n";
    let svg = image(source);
    assert_eq!(svg.matches("data-deployment-cube=").count(), 3);
    assert!(svg.contains("«device»"));
    assert!(svg.contains("«executionEnvironment»"));
    assert!(svg.contains("This is a note"));
}

#[test]
fn deploy_and_manifest_reject_swapped_or_faked_meanings() {
    let base = "settings {\ntype: deployment\n}\nsemantics {\ndevice Server\nartifact App\ncomponent API\nREPLACE\n}\n";
    for (relation, expected) in [
        ("App ..> Server", "deployment requires"),
        ("Server ..> App <<deploy>>", "deployment requires"),
        ("App ..> API <<deploy>>", "deployment requires"),
        ("App ..> Server <<manifest>>", "deployment requires"),
        ("App -- Server", "deployment requires"),
        ("Server -- App", "deployment requires"),
        ("Server -- API", "deployment requires"),
        (
            "App ..> Server <<deploy>> <<manifest>>",
            "deployment requires",
        ),
        ("App ..|> API", "operator"),
    ] {
        let source = base.replace("REPLACE", relation);
        let error = parse(&source).unwrap_err();
        assert!(error.message.contains(expected), "{relation}: {error:?}");
    }
}

#[test]
fn nesting_rejects_ownership_that_would_claim_a_different_uml_model() {
    let base = "settings {\ntype: deployment\n}\nsemantics {\nREPLACE\n}\n";
    for (fragment, expected) in [
        ("device Server {\ncomponent API\n}", "node may contain"),
        (
            "device Server {\npackage P {\nartifact App\n}\n}",
            "node may contain",
        ),
        ("artifact App {\ndevice Server\n}", "no nested deployment"),
        ("component API {\nartifact App\n}", "no nested deployment"),
        ("device Server {\nclass API\n}", "not supported"),
        ("Unspecified", "not supported"),
        ("device Server <<artifact>>", "reserved UML"),
        ("+device Server", "classifier visibility"),
    ] {
        let error = parse(&base.replace("REPLACE", fragment)).unwrap_err();
        assert!(error.message.contains(expected), "{fragment}: {error:?}");
    }
}

#[test]
fn package_scopes_resolve_nested_artifacts_without_changing_relation_meaning() {
    let source = "settings {\ntype: deployment\n}\nsemantics {\npackage Production {\ncomponent API\ndevice Server {\nartifact App\n}\n@builds Server.App ..> API <<manifest>>\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.relations[0].from, "Production.Server.App");
    assert_eq!(diagram.relations[0].to, "Production.API");
    let styled =
        format!("{source}\nstyling {{\nnode[Production.Server] {{\nshape: ellipse\n}}\n}}\n");
    let diagram = parse(&styled).unwrap();
    let sheet = load(&diagram, Path::new("examples/deployment-basic.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("UML node form")
    );
}

#[test]
fn fixed_cube_cannot_clip_its_nested_execution_environment() {
    let source = format!("{SOURCE}\nstyling {{\nnode[Edge] {{\nwidth: 140\nheight: 110\n}}\n}}\n");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, Path::new("examples/deployment-basic.laml")).unwrap();
    let failure = place(&diagram, &sheet).unwrap_err();
    assert!(failure.message.contains("fixed container"), "{failure:?}");
}
