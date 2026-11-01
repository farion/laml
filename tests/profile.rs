use laml::export;
use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::load;
use laml::svg::{TextMode, render_with_mode};
use laml::syntax::{DiagramType, EndSymbol, NodeKind};
use std::path::Path;

const EXAMPLE: &str = include_str!("../examples/profile-audit.laml");
fn source() -> String {
    EXAMPLE.replace("\r\n", "\n")
}
fn graph(text: &str) -> laml::syntax::Diagram {
    parse(text).unwrap()
}
fn failure(text: &str, part: &str) {
    let err = parse(text).unwrap_err();
    assert!(err.message.contains(part), "expected '{part}', got {err:?}");
}
fn blocks(text: &str) -> String {
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
fn profile_structure_and_filled_extension_are_checked() {
    let d = graph(EXAMPLE);
    assert_eq!(d.settings.diagram_type, DiagramType::Profile);
    let owner = d.nodes.iter().find(|n| n.id == "Audit").unwrap();
    let stereotype = d.nodes.iter().find(|n| n.id == "Audit.Tracked").unwrap();
    let metaclass = d.nodes.iter().find(|n| n.id == "Class").unwrap();
    assert_eq!(owner.kind, NodeKind::Profile);
    assert_eq!(stereotype.kind, NodeKind::Stereotype);
    assert_eq!(stereotype.parent.as_deref(), Some("Audit"));
    assert_eq!(metaclass.kind, NodeKind::Metaclass);
    assert_eq!(
        d.relations
            .iter()
            .find(|r| r.from == stereotype.id)
            .unwrap()
            .appearance
            .end,
        EndSymbol::FilledTriangle
    );
    let sheet = load(&d, Path::new("examples/profile-audit.laml")).unwrap();
    let boxes = place(&d, &sheet).unwrap();
    let routes = route(&d, &boxes, &sheet).unwrap();
    let svg = render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap();
    assert!(svg.contains("id=\"filled-triangle\""));
    assert!(svg.contains("marker-end=\"url(#filled-triangle)\""));
    assert!(
        svg.contains("«profile»") && svg.contains("«stereotype»") && svg.contains("«metaclass»")
    );
    assert!(svg.contains("«apply»") && svg.contains("«reference»"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
}

#[test]
fn extension_does_not_become_generalization_or_unmarked_dependency() {
    failure(&source().replace("--^>", "--|>"), "operator");
    failure(
        &source().replace("Audit.Tracked --^> Class", "Audit.Tracked ..> Class"),
        "profile relationships",
    );
    failure(
        &source().replace("Audit.Tracked --^> Class", "Class --^> Audit.Tracked"),
        "profile relationships",
    );
    failure(
        &source().replace("Commerce ..> Audit <<apply>>", "Commerce ..> Audit"),
        "profile relationships",
    );
    failure(
        &source().replace(
            "Audit ..> Class <<reference>>",
            "Audit ..> Class <<import>>",
        ),
        "profile relationships",
    );
    failure(
        &source().replace(
            "Commerce ..> Audit <<apply>>",
            "Audit ..> Commerce <<apply>>",
        ),
        "profile relationships",
    );
    failure(
        &source().replace(
            "Audit.Tracked --^> Class",
            "Audit.Tracked --^> Class <<required>>",
        ),
        "profile relationships",
    );
    failure(
        &source().replace(
            "Commerce ..> Audit <<apply>>",
            "Commerce ..> Audit <<apply>> : some label",
        ),
        "profile relationships",
    );
}

#[test]
fn profile_scope_and_metaclass_identity_are_explicit() {
    failure(
        &source().replace("stereotype Tracked", "class Tracked"),
        "element kind",
    );
    failure("semantics {\nprofile Demo\n}", "element kind");
    failure(
        "settings {\ntype: class\n}\nsemantics {\nstereotype Demo\n}",
        "element kind",
    );
    failure(
        &source().replace("stereotype Tracked", "package Tracked"),
        "external leaf",
    );
    failure(
        &source().replace(
            "    profile Audit {\n        stereotype Tracked\n    }",
            "    stereotype Tracked",
        ),
        "profile diagram requires",
    );
    failure(
        &source().replace(
            "    metaclass Class",
            "    profile Empty {\n    }\n    metaclass Class",
        ),
        "each profile",
    );
    failure(
        &source().replace("metaclass Class", "metaclass Forgotten"),
        "metaclass subset",
    );
    failure(
        &source().replace("metaclass Class", "class Class"),
        "element kind",
    );
    failure(
        &source().replace("profile Audit {", "profile Audit <<custom>> {"),
        "unchecked node modifiers",
    );
    failure(
        &source().replace("profile Audit {", "profile Audit(label=\"Other\") {"),
        "custom labels",
    );
    failure(
        &source().replace("Audit.Tracked --^> Class", ""),
        "needs an explicit extension",
    );
    failure(
        &source().replace(
            "Audit.Tracked --^> Class",
            "Audit.Tracked --^> Class\n    Audit.Tracked --^> Class",
        ),
        "duplicate profile",
    );
}

#[test]
fn optional_reference_and_strict_application_have_distinct_notation() {
    graph(&source().replace("Audit ..> Class <<reference>>", ""));
    let d = graph(&source().replace("<<apply>>", "<<strict>>"));
    assert_eq!(
        d.relations.last().unwrap().stereotypes.as_slice(),
        &["strict"]
    );
    graph(&source().replace("Commerce ..> Audit <<apply>>", ""));
}

#[test]
fn mandatory_visuals_cannot_be_styled_away() {
    for (style, part) in [
        (
            "node[Audit.Tracked] {\ndisplay: none\n}",
            "must remain visible",
        ),
        (
            "relation {\ndisplay: none\n}",
            "profile relationships must remain visible",
        ),
        ("node[Audit] {\nborder-color: none\n}", "visible outline"),
        ("node[Audit.Tracked] {\ncolor: none\n}", "visible outline"),
    ] {
        let source = format!("{EXAMPLE}\nstyling {{\n{style}\n}}");
        let d = graph(&source);
        let sheet = load(&d, Path::new("examples/profile-audit.laml")).unwrap();
        let err = place(&d, &sheet).unwrap_err();
        assert!(err.message.contains(part), "{err:?}");
    }
    for style in ["color: none", "width: 0"] {
        let source = format!("{EXAMPLE}\nstyling {{\nrelation {{\n{style}\n}}\n}}");
        let d = graph(&source);
        let sheet = load(&d, Path::new("examples/profile-audit.laml")).unwrap();
        let b = place(&d, &sheet).unwrap();
        let err = route(&d, &b, &sheet).unwrap_err();
        assert!(
            err.message.contains("stroke must remain visible"),
            "{err:?}"
        );
    }
}

#[test]
fn guide_block_is_buildable_and_line_ending_agnostic() {
    let guide = std::fs::read_to_string("docs/profile-diagrams.md")
        .unwrap()
        .replace("\r\n", "\n");
    let code = blocks(&guide);
    assert_eq!(
        code.trim(),
        source().trim(),
        "guide must match published example"
    );
    assert_eq!(code, blocks(&guide.replace('\n', "\r\n")));
    let d = graph(&code);
    let sheet = load(&d, Path::new("examples/profile-audit.laml")).unwrap();
    let boxes = place(&d, &sheet).unwrap();
    let routes = route(&d, &boxes, &sheet).unwrap();
    assert!(
        render_with_mode(&boxes, &routes, TextMode::Paths)
            .unwrap()
            .contains("<svg")
    );
}
