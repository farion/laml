use laml::export;
use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::load;
use laml::svg::{TextMode, render_with_mode};
use std::path::Path;

fn sample(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let diagram = parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let sheet = load(&diagram, &path).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap()
}

#[test]
fn all_examples_export_valid_png_and_vector_pdf() {
    for name in [
        "architecture.laml",
        "crossing.laml",
        "parallel-loops.laml",
        "label-modes.laml",
    ] {
        let svg = sample(name);
        let png = export::png(&svg).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "{name}");
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert!(pixmap.width() > 100 && pixmap.height() > 100, "{name}");
        assert!(
            pixmap
                .pixels()
                .iter()
                .any(|p| p.red() != 255 || p.green() != 255 || p.blue() != 255),
            "{name}"
        );
        let pdf = export::pdf(&svg).unwrap();
        assert!(pdf.starts_with(b"%PDF-"), "{name}");
        assert!(
            pdf.windows(b"Inter-Regular".len())
                .any(|w| w == b"Inter-Regular"),
            "{name}: PDF must embed bundled Inter"
        );
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"), "{name}");
        assert_eq!(
            pdf,
            export::pdf(&svg).unwrap(),
            "{name}: PDF output should be deterministic"
        );
        assert_eq!(
            png,
            export::png(&svg).unwrap(),
            "{name}: PNG output should be deterministic"
        );
    }
}

#[test]
fn svg_modes_have_distinct_font_and_text_contracts() {
    let diagram = parse(include_str!("../examples/architecture.laml")).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/architecture.laml");
    let sheet = load(&diagram, &path).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let embedded = render_with_mode(&boxes, &routes, TextMode::Embedded).unwrap();
    let system = render_with_mode(&boxes, &routes, TextMode::System).unwrap();
    let paths = render_with_mode(&boxes, &routes, TextMode::Paths).unwrap();
    assert!(embedded.contains("data:font/otf;base64,"));
    assert!(embedded.contains("<text "));
    assert!(system.contains("<text "));
    assert!(!system.contains("data:font/"));
    assert!(!paths.contains("<text"));
    assert!(!paths.contains("data:font/"));
    assert!(paths.contains("<path"));
    assert_eq!(
        paths,
        render_with_mode(&boxes, &routes, TextMode::Paths).unwrap()
    );
    assert!(export::png(&paths).is_ok());
}
