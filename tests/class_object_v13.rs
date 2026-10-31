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
    assert!(svg.contains("+bestellen(): void"));
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
fn classifier_visibility_is_visible_but_does_not_change_the_node_id() {
    let source = "settings {\ntype: class\n}\nsemantics {\nabstract +class Dienst\n#interface Intern\nDienst ..|> Intern\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.nodes[0].id, "Dienst");
    assert_eq!(diagram.nodes[0].visibility, Some('+'));
    assert_eq!(diagram.nodes[1].id, "Intern");
    let svg = image(source);
    assert!(svg.contains("+Dienst {abstract}"));
    assert!(svg.contains("#Intern"));
    assert!(svg.contains("font-style=\"italic\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let bad = "settings {\ntype: object\n}\nsemantics {\n+object Eins\n}\n";
    assert!(parse(bad).unwrap_err().message.contains("visibility"));
}

#[test]
fn stereotypes_are_visible_without_becoming_style_classes_or_unchecked_bindings() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Dienst <<service>>\ninterface API\n@use Dienst ..|> API <<integration>> : verbindet\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.nodes[0].stereotypes, ["service"]);
    assert!(diagram.nodes[0].classes.is_empty());
    assert_eq!(diagram.relations[0].stereotypes, ["integration"]);
    let svg = image(source);
    assert!(svg.contains("«service»"));
    assert!(svg.contains("«integration»: verbindet"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let binding = source.replace("<<integration>>", "<<bind>>");
    assert!(
        parse(&binding)
            .unwrap_err()
            .message
            .contains("binding validation")
    );
}

#[test]
fn compact_template_name_and_anonymous_instance_have_stable_ids() {
    let template = "settings {\ntype: class\n}\nsemantics {\nclass Repository<T>\n}\n";
    let diagram = parse(template).unwrap();
    assert_eq!(diagram.nodes[0].id, "Repository");
    assert_eq!(diagram.nodes[0].templates, ["T"]);
    let svg = image(template);
    assert!(svg.contains(">Repository</text>"));
    assert!(svg.contains(">T</text>"));
    assert!(svg.contains("stroke-dasharray=\"4 3\""));
    let generic = "semantics {\nclass Repository<T>\n}\n";
    assert!(image(generic).contains("Repository&lt;T&gt;"));
    let fixed =
        format!("{template}styling {{\nnode[Repository] {{\nwidth: 40\nheight: 30\n}}\n}}\n");
    let diagram = parse(&fixed).unwrap();
    let sheet =
        laml::styles::load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(place(&diagram, &sheet).is_err());
    for head in ["Repository<T, T>", "Repository<T>Suffix", "Repository<>"] {
        let source = format!("settings {{\ntype: class\n}}\nsemantics {{\nclass {head}\n}}\n");
        assert!(parse(&source).is_err(), "{head}");
    }

    let anonymous =
        "settings {\ntype: object\n}\nsemantics {\n@gast anonymous object(of=Kunde)\n}\n";
    let diagram = parse(anonymous).unwrap();
    assert_eq!(diagram.nodes[0].id, "gast");
    assert_eq!(diagram.nodes[0].label.as_deref(), Some(":Kunde"));
    assert!(image(anonymous).contains(":Kunde"));
    let invalid = anonymous.replace("@gast ", "");
    assert!(parse(&invalid).unwrap_err().message.contains("@id"));
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
fn member_styles_select_id_kind_and_class_without_hiding_connected_members() {
    let source = "settings {\ntype: class\n}\nsemantics {\nA {\n@key(style-classes=primary) id: UUID\n--\n@run +void run()\n}\n}\nstyling {\nmember[kind=method] {\ndisplay: none\n}\nmember.primary {\ncolor: \"#aa0000\"\nfont-size: 18\n}\nnode[A] member[key] {\nfont-size: 20\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert_eq!(boxes[0].body.len(), 1);
    assert_eq!(boxes[0].body_styles[0].font_size, 20.0);
    let svg = render(&boxes, &[]).unwrap();
    assert!(svg.contains("id: UUID"));
    assert!(!svg.contains("void run"));
    assert!(svg.contains("fill=\"#aa0000\""));

    let hidden = "semantics {\nA {\n@secret id: UUID\n}\nB\n@r A --> secret\n}\nstyling {\nmember[secret] {\ndisplay: none\n}\n}\n";
    let diagram = parse(hidden).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("hidden member")
    );
    let hide_node = "semantics {\nA\n}\nstyling {\nnode[A] {\ndisplay: none\n}\n}\n";
    let diagram = parse(hide_node).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(place(&diagram, &sheet).unwrap().is_empty());
}

#[test]
fn hidden_nodes_and_relations_cannot_leave_visible_references_or_mandatory_rules() {
    let source = "semantics {\nA\nB\nC\n@hidden A --> B\n@shown B --> C\n}\nstyling {\nnode[A] {\ndisplay: none\n}\nrelation[hidden] {\ndisplay: none\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(boxes.len(), 2);
    assert_eq!(routes.len(), 1);
    assert_eq!(boxes[routes[0].from].id, "B");
    let svg = render(&boxes, &routes).unwrap();
    assert!(!svg.contains(">A</text>"));
    let referenced = source.replace("relation[hidden] {", "relation[shown] {");
    let diagram = parse(&referenced).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("hidden node")
    );
    let layout = source.replace("styling {", "layout {\nA < B\n}\nstyling {");
    let diagram = parse(&layout).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("mandatory layout")
    );
    let label_layout = source
        .replace(
            "styling {",
            "layout {\nrelation[hidden] : label=near\n}\nstyling {",
        )
        .replace("@hidden A --> B", "@hidden A --> B : ignored");
    let diagram = parse(&label_layout).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("hidden relation")
    );
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
fn multiline_note_keeps_text_lines_without_parsing_them_as_graph_compartments() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Kunde\nnote Hinweis {\nErste Zeile\n\n--\n\\}\nLetzte Zeile\n}\nHinweis .. Kunde\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram.nodes[1].label.as_deref(),
        Some("Erste Zeile\n\n--\n}\nLetzte Zeile")
    );
    assert!(diagram.nodes[1].parts.is_empty());
    let svg = image(source);
    assert!(svg.contains("Erste Zeile"));
    assert!(svg.contains("Letzte Zeile"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    assert!(
        parse("semantics {\nnote Hinweis {\n\n}\n}\n")
            .unwrap_err()
            .message
            .contains("visible text")
    );
    assert!(parse("semantics {\nnote Hinweis {\n--\n}\n}\n").is_ok());
    assert!(parse("semantics {\nnote Hinweis \"eins\" {\nzwei\n}\n}\n").is_err());
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
fn scoped_default_members_have_qualified_targets_and_reject_overload_ambiguity() {
    let source = "semantics {\nA {\nid: Int\n}\nB {\nid: Int\n}\n@r A::id --> B::id\n}\nstyling {\nmember[A::id] {\ncolor: \"#aa0000\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routed.len(), 1);
    let svg = render(&boxes, &routed).unwrap();
    assert!(svg.contains("data-member-id=\"A::id\""));
    assert!(svg.contains("data-member-id=\"B::id\""));
    assert!(svg.contains("fill=\"#aa0000\""));
    let overloaded = "semantics {\nA {\nmethod f()\nmethod f(Int x)\n}\nB\nA::f --> B\n}\n";
    assert!(
        parse(overloaded)
            .unwrap_err()
            .message
            .contains("ambiguous member")
    );
    let aliased = "semantics {\nA {\n@key id: Int\n}\nB\nA::id --> B\n}\n";
    assert!(
        parse(aliased)
            .unwrap_err()
            .message
            .contains("unknown node or member")
    );
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
fn association_end_layout_mandates_text_side_without_reversing_the_relation() {
    let source = "settings {\ntype: class\n}\nsemantics {\nA\nB\n@r A \"1\"/owner -- \"0..*\"/+parts B\n}\nlayout {\nA < B : align=center; gap=300\nrelation[r].source : multiplicity=below; role=above\nrelation[r].target : multiplicity=above; role=below\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routed[0].end_labels.len(), 4);
    let y = routed[0].points[0].y;
    assert!(routed[0].end_labels[0].y > y);
    assert!(routed[0].end_labels[1].y + routed[0].end_labels[1].height < y);
    assert!(routed[0].end_labels[2].y + routed[0].end_labels[2].height < y);
    assert!(routed[0].end_labels[3].y > y);
    let missing = source.replace("\"1\"/owner", "\"1\"");
    assert!(
        parse(&missing)
            .unwrap_err()
            .message
            .contains("missing multiplicity or role")
    );
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
