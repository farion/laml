use laml::{export, geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

#[test]
fn member_modifiers_are_order_independent_and_deduplicated() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass C {\nstatic field int counter\n--\nabstract method +void start()\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert!(diagram.nodes[0].has_endpoint("C::counter"));
    assert!(diagram.nodes[0].has_endpoint("C::start"));
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let image = render(&boxes, &[]).unwrap();
    assert!(image.contains("counter: int"));
    assert!(image.contains("+start(): void"));
    let one_parameter = source.replace("+void start()", "+void start(int timeoutms)");
    let diagram = parse(&one_parameter).unwrap();
    assert!(
        matches!(&diagram.nodes[0].parts[2], laml::syntax::NodePart::Member {text,..} if text == "+start(timeoutms: int): void")
    );
    let conventional = source.replace("+void start()", "+start(timeoutms: int): void");
    assert!(parse(&conventional).is_ok());
    let malformed_field = source.replace("static field int counter", "field int counter extra");
    assert!(
        parse(&malformed_field)
            .unwrap_err()
            .message
            .contains("unsupported programming-style UML attribute")
    );
    let malformed_type = source.replace("static field int counter", "field counter: ???");
    assert!(
        parse(&malformed_type)
            .unwrap_err()
            .message
            .contains("attribute name or type")
    );
    let malformed = source.replace("+void start()", "+start(int timeoutms): void");
    assert!(
        parse(&malformed)
            .unwrap_err()
            .message
            .contains("parameters require")
    );
    let trailing = source.replace("+void start()", "+void start(int x,)");
    assert!(
        parse(&trailing)
            .unwrap_err()
            .message
            .contains("empty UML operation parameter")
    );
    assert!(!image.contains("field int counter"));
    for invalid in [
        "static static field int counter",
        "field method int counter",
        "abstract abstract method +void start()",
    ] {
        assert!(parse(&source.replace("static field int counter", invalid)).is_err());
    }
}

#[test]
fn object_links_can_name_ends_but_cannot_assign_multiplicities() {
    let source = "settings {\ntype: object\n}\nsemantics {\na(of=A)\nb(of=B)\n@link a /owner -- /item b\n}\nlayout {\na < b : gap=180\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let nodes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &nodes, &styles).unwrap();
    let svg = render(&nodes, &routes).unwrap();
    assert!(svg.contains("owner"));
    assert!(svg.contains("item"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let invalid_classifier = source.replace("of=A", "of=???");
    assert!(
        parse(&invalid_classifier)
            .unwrap_err()
            .message
            .contains("classifier must")
    );
    let bad = source.replace("a /owner", "a \"1\"/owner");
    assert!(
        parse(&bad)
            .unwrap_err()
            .message
            .contains("cannot show multiplicities")
    );
}

#[test]
fn uml_members_reject_modifiers_and_kinds_that_misstate_notation() {
    for (setting, body, expected) in [
        (
            "type: object",
            "object O {\ntext \"anything\"\n}",
            "only name = value slots",
        ),
        (
            "type: object",
            "object O {\nstatic id = 1\n}",
            "only name = value slots",
        ),
        (
            "type: class",
            "enum E {\nstatic READY\n}",
            "enumeration literals cannot",
        ),
        (
            "type: class",
            "enum E {\nmethod READY\n}",
            "enumeration literals cannot",
        ),
        (
            "type: class",
            "class C {\nabstract id: UUID\n}",
            "attributes cannot be abstract",
        ),
        (
            "type: class",
            "class C {\nfield +go()\n}",
            "attribute cannot use operation parentheses",
        ),
        (
            "type: class",
            "class C {\nmethod +go: Void\n}",
            "operation requires parentheses",
        ),
    ] {
        let diagram = format!("settings {{\n{setting}\n}}\nsemantics {{\n{body}\n}}\n");
        let err = parse(&diagram).unwrap_err();
        assert!(err.message.contains(expected), "{err:?}");
    }
}
