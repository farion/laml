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
