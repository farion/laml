use laml::export;
use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::StyleSheet;
use laml::svg::render;
use laml::syntax::{DiagramType, MemberKind, NodeKind, NodePart};

fn image(source: &str) -> String {
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    render(&boxes, &routes).unwrap()
}

#[test]
fn class_body_renders_real_compartments_in_all_formats() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Kunde {\nstatic int counter\n--\n+void bestellen()\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.settings.diagram_type, DiagramType::Class);
    assert_eq!(diagram.nodes[0].kind, NodeKind::Class);
    assert_eq!(diagram.nodes[0].parts.len(), 3);
    assert!(matches!(
        diagram.nodes[0].parts[2],
        NodePart::Member {
            kind: MemberKind::Method,
            ..
        }
    ));
    let svg = image(source);
    assert!(svg.contains("counter"));
    assert!(svg.contains("+void bestellen()"));
    assert!(svg.matches("<line ").count() >= 2);
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF"));
}

#[test]
fn classifier_kinds_use_distinct_visible_headers() {
    let source = "settings {\ntype: class\n}\nsemantics {\ninterface API {\n+void senden()\n}\nenum Status {\nOFFEN\nBEZAHLT\n}\ndatatype Geld {\nbetrag: Decimal\n}\n}\n";
    let svg = image(source);
    assert!(svg.contains("«interface»"));
    assert!(svg.contains("«enumeration»"));
    assert!(svg.contains("«dataType»"));
    assert!(svg.contains("BEZAHLT"));
}

#[test]
fn object_slots_and_underlined_header_are_visible() {
    let source = "settings {\ntype: object\n}\nsemantics {\n@kunde object kunde1(of=Kunde) {\nname = \"Ada\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.nodes[0].id, "kunde");
    assert_eq!(diagram.nodes[0].label.as_deref(), Some("kunde1:Kunde"));
    let svg = image(source);
    assert!(svg.contains("kunde1:Kunde"));
    assert!(svg.contains("name = &quot;Ada&quot;"));
    assert!(svg.matches("<line ").count() >= 2);
}

#[test]
fn class_and_object_compartment_errors_are_not_silent() {
    let cases = [
        (
            "settings {\ntype: class\n}\nsemantics {\nA {\nx: Int\n+void f()\n}\n}\n",
            "compartments",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nA {\n+void f()\n--\nx: Int\n}\n}\n",
            "compartment",
        ),
        (
            "settings {\ntype: object\n}\nsemantics {\nk {\n+void f()\n}\n}\n",
            "slot",
        ),
    ];
    for (source, expected) in cases {
        let err = parse(source).unwrap_err();
        assert!(err.message.contains(expected), "{err:?}");
    }
}

#[test]
fn unsupported_uml_arrow_is_rejected_instead_of_drawn_as_a_gne_edge() {
    let source = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA -> B\n}\n";
    let err = parse(source).unwrap_err();
    assert!(err.message.contains("not implemented yet"));
    assert!(parse("semantics {\nA\nB\nA -> B\n}\n").is_ok());
}

#[test]
fn undirected_links_prefer_proximity_but_yield_to_hard_layout() {
    let source = "semantics {\nA\nB\nA -- B\n}\n";
    let diagram = parse(source).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!(boxes[0].x + boxes[0].width < boxes[1].x);
    let explicit = "semantics {\nA\nB\nA -- B\n}\nlayout {\nB < A\n}\n";
    let diagram = parse(explicit).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!(boxes[1].x + boxes[1].width < boxes[0].x);
}

#[test]
fn generic_may_mix_rows_and_plain_relations_have_no_arrowhead() {
    let source = "semantics {\nA {\ntext \"--\"\n--\n+void f()\nx: Int\n}\nB\n@edge A -- B\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.nodes[0].parts.len(), 4);
    assert_eq!(diagram.relations[0].name.as_deref(), Some("edge"));
    let svg = image(source);
    assert!(svg.contains("+void f()"));
    assert!(svg.contains("x: Int"));
    assert!(!svg.contains("marker-end=\"url(#arrow)\""));
}
