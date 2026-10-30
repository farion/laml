use laml::export;
use laml::geometry::{NodeShape, place};
use laml::parser::parse;
use laml::route::route;
use laml::styles::{StyleSheet, load};
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
fn generic_shapes_route_to_visible_ellipse_and_diamond_outlines() {
    let source = "semantics {\nA\nB\nA -- B\n}\nstyling {\nnode[A] {\nshape: ellipse\n}\nnode[B] {\nshape: diamond\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!(boxes[0].shape, NodeShape::Ellipse);
    assert_eq!(boxes[1].shape, NodeShape::Diamond);
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let first = routes[0].points[0];
    let last = *routes[0].points.last().unwrap();
    let (a, b) = (&boxes[0], &boxes[1]);
    let ellipse = ((first.x - a.x - a.width / 2.0) / (a.width / 2.0)).powi(2)
        + ((first.y - a.y - a.height / 2.0) / (a.height / 2.0)).powi(2);
    let diamond = (last.x - b.x - b.width / 2.0).abs() / (b.width / 2.0)
        + (last.y - b.y - b.height / 2.0).abs() / (b.height / 2.0);
    assert!((ellipse - 1.0).abs() < 1e-6);
    assert!((diamond - 1.0).abs() < 1e-6);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("<ellipse "));
    assert!(svg.contains("<polygon "));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let uml = source.replacen("semantics {", "settings {\ntype: class\n}\nsemantics {", 1);
    let diagram = parse(&uml).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &styles)
            .unwrap_err()
            .message
            .contains("UML node form")
    );
}

#[test]
fn notes_render_folded_shape_and_annotate_nodes_in_both_uml_types() {
    for (kind, named) in [
        ("class", "class Kunde"),
        ("object", "object kunde1(of=Kunde)"),
    ] {
        let source = format!(
            "settings {{\ntype: {kind}\n}}\nsemantics {{\n{named}\nnote Hinweis \"Nur aktive Kunden\"\nHinweis .. {}\n}}\n",
            if kind == "class" { "Kunde" } else { "kunde1" }
        );
        let svg = image(&source);
        assert!(svg.contains("Nur aktive Kunden"));
        assert!(svg.contains("stroke-dasharray=\"6 4\""));
        assert!(svg.contains("<path d=\"M"));
        assert!(export::png(&svg).is_ok());
    }
    let bad = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA .. B\n}\n";
    assert!(parse(bad).unwrap_err().message.contains("requires a note"));
}

#[test]
fn relations_can_connect_explicitly_named_member_rows() {
    let source = "semantics {\nKunde {\n@kundenId id: UUID\n}\nBestellung {\n@kundeId kundeId: UUID\n}\n@fk kundeId --> kundenId\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].from, 1);
    assert_eq!(routes[0].to, 0);
    assert!(routes[0].points[0].y > boxes[1].y + boxes[1].font_size * 1.25);
    assert!(routes[0].points.last().unwrap().y > boxes[0].y + boxes[0].font_size * 1.25);
    assert!(image(source).contains("data-member-id=\"kundeId\""));
}

#[test]
fn explicitly_assigned_ids_are_unique_across_graph_element_kinds() {
    for source in [
        "semantics {\n@same class A {\n@same id: Int\n}\n}\n",
        "semantics {\nA {\n@same id: Int\n}\nB\n@same A -- B\n}\n",
        "semantics {\nA\nB\n@A A -- B\n}\n",
    ] {
        assert!(
            parse(source).unwrap_err().message.contains("global ID"),
            "{source}"
        );
    }
}

#[test]
fn association_end_multiplicities_and_roles_render_and_validate() {
    let source = "settings {\ntype: class\n}\nsemantics {\nKunde\nBestellung\n@orders Kunde \"1\"/kunde -- \"0..*\"/+bestellungen Bestellung : bestellt\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram.relations[0].source_end.multiplicity.as_deref(),
        Some("1")
    );
    assert_eq!(
        diagram.relations[0].target_end.role.as_deref(),
        Some("+bestellungen")
    );
    let svg = image(source);
    for text in ["kunde", "0..*", "+bestellungen", "bestellt"] {
        assert!(svg.contains(text), "{text}");
    }
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    for (diagram_type, adornment, expected) in [
        ("class", "\"3..1\"", "multiplicity"),
        ("object", "\"0..*\"", "object links"),
    ] {
        let text = format!(
            "settings {{\ntype: {diagram_type}\n}}\nsemantics {{\nA\nB\nA {adornment} -- B\n}}\n"
        );
        assert!(parse(&text).unwrap_err().message.contains(expected));
    }
}

#[test]
fn uml_operators_render_distinct_semantic_markers() {
    for (operator, expected, dashed) in [
        ("--> ", "marker-end=\"url(#arrow)\"", false),
        ("--|> ", "marker-end=\"url(#triangle)\"", false),
        ("..|> ", "marker-end=\"url(#triangle)\"", true),
        ("..> ", "marker-end=\"url(#arrow)\"", true),
        ("*-- ", "marker-start=\"url(#filled-diamond)\"", false),
        ("o-- ", "marker-start=\"url(#open-diamond)\"", false),
        ("*--> ", "marker-start=\"url(#filled-diamond)\"", false),
    ] {
        let source = format!(
            "settings {{\ntype: class\n}}\nsemantics {{\nWhole\nPart\nWhole {operator}Part\n}}\n"
        );
        let svg = image(&source);
        assert!(svg.contains(expected), "{operator}");
        assert_eq!(
            svg.contains("stroke-dasharray=\"6 4\""),
            dashed,
            "{operator}"
        );
        assert!(export::png(&svg).is_ok(), "{operator}");
        assert!(export::pdf(&svg).is_ok(), "{operator}");
    }
    let object = "settings {\ntype: object\n}\nsemantics {\na\nb\na *-- b\n}\n";
    assert!(parse(object).unwrap_err().message.contains("not valid"));
}

#[test]
fn unsupported_uml_arrow_is_rejected_instead_of_drawn_as_a_gne_edge() {
    let source = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA -> B\n}\n";
    let err = parse(source).unwrap_err();
    assert!(err.message.contains("not valid"));
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
