use laml::parser::parse;
use laml::syntax::{Alignment, FlowDirection, Placement, StyleEntry};

#[test]
fn named_relation_uses_at_prefix_with_style_classes() {
    let source = "semantics {\nA\nB\n@signal(style-classes=critical) A -> B : message\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.relations[0].name.as_deref(), Some("signal"));
    assert_eq!(diagram.relations[0].classes, ["critical"]);
}

#[test]
fn parses_published_example() {
    let diagram = parse(include_str!("../examples/architecture.laml")).unwrap();
    assert_eq!(diagram.nodes.len(), 12);
    assert_eq!(diagram.relations.len(), 11);
    assert_eq!(diagram.layout.len(), 11);
}

#[test]
fn parses_documented_diagram_and_preserves_order() {
    let source = r##"
settings {
    direction: left-to-right
    naming-convention: uppercase-nodes-lowercase-relations
}
semantics {
    A(label="Backend #1 -> running"; style-classes=backend,critical) // comment
    B(style-classes=database)
    mqtt(style-classes=protocol,async): A -> B : MQTT
    (style-classes=protocol): B <- A : HTTP
    A <-> B
}
layout {
    A < B : align=top; gap=24
    B below A : align=left
}
styling {
    import "architecture.lamlstyle"
    node.backend {
        fill: "#124578"
        font-size: 14
    }
}
"##;
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.settings.direction, FlowDirection::LeftToRight);
    assert_eq!(
        diagram.nodes[0].label.as_deref(),
        Some("Backend #1 -> running")
    );
    assert_eq!(diagram.relations[0].name.as_deref(), Some("mqtt"));
    assert_eq!(diagram.relations[0].classes, ["protocol", "async"]);
    assert_eq!(diagram.relations[1].from, "A"); // B <- A
    assert_eq!(diagram.relations[1].to, "B");
    assert_eq!(diagram.layout[0].placement, Placement::Left);
    assert_eq!(diagram.layout[0].align, Some(Alignment::Top));
    assert_eq!(diagram.layout[0].gap, Some(24.0));
    assert_eq!(diagram.layout[1].placement, Placement::Below);
    assert!(
        matches!(&diagram.styling[0], StyleEntry::Import { path, .. } if path == "architecture.lamlstyle")
    );
    assert!(
        matches!(&diagram.styling[1], StyleEntry::Rule(rule) if rule.properties[0].value == "\"#124578\"")
    );
}

#[test]
fn symbolic_and_word_placements_have_equal_meaning() {
    let baseline = "semantics {\nA\nB\n}\nlayout {\nA < B\n}\n";
    let word = baseline.replace("A < B", "A left of B");
    assert_eq!(
        parse(baseline).unwrap().layout[0].placement,
        parse(&word).unwrap().layout[0].placement
    );
}

#[test]
fn errors_are_linked_to_source_lines() {
    let cases = [
        (
            "semantics {\nA\n}\nlayout {\nA < Missing\n}\n",
            5,
            "unknown node",
        ),
        (
            "semantics {\nA\nB\n}\nlayout {\nA < B : align=left\n}\n",
            6,
            "incompatible",
        ),
        (
            "semantics {\nA\nB\n}\nlayout {\nA < B : gap=-1\n}\n",
            6,
            "nonnegative",
        ),
        (
            "settings {\ndirection: left-to-right\ndirection: right-to-left\n}\nsemantics {\nA\n}\n",
            3,
            "duplicate setting",
        ),
        (
            "semantics {\nA(style-classes=foo; style-classes=bar)\n}\n",
            2,
            "duplicate attribute",
        ),
        (
            "settings {\nnaming-convention: uppercase-nodes-lowercase-relations\n}\nsemantics {\na\n}\n",
            5,
            "uppercase",
        ),
        (
            "semantics {\nA\nB\nname: A -> B : \"\"\n}\n",
            4,
            "empty relation label",
        ),
    ];
    for (source, expected_line, expected_message) in cases {
        let diagnostic = parse(source).unwrap_err();
        assert_eq!(diagnostic.line, expected_line, "{source}");
        assert!(
            diagnostic.message.contains(expected_message),
            "{diagnostic:?}"
        );
    }
}

#[test]
fn labels_accept_escaped_quotes_and_explicit_newlines() {
    let diagram = parse("semantics {\nA(label=\"say \\\"hi\\\"\\nagain\")\n}\n").unwrap();
    assert_eq!(diagram.nodes[0].label.as_deref(), Some("say \"hi\"\nagain"));
}
