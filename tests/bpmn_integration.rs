use laml::{bpmn, export, styles, svg::TextMode, syntax::Diagram};
use std::{fs, path::Path};

fn example(name: &str) -> (bpmn::Document, styles::StyleSheet) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(format!("{name}.laml"));
    let source = fs::read_to_string(&path).unwrap();
    let doc = bpmn::parse(&source).unwrap();
    let sheet = styles::load(
        &Diagram {
            styling: doc.styling.clone(),
            ..Default::default()
        },
        &path,
    )
    .unwrap();
    bpmn::validate_styles(&doc, &sheet).unwrap();
    bpmn::validate_geometry(&doc).unwrap();
    (doc, sheet)
}

#[test]
fn all_bpmn_examples_export_to_every_format() {
    for name in [
        "bpmn-basic",
        "bpmn-order",
        "bpmn-subprocess",
        "bpmn-vertical",
    ] {
        let (doc, sheet) = example(name);
        for mode in [TextMode::Embedded, TextMode::System, TextMode::Paths] {
            let svg = bpmn::render_with_styles(&doc, &sheet, mode).unwrap();
            assert!(svg.starts_with("<svg "), "{name}");
            assert!(svg.ends_with("</svg>\n"), "{name}");
            let png = export::png(&svg).unwrap();
            let pdf = export::pdf(&svg).unwrap();
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "{name}");
            assert!(pdf.starts_with(b"%PDF-"), "{name}");
            assert!(
                svg.contains("phosphor-license"),
                "{name} needs its icon attribution"
            );
        }
    }
}

#[test]
fn gateway_side_rules_parse_validate_and_reject_conflicts() {
    let source = include_str!("../examples/bpmn-basic.laml");
    let custom = format!(
        "{source}\nlayout {{\nflow[Complete->Approve] : from=right\nflow[RequestDocuments->Merge] : to=left\n}}"
    );
    let doc = bpmn::parse(&custom).unwrap();
    bpmn::validate_geometry(&doc).unwrap();
    assert_eq!(doc.flow_layout.len(), 2);
    assert_eq!(doc.flow_layout[0].from_side, Some(bpmn::Side::Right));
    assert_eq!(doc.flow_layout[1].to_side, Some(bpmn::Side::Left));
    let conflict = format!(
        "{source}\nlayout {{\nflow[Complete->Approve] : from=top\nflow[Complete->RequestDocuments] : from=top\n}}"
    );
    assert!(bpmn::validate_geometry(&bpmn::parse(&conflict).unwrap()).is_err());
    let unknown = format!("{source}\nlayout {{\nflow[Review->Approve] : from=top\n}}");
    assert!(bpmn::parse(&unknown).is_err());
}

#[test]
fn flow_label_orientation_can_be_auto_horizontal_or_vertical() {
    let source = include_str!("../examples/bpmn-order.laml");
    let custom = format!(
        "{source}\nlayout {{\nflow[Decision->Accept] : label-orientation=vertical\nflow[Decision->Decline] : label-orientation=horizontal\nflow[Deadline->Escalate] : label-orientation=horizontal\n}}"
    );
    let doc = bpmn::parse(&custom).unwrap();
    assert_eq!(
        doc.flow_layout[0].label_orientation,
        Some(bpmn::LabelOrientation::Vertical)
    );
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    let yes = svg
        .lines()
        .find(|line| line.ends_with(">yes</text>"))
        .unwrap();
    let no = svg
        .lines()
        .find(|line| line.ends_with(">no</text>"))
        .unwrap();
    let days = svg
        .lines()
        .find(|line| line.ends_with(">2 days</text>"))
        .unwrap();
    assert!(yes.contains("rotate(-90"));
    assert!(!no.contains("rotate("));
    assert!(!days.contains("rotate("));
    let automatic =
        format!("{source}\nlayout {{\nflow[Decision->Accept] : label-orientation=auto\n}}");
    assert_eq!(
        bpmn::parse(&automatic).unwrap().flow_layout[0].label_orientation,
        Some(bpmn::LabelOrientation::Auto)
    );
    let combined = format!(
        "{}\nlayout {{\nflow[Complete->Approve] : from=top; label-orientation=horizontal\n}}",
        include_str!("../examples/bpmn-basic.laml")
    );
    bpmn::validate_geometry(&bpmn::parse(&combined).unwrap()).unwrap();
    let bad =
        format!("{source}\nlayout {{\nflow[Decision->Accept] : label-orientation=diagonal\n}}");
    assert!(bpmn::parse(&bad).is_err());
    let unlabeled =
        format!("{source}\nlayout {{\nflow[Accept->Pack] : label-orientation=vertical\n}}");
    assert!(bpmn::parse(&unlabeled).is_err());
    let impossible = source.replace("Reply --> ShopDone", "Reply --> ShopDone : done");
    let impossible =
        format!("{impossible}\nlayout {{\nflow[Reply->ShopDone] : label-orientation=vertical\n}}");
    assert!(bpmn::validate_geometry(&bpmn::parse(&impossible).unwrap()).is_err());
}

#[test]
fn annotation_is_not_data_or_sequence_flow() {
    let source =
        "settings {\ntype: bpmn\n}\nsemantics {\nA --> B\nnote Info(\"Check again\")\nInfo .. A\n}";
    let doc = bpmn::parse(source).unwrap();
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    assert!(svg.contains("stroke-dasharray=\"4 4\""));
    assert!(bpmn::parse(&source.replace("Info .. A", "Info ..> A")).is_err());
    assert!(bpmn::parse(&source.replace("Info .. A", "Info --> A")).is_err());
}

#[test]
fn no_icon_needs_no_phosphor_notice() {
    let source = "settings {\ntype: bpmn\n}\nsemantics {\nstartEvent Start\nStart --> Plain\nPlain --> endEvent End\n}";
    let doc = bpmn::parse(source).unwrap();
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    assert!(!svg.contains("phosphor-license"));
}

#[test]
fn icon_catalogue_cli_lists_and_filters_names() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_laml"))
        .args(["icons", "clock"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    assert!(names.lines().any(|name| name == "clock"));
    assert!(names.lines().all(|name| name.contains("clock")));
}

#[test]
fn full_phosphor_regular_catalogue_is_selectable() {
    let names = include_str!("../assets/phosphor/icons.txt");
    assert_eq!(names.lines().count(), 1512);
    for icon in ["clock", "magnifying-glass", "airplane", "yin-yang"] {
        assert!(names.lines().any(|name| name == icon));
        let source =
            format!("settings {{\ntype: bpmn\n}}\nsemantics {{\nA(icon=\"{icon}\") --> B\n}}");
        assert!(bpmn::parse(&source).is_ok(), "{icon}");
    }
}
