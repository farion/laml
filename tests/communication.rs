use laml::{
    communication, export, geometry::place, layout::validate_ordering, route::route, styles::load,
    svg::TextMode,
};
use std::path::Path;

const EXAMPLE: &str = include_str!("../examples/communication-order.laml");
fn source(body: &str) -> String {
    format!("settings {{\ntype: communication\ninteraction: Checkout\n}}\nsemantics {{\n{body}\n}}")
}

#[test]
fn messages_use_links_but_keep_direction_and_explicit_sequence_numbers() {
    let doc = communication::parse(EXAMPLE).unwrap();
    assert!(
        laml::parser::parse(EXAMPLE)
            .unwrap_err()
            .message
            .contains("communication::parse")
    );
    assert_eq!(doc.graph.nodes.len(), 3);
    assert_eq!(doc.graph.relations.len(), 2);
    assert_eq!(doc.messages.len(), 4);
    assert_eq!(doc.messages[1].number, [1, 1]);
    assert_eq!(
        (&doc.messages[2].from[..], &doc.messages[2].to[..]),
        ("Payment", "Shop")
    );
    let sheet = load(&doc.graph, Path::new("examples/communication-order.laml")).unwrap();
    validate_ordering(&doc.graph).unwrap();
    let boxes = place(&doc.graph, &sheet).unwrap();
    let routes = route(&doc.graph, &boxes, &sheet).unwrap();
    let svg = communication::render(&doc, &boxes, &routes, TextMode::Embedded).unwrap();
    assert!(svg.contains("sd Checkout"));
    assert!(svg.contains("1: placeOrder()"));
    assert!(svg.contains("1.2: approved()"));
    assert!(svg.contains("Buyer:Customer"));
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
    let paths = communication::render(&doc, &boxes, &routes, TextMode::Paths).unwrap();
    assert!(!paths.contains("<text "));
    assert!(paths.contains("<path"));
}

#[test]
fn explicit_links_cannot_be_replaced_by_message_arrows() {
    let base = "object A(of=Client)\nobject B(of=Service)\nRELATION\nmessage 1 A --> B : call()";
    for (edge, reason) in [
        ("A --> B", "operator"),
        ("A .. B", "dotted UML annotation"),
        ("A -- B : message", "communication link"),
        ("A -- B <<dependency>>", "communication link"),
        ("A -- A", "communication link"),
    ] {
        let err = communication::parse(&source(&base.replace("RELATION", edge))).unwrap_err();
        assert!(err.message.contains(reason), "{edge}: {err:?}");
    }
    assert!(
        communication::parse(&source("object A\nobject B\nmessage 1 A --> B : call()"))
            .unwrap_err()
            .message
            .contains("links and numbered messages")
    );
}

#[test]
fn sequence_expressions_and_message_targets_are_checked() {
    let prefix = "object A\nobject B\nobject C\nA -- B\n";
    for (messages, reason) in [
        ("message 0 A --> B : call()", "positive"),
        ("message 01 A --> B : call()", "positive"),
        ("message 1a A --> B : call()", "positive"),
        ("message 1.1 A --> B : nested()", "parent message"),
        (
            "message 1 A --> B : call()\nmessage 1 B --> A : reply()",
            "duplicate",
        ),
        ("message 1 A --> C : other()", "one undirected link"),
        ("message 1 A --> A : recursive()", "self messages"),
        (
            "message 1 A --> B : call()\nmessage 2 A --> B : [guard] other()",
            "guard and iteration",
        ),
        ("message 1 A -> B : call()", "Sender --> Receiver"),
        ("message 1 A --> B : call(", "guard and iteration"),
        ("message 1 A --> B : call())", "guard and iteration"),
        (
            "message 1 A --> B : call(\"unfinished)",
            "guard and iteration",
        ),
    ] {
        let err = communication::parse(&source(&format!("{prefix}{messages}"))).unwrap_err();
        assert!(err.message.contains(reason), "{messages}: {err:?}");
    }
}

#[test]
fn numbered_links_must_stay_visible() {
    let text = format!("{EXAMPLE}\nstyling {{\nrelation {{\ndisplay: none\n}}\n}}");
    let doc = communication::parse(&text).unwrap();
    let sheet = load(&doc.graph, Path::new("examples/communication-order.laml")).unwrap();
    assert!(
        place(&doc.graph, &sheet)
            .unwrap_err()
            .message
            .contains("communication links must remain visible")
    );
}

#[test]
fn duplicate_links_and_missing_interaction_name_fail() {
    let body = "object A\nobject B\nA -- B\nB -- A\nmessage 1 A --> B : call()";
    assert!(
        communication::parse(&source(body))
            .unwrap_err()
            .message
            .contains("exactly one undirected link")
    );
    let missing = source("object A\nobject B\nA -- B\nmessage 1 A --> B : call()")
        .replace("interaction: Checkout\n", "");
    assert!(
        communication::parse(&missing)
            .unwrap_err()
            .message
            .contains("interaction: Name")
    );
}

#[test]
fn message_labels_escape_svg_and_no_hidden_execution_notation_is_inferred() {
    let text = source("object A\nobject B\nA -- B\nmessage 1 A --> B : send(\"<unsafe&>\")\n")
        + "\nlayout {\nA < B : gap=220\n}";
    let doc = communication::parse(&text).unwrap();
    let sheet = load(&doc.graph, Path::new("examples/communication-order.laml")).unwrap();
    let boxes = place(&doc.graph, &sheet).unwrap();
    let routes = route(&doc.graph, &boxes, &sheet).unwrap();
    let svg = communication::render(&doc, &boxes, &routes, TextMode::Embedded).unwrap();
    assert!(svg.contains("&lt;unsafe&amp;&gt;"));
    assert!(!svg.contains("<unsafe&>"));
}
