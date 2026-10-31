use laml::{export, styles::load, svg::TextMode, syntax::Diagram, timing};
use std::path::Path;
const EXAMPLE: &str = include_str!("../examples/timing-access.laml");
fn source(semantics: &str, layout: &str) -> String {
    format!(
        "settings {{\ntype: timing\ninteraction: Access\ntime-unit: ms\n}}\nsemantics {{\n{semantics}\n}}\nlayout {{\n{layout}\n}}"
    )
}
fn base() -> String {
    source(
        "lifeline User(of=Customer) : Idle\nstate User at 2 : Busy",
        "tick 0\ntick 2",
    )
}

#[test]
fn draws_numeric_discrete_lifelines_and_exports_all_formats() {
    let doc = timing::parse(EXAMPLE).unwrap();
    assert_eq!(doc.lifelines.len(), 2);
    assert_eq!(doc.lifelines[0].changes.len(), 3);
    let styles = load(
        &Diagram {
            styling: doc.styling.clone(),
            ..Diagram::default()
        },
        Path::new("examples/timing-access.laml"),
    )
    .unwrap();
    let svg = timing::render(&doc, &styles, TextMode::Embedded).unwrap();
    assert!(svg.contains("sd Access"));
    assert!(svg.contains("WaitCard"));
    assert!(svg.contains("time (ms)"));
    assert!(svg.contains("CardInserted"));
    assert!(svg.contains("rotate(-90"));
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
    let paths = timing::render(&doc, &styles, TextMode::Paths).unwrap();
    assert!(!paths.contains("<text "));
    assert!(paths.contains("<path"));
}

#[test]
fn parser_rejects_unsupported_or_false_time_notation() {
    for (source, part) in [
        (
            base().replace("state User at 2 : Busy", "state User at -2 : Busy"),
            "nonnegative",
        ),
        (
            base().replace("state User at 2 : Busy", "state User at 0 : Busy"),
            "increasing",
        ),
        (
            base().replace("state User at 2 : Busy", "state User at 2 : Idle"),
            "unchanged",
        ),
        (base().replace("tick 2", "tick 0"), "duplicate"),
        (base().replace("tick 2", ""), "positive tick"),
        (
            base().replace("tick 2", "end: 2\ntick 2"),
            "end must exceed",
        ),
        (base().replace("tick 2", "scale: 0\ntick 2"), "positive"),
        (
            base().replace("state User at 2 : Busy", "message User --> Server : call()"),
            "unsupported timing declaration",
        ),
        (
            base().replace(
                "lifeline User(of=Customer) : Idle",
                "lifeline User(of=Customer) : Idle\nlifeline User : Ready",
            ),
            "duplicate",
        ),
        (
            base().replace("state User at 2 : Busy", "state Ghost at 2 : Busy"),
            "earlier lifeline",
        ),
        (
            base().replace(
                "state User at 2 : Busy",
                "state User at 2 : Busy\nevent User at 1 : Wake",
            ),
            "explicit state transition",
        ),
        (
            base().replace("interaction: Access\n", ""),
            "interaction: Name",
        ),
    ] {
        let error = timing::parse(&source).unwrap_err();
        assert!(error.message.contains(part), "{part}: {error:?}");
    }
}

#[test]
fn dimensions_and_event_labels_must_fit_without_retiming() {
    let doc = timing::parse(&source(
        "lifeline A : Idle\nstate A at 1 : Busy\nstate A at 1.05 : Done",
        "tick 0\ntick 1\ntick 1.05\nscale: 70",
    ))
    .unwrap();
    let sheet = Default::default();
    assert!(
        timing::render(&doc, &sheet, TextMode::Embedded)
            .unwrap_err()
            .contains("tick")
    );
    let doc = timing::parse(&source(
        "lifeline A : Idle\nstate A at 1 : Busy\nstate A at 1.05 : Done",
        "tick 0\ntick 1\nscale: 70",
    ))
    .unwrap();
    assert!(
        timing::render(&doc, &sheet, TextMode::Embedded)
            .unwrap_err()
            .contains("too close")
    );
    let doc = timing::parse(&source(
        "lifeline A : Idle\nstate A at 2 : Busy\nevent A at 2 : ExtremelyLongStimulus",
        "tick 0\ntick 2\nend: 2.01",
    ))
    .unwrap();
    assert!(
        timing::render(&doc, &sheet, TextMode::Embedded)
            .unwrap_err()
            .contains("event label")
    );
}

#[test]
fn styling_is_checked_and_does_not_hide_required_timelines() {
    for (rule, part) in [
        ("display: none", "does not support 'display'"),
        ("shape: ellipse", "does not support 'shape'"),
        ("font-size: 30", "font-size 10..20"),
        ("border-color: none", "must remain visible"),
    ] {
        let text = base() + &format!("\nstyling {{\nnode[User] {{\n{rule}\n}}\n}}");
        let doc = timing::parse(&text).unwrap();
        let input = Diagram {
            styling: doc.styling.clone(),
            ..Diagram::default()
        };
        let sheet = load(&input, Path::new("examples/timing-access.laml")).unwrap();
        assert!(
            timing::render(&doc, &sheet, TextMode::Embedded)
                .unwrap_err()
                .contains(part),
            "{rule}"
        );
    }
}

#[test]
fn labeled_events_are_plain_text_and_lifelines_share_one_time_scale() {
    let text = source(
        "lifeline A : Idle\nstate A at 2 : Busy\nevent A at 2 : Wait & Check\nlifeline B : Ready\nstate B at 2 : Working",
        "tick 0\ntick 2\nend: 5",
    );
    let doc = timing::parse(&text).unwrap();
    let svg = timing::render(&doc, &Default::default(), TextMode::System).unwrap();
    assert!(svg.contains("Wait &amp; Check"));
    assert!(svg.matches("M").count() > 1);
    assert_eq!(
        doc.lifelines[0].changes[0].time,
        doc.lifelines[1].changes[0].time
    );
}
