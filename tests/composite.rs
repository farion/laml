use laml::{
    export,
    geometry::place,
    layout::validate_ordering,
    parser::parse,
    route::{Point, route},
    styles::load,
    svg::{TextMode, render_with_mode},
    syntax::{DiagramType, NodeKind},
};
use std::path::Path;
const EXAMPLE: &str = include_str!("../examples/composite-checkout.laml");
fn graph(source: &str) -> laml::syntax::Diagram {
    parse(source).unwrap()
}
fn shell(body: &str) -> String {
    format!("settings {{\ntype: composite\n}}\nsemantics {{\n{body}\n}}")
}

#[test]
fn internal_structure_distinguishes_parts_roles_typed_ports_and_connectors() {
    let doc = graph(EXAMPLE);
    assert_eq!(doc.settings.diagram_type, DiagramType::Composite);
    assert_eq!(
        doc.nodes
            .iter()
            .filter(|n| n.kind == NodeKind::CompositeStructure)
            .count(),
        1
    );
    let part = doc.nodes.iter().find(|n| n.id == "Checkout.store").unwrap();
    let role = doc
        .nodes
        .iter()
        .find(|n| n.id == "Checkout.gateway")
        .unwrap();
    assert_eq!(part.kind, NodeKind::CompositePart);
    assert_eq!(role.kind, NodeKind::CompositeRole);
    assert_eq!(part.ports.len(), 1);
    assert_eq!(role.ports.len(), 2);
    assert_eq!(doc.relations.len(), 2);
    let path = Path::new("examples/composite-checkout.laml");
    let sheet = load(&doc, path).unwrap();
    validate_ordering(&doc).unwrap();
    let boxes = place(&doc, &sheet).unwrap();
    let routes = route(&doc, &boxes, &sheet).unwrap();
    let svg = render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap();
    assert!(svg.contains("internal structure"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains("gateway:Gateway"));
    assert!(svg.contains("store:Store"));
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
    let paths = render_with_mode(&boxes, &routes, TextMode::Paths).unwrap();
    assert!(!paths.contains("<text "));
}

#[test]
fn ownership_and_type_definitions_are_explicit() {
    for (source, part) in [
        (
            EXAMPLE.replace("structure Checkout {", "structure Checkout\n"),
            "root structure",
        ),
        (
            EXAMPLE.replace(
                "structure Checkout {",
                "structure Checkout {\n        role orphan(of=Missing)",
            ),
            "declared class type",
        ),
        (
            EXAMPLE.replace("role gateway(of=Gateway)", "role gateway(of=OrderAPI)"),
            "declared class type",
        ),
        (
            EXAMPLE.replace("role gateway(of=Gateway)", "role gateway"),
            "of=Component",
        ),
        (
            EXAMPLE.replace(
                "structure Checkout {",
                "structure Checkout {\n        text \"not a port\"",
            ),
            "body accepts only",
        ),
        (
            EXAMPLE.replace(
                "    structure Checkout {",
                "    structure Checkout {\n        part untyped",
            ),
            "of=Component",
        ),
        (
            EXAMPLE.replace(
                "    structure Checkout {",
                "    structure Checkout {\n        class Nested",
            ),
            "body accepts only",
        ),
    ] {
        let e = parse(&source).unwrap_err();
        assert!(e.message.contains(part), "{part}: {e:?}");
    }
    for (body, part) in [
        ("interface I\nclass A\npart p(of=A)", "root structure"),
        (
            "interface I\nclass A\nstructure S {\npart p(of=A)\n}\nstructure T {\npart q(of=A)\n}",
            "exactly one",
        ),
    ] {
        let e = parse(&shell(body)).unwrap_err();
        assert!(e.message.contains(part), "{part}: {e:?}");
    }
}

#[test]
fn connectors_are_not_generic_dependencies_or_associations() {
    for (source, part) in [
        (
            EXAMPLE.replace(
                "@internal gateway::Backend == store::In",
                "gateway::Backend -- store::In",
            ),
            "body accepts only",
        ),
        (
            EXAMPLE.replace(
                "@internal gateway::Backend == store::In",
                "Gateway::Backend == Store::In",
            ),
            "assembly",
        ),
        (
            EXAMPLE.replace(
                "@internal gateway::Backend == store::In",
                "gateway::Backend == gateway::Public",
            ),
            "assembly",
        ),
        (
            EXAMPLE.replace(
                "@external Checkout::Front => gateway::Public",
                "gateway::Public => Checkout::Front",
            ),
            "delegation",
        ),
        (
            EXAMPLE.replace(
                "@internal gateway::Backend == store::In",
                "gateway::Backend == store::In : link",
            ),
            "no association",
        ),
        (
            EXAMPLE.replace(
                "@internal gateway::Backend == store::In",
                "gateway::Backend == store::In <<assembly>>",
            ),
            "no association",
        ),
        (
            EXAMPLE.replace("port In(provides=OrderAPI)", "port In(requires=OrderAPI)"),
            "assembly",
        ),
    ] {
        let e = parse(&source).unwrap_err();
        assert!(e.message.contains(part), "{part}: {e:?}");
    }
}

#[test]
fn hiding_structural_roles_ports_or_connectors_fails() {
    for (styling, part) in [
        (
            "node[Checkout.gateway] {\ndisplay: none\n}",
            "must remain visible",
        ),
        (
            "relation[internal] {\ndisplay: none\n}",
            "connectors must remain visible",
        ),
        (
            "port[Checkout.gateway::Backend] {\ndisplay: none\n}",
            "ports must remain visible",
        ),
        (
            "node[Checkout.gateway] {\nborder-color: none\n}",
            "visible outline",
        ),
        (
            "port[Checkout.gateway::Backend] {\nborder-color: none\n}",
            "port outline and label",
        ),
    ] {
        let source = format!("{EXAMPLE}\nstyling {{\n{styling}\n}}");
        let doc = graph(&source);
        let sheet = load(&doc, Path::new("examples/composite-checkout.laml")).unwrap();
        let e = place(&doc, &sheet).unwrap_err();
        assert!(e.message.contains(part), "{styling}: {e:?}");
    }
}

#[test]
fn connector_stroke_cannot_be_styled_away() {
    for rule in ["color: none", "width: 0"] {
        let source = format!("{EXAMPLE}\nstyling {{\nrelation[internal] {{\n{rule}\n}}\n}}");
        let doc = graph(&source);
        let sheet = load(&doc, Path::new("examples/composite-checkout.laml")).unwrap();
        let boxes = place(&doc, &sheet).unwrap();
        let err = route(&doc, &boxes, &sheet).unwrap_err();
        assert!(
            err.message.contains("stroke must remain visible"),
            "{err:?}"
        );
    }
}

#[test]
fn compartment_label_occupies_real_checked_geometry() {
    let doc = graph(EXAMPLE);
    let sheet = load(&doc, Path::new("examples/composite-checkout.laml")).unwrap();
    let boxes = place(&doc, &sheet).unwrap();
    let mut routes = route(&doc, &boxes, &sheet).unwrap();
    let owner = boxes.iter().find(|b| b.id == "Checkout").unwrap();
    let y = owner.y
        + (owner.port_start + owner.ports.len() as f64 * owner.font_size * 1.25 + owner.padding)
            .max(50.0)
        + 15.0;
    routes[0].points = vec![
        Point {
            x: owner.x + owner.padding + 26.0,
            y,
        },
        Point {
            x: owner.x + owner.padding + 70.0,
            y,
        },
    ];
    let err = render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap_err();
    assert!(err.contains("crosses internal structure label"));
}

fn first_guide_example(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split("```text\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn guide_example_is_extracted_on_lf_and_crlf_checkouts() {
    let sample = "```text\nsemantics {\nA\n}\n```\n";
    assert_eq!(
        first_guide_example(sample),
        first_guide_example(&sample.replace('\n', "\r\n"))
    );
}

#[test]
fn guide_example_matches_compiler_and_preserves_pdf_vector_text() {
    let text = std::fs::read_to_string("docs/composite-structure-diagrams.md").unwrap();
    let source = first_guide_example(&text);
    let doc = graph(&source);
    let sheet = load(&doc, Path::new("examples/composite-checkout.laml")).unwrap();
    let boxes = place(&doc, &sheet).unwrap();
    let routes = route(&doc, &boxes, &sheet).unwrap();
    let svg = render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap();
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}
