use laml::{
    export, geometry::place, interaction_overview as overview, layout::validate_ordering,
    parser::parse as generic_parse, route::route, styles::load, svg::TextMode,
};
use std::path::Path;
const PURCHASE: &str = include_str!("../examples/interaction-overview-purchase.laml");
const PARALLEL: &str = include_str!("../examples/interaction-overview-parallel.laml");
fn source(body: &str) -> String {
    format!(
        "settings {{\ntype: interaction-overview\ninteraction: Checkout\n}}\nsemantics {{\n{body}\n}}"
    )
}

#[test]
fn examples_draw_ref_frames_and_control_symbols_in_all_formats() {
    for (text, path) in [
        (PURCHASE, "examples/interaction-overview-purchase.laml"),
        (PARALLEL, "examples/interaction-overview-parallel.laml"),
    ] {
        let doc = overview::parse(text).unwrap();
        assert_eq!(
            doc.graph.settings.diagram_type,
            laml::syntax::DiagramType::InteractionOverview
        );
        assert!(
            generic_parse(text)
                .unwrap_err()
                .message
                .contains("specialized parser")
        );
        let styles = load(&doc.graph, Path::new(path)).unwrap();
        validate_ordering(&doc.graph).unwrap();
        let boxes = place(&doc.graph, &styles).unwrap();
        let routes = route(&doc.graph, &boxes, &styles).unwrap();
        let svg = overview::render(&doc, &boxes, &routes, TextMode::Embedded).unwrap();
        assert!(svg.contains("sd "));
        assert!(svg.contains(">ref</text>"));
        assert!(svg.contains("data-node-id=\""));
        assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
        assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
        let paths = overview::render(&doc, &boxes, &routes, TextMode::Paths).unwrap();
        assert!(!paths.contains("<text "));
    }
}

#[test]
fn rejects_misleading_activity_notation_and_missing_ref() {
    for (node, why) in [
        ("action A", "element kind"),
        ("objectNode A", "element kind"),
        ("interaction-use A", "requires ref"),
        ("interaction-use A(ref=123)", "unknown node attribute"),
        ("interaction-use A(ref=Other) {\n}", "without bodies"),
        (
            "interaction-use A(ref=Other;label=\"A\")",
            "unknown node attribute",
        ),
    ] {
        let err = overview::parse(&source(&format!(
            "initial Start\n{node}\nfinal End\nStart --> A\nA --> End"
        )))
        .unwrap_err();
        assert!(err.message.contains(why), "{node}: {err:?}");
    }
    let missing =
        source("initial Start\ninteraction-use A(ref=Other)\nfinal End\nStart --> A\nA --> End")
            .replace("interaction: Checkout\n", "");
    assert!(
        overview::parse(&missing)
            .unwrap_err()
            .message
            .contains("interaction: Name")
    );
}

#[test]
fn accepts_nested_alternatives_and_parallel_branches() {
    let text = source(
        "initial S\ninteraction-use U(ref=Use)\ndecision D\nfork F\ninteraction-use A(ref=A)\ninteraction-use B(ref=B)\njoin J\nmerge M\nfinal T\nS --> D\nD --> F : [yes]\nD --> U : [else]\nF --> A\nF --> B\nA --> J\nB --> J\nJ --> M\nU --> M\nM --> T",
    );
    assert_eq!(overview::parse(&text).unwrap().graph.nodes.len(), 9);
}

#[test]
fn rejects_crossed_and_unfinished_branches_cycles_and_missing_vertices() {
    let cases = [
        (
            "initial S\ndecision D\nfork F\ninteraction-use A(ref=A)\ninteraction-use B(ref=B)\ninteraction-use C(ref=C)\nmerge M\njoin J\nfinal T\nS --> D\nD --> F : [yes]\nD --> A : [else]\nF --> B\nF --> C\nA --> M\nC --> M\nM --> J\nB --> J\nJ --> T",
            "properly nested",
        ),
        (
            "initial S\ninteraction-use A(ref=A)\ndecision D\ninteraction-use B(ref=B)\ninteraction-use C(ref=C)\nmerge M\nfinal T\nS --> D\nD --> A : [x]\nD --> B : [else]\nA --> M\nB --> C\nC --> T\nM --> T",
            "topology",
        ),
        (
            "initial S\ninteraction-use A(ref=A)\ninteraction-use B(ref=B)\nfinal T\nS --> A\nA --> B\nB --> A\nB --> T",
            "topology",
        ),
        (
            "initial S\ninteraction-use A(ref=A)\nfinal T\nS --> A\nA --> T\nA --> T",
            "duplicate",
        ),
        (
            "initial S\ninteraction-use A(ref=A)\nfinal T\nS --> A\nA --> T\ninitial X",
            "exactly one",
        ),
    ];
    for (body, why) in cases {
        let err = overview::parse(&source(body)).unwrap_err();
        assert!(err.message.contains(why), "{why}: {err:?}");
    }
}

#[test]
fn rejects_wrong_flow_labels_and_operators() {
    let base = "initial S\ninteraction-use A(ref=A)\nfinal T\nS --> A\nA --> T";
    for (bad, why) in [
        ("S --> A : arbitrary", "only decision"),
        ("S -- A", "operator"),
        ("S .. A", "note endpoint"),
        ("S --> A <<include>>", "plain directed"),
    ] {
        let err = overview::parse(&source(&base.replace("S --> A", bad))).unwrap_err();
        assert!(err.message.contains(why), "{bad}: {err:?}");
    }
}

#[test]
fn hides_neither_ref_nodes_nor_control_flows() {
    let doc = overview::parse(&format!(
        "{PURCHASE}\nstyling {{\nnode[SignIn] {{\ndisplay: none\n}}\n}}"
    ))
    .unwrap();
    let sheet = load(
        &doc.graph,
        Path::new("examples/interaction-overview-purchase.laml"),
    )
    .unwrap();
    assert!(
        place(&doc.graph, &sheet)
            .unwrap_err()
            .message
            .contains("overview nodes must remain visible")
    );
    let doc = overview::parse(&format!(
        "{PURCHASE}\nstyling {{\nrelation[approved] {{\ndisplay: none\n}}\n}}"
    ))
    .unwrap();
    let sheet = load(
        &doc.graph,
        Path::new("examples/interaction-overview-purchase.laml"),
    )
    .unwrap();
    assert!(
        place(&doc.graph, &sheet)
            .unwrap_err()
            .message
            .contains("overview flows must remain visible")
    );
}
