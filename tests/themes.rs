use laml::{geometry::place, parser::parse, styles::load, theme::Theme};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
fn workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "laml-theme-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
fn build(dir: &Path, source: &str, extension: &str) -> Result<Vec<u8>, String> {
    let input = dir.join("diagram.laml");
    let output = dir.join(format!("diagram.{extension}"));
    fs::write(&input, source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .env("LAML_NO_UPDATE_CHECK", "1")
        .arg("build")
        .arg(&input)
        .arg(&output)
        .output()
        .unwrap();
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into());
    }
    Ok(fs::read(output).unwrap())
}
fn diagram(theme: &str, background: &str) -> String {
    format!(
        "settings {{\ntheme: {theme}\nbackground: {background}\n}}\nsemantics {{\nA\nB\nA -> B : message\n}}\n"
    )
}

#[test]
fn all_six_themes_export_and_choose_the_correct_canvas() {
    let dir = workspace();
    for name in [
        "plain",
        "modern",
        "modern-mono",
        "plain-dark",
        "modern-dark",
        "modern-mono-dark",
    ] {
        let png = build(&dir, &diagram(name, "opaque"), "png").unwrap();
        let pix = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        let corner = pix.pixels()[0];
        assert_eq!(corner.alpha(), 255, "{name} must be opaque");
        let expected = if name.ends_with("dark") { 0 } else { 255 };
        assert_eq!(
            (corner.red(), corner.green(), corner.blue()),
            (expected, expected, expected),
            "{name}"
        );
        let last = pix.pixels().last().unwrap();
        assert_eq!(
            (last.red(), last.green(), last.blue()),
            (expected, expected, expected),
            "{name}: bottom-right canvas fringe"
        );
        let svg = String::from_utf8(build(&dir, &diagram(name, "opaque"), "svg").unwrap()).unwrap();
        let face = if Theme::parse(name).unwrap().modern() {
            "Geist"
        } else {
            "Inter"
        };
        assert!(svg.contains(&format!("font-family=\"{face}\"")), "{name}");
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mono_theme_defaults_are_grayscale_across_diagram_types() {
    let dir = workspace();
    for source in [
        include_str!("../examples/theme-modern.laml"),
        include_str!("../examples/bpmn-basic.laml"),
        include_str!("../examples/sequence-basic.laml"),
        include_str!("../examples/timing-basic.laml"),
    ] {
        let source = source
            .replace("\r\n", "\n")
            .replacen("settings {", "settings {\ntheme: modern-mono", 1)
            .replace("theme: modern\n", "");
        let svg = String::from_utf8(build(&dir, &source, "svg").unwrap()).unwrap();
        for attribute in ["fill=\"#", "stroke=\"#"] {
            for color in svg
                .split(attribute)
                .skip(1)
                .filter_map(|tail| tail.get(..6))
            {
                let channel = |offset| &color[offset..offset + 2];
                assert!(
                    channel(0) == channel(2) && channel(2) == channel(4),
                    "chromatic theme default #{color} in {attribute}"
                );
            }
        }
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn transparent_dark_preserves_alpha_in_png_and_svg() {
    let dir = workspace();
    let source = diagram("modern-mono-dark", "transparent");
    let svg = String::from_utf8(build(&dir, &source, "svg").unwrap()).unwrap();
    assert!(svg.contains("data-laml-transparent=\"true\""));
    assert!(!svg.contains("fill=\"#000000\""));
    let png = build(&dir, &source, "png").unwrap();
    let pix = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!(pix.pixels()[0].alpha(), 0);
    assert!(pix.pixels().iter().any(|p| p.alpha() == 255));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn base_color_derives_paints_but_individual_override_wins() {
    let one = "semantics {\nA\n}\nstyling {\nnode[A] {\nbase-color: \"#00ff00\"\nfill: \"#ffffff\"\n}\n}\n";
    let two = one.replace("#00ff00", "#ff0000");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/architecture.laml");
    let paints = |source: &str| {
        let graph = parse(source).unwrap();
        let sheet = load(&graph, &root).unwrap();
        place(&graph, &sheet).unwrap().remove(0)
    };
    let green = paints(one);
    let red = paints(&two);
    assert_eq!(green.fill, "#ffffff");
    assert_eq!(green.fill, red.fill);
    assert_ne!(green.border_color, red.border_color);
    assert_ne!(green.color, red.color);
    let mono = paints(&one.replacen(
        "semantics {",
        "settings {\ntheme: modern-mono\n}\nsemantics {",
        1,
    ));
    // A deliberately chosen base color remains colored in a mono theme.
    assert_ne!(&mono.border_color[1..3], &mono.border_color[3..5]);
}

#[test]
fn generic_note_shape_gets_a_warm_default_without_inspecting_its_name() {
    let source = "semantics {\nA\nB\n}\nstyling {\nnode[B] {\nshape: note\n}\n}\n";
    let graph = parse(source).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/gne-basic.laml");
    let sheet = load(&graph, &root).unwrap();
    let nodes = place(&graph, &sheet).unwrap();
    assert_ne!(nodes[0].fill, nodes[1].fill);
    assert_ne!(nodes[0].accent_color, nodes[1].accent_color);
}

#[test]
fn custom_font_is_measured_and_embedded_in_every_format() {
    let dir = workspace();
    fs::write(
        dir.join("Inter.otf"),
        include_bytes!("../assets/Inter-Regular.otf"),
    )
    .unwrap();
    let source = "settings {\ntheme: modern\nfont-file: \"Inter.otf\"\n}\nsemantics {\nA\nB\nA -> B : message\n}\n";
    let svg = String::from_utf8(build(&dir, source, "svg").unwrap()).unwrap();
    assert!(svg.contains("font-family=\"Inter\""));
    assert!(!svg.contains("font-family=\"Geist\""));
    assert!(svg.contains("data:font/otf;base64,"));
    let png = build(&dir, source, "png").unwrap();
    let pixels = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert!(
        pixels
            .pixels()
            .iter()
            .filter(|p| p.red() != 255 || p.green() != 255 || p.blue() != 255)
            .count()
            > 100
    );
    let pdf = build(&dir, source, "pdf").unwrap();
    assert!(
        pdf.windows(b"Inter-Regular".len())
            .any(|w| w == b"Inter-Regular")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_fonts_do_not_silently_fallback_to_an_unrelated_italic_face() {
    let dir = workspace();
    fs::write(
        dir.join("Inter.otf"),
        include_bytes!("../assets/Inter-Regular.otf"),
    )
    .unwrap();
    let source = "settings {\ntype: class\nfont-file: \"Inter.otf\"\n}\nsemantics {\nabstract class Widget\n}\n";
    let error = build(&dir, source, "svg").unwrap_err();
    assert!(
        error.contains("selected font has no italic face"),
        "{error}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn specialized_parsers_accept_and_keep_theme_settings() {
    let bpmn = include_str!("../examples/bpmn-basic.laml").replacen(
        "settings {",
        "settings {\ntheme: plain-dark\nbackground: transparent",
        1,
    );
    assert_eq!(laml::bpmn::parse(&bpmn).unwrap().theme, Theme::PlainDark);
    let sequence = include_str!("../examples/sequence-basic.laml").replacen(
        "settings {",
        "settings {\ntheme: modern-mono-dark",
        1,
    );
    assert_eq!(
        laml::sequence::parse(&sequence).unwrap().theme,
        Theme::ModernMonoDark
    );
    let timing = include_str!("../examples/timing-basic.laml").replacen(
        "settings {",
        "settings {\ntheme: modern-dark",
        1,
    );
    assert_eq!(
        laml::timing::parse(&timing).unwrap().theme,
        Theme::ModernDark
    );
}

#[test]
fn outlined_svg_preserves_icon_attribution() {
    let dir = workspace();
    let input = dir.join("icon.laml");
    let output = dir.join("icon.svg");
    fs::write(
        &input,
        "semantics {\nA\n}\nstyling {\nnode[A] {\nicon: \"folder\"\n}\n}\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .env("LAML_NO_UPDATE_CHECK", "1")
        .arg("build")
        .arg(&input)
        .arg(&output)
        .arg("--svg-text=paths")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = fs::read_to_string(output).unwrap();
    assert!(!svg.contains("<text "));
    assert!(svg.contains("<metadata id=\"phosphor-license\">"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_font_or_illegal_theme_fails_without_output() {
    let dir = workspace();
    let err = build(
        &dir,
        "settings {\nfont-file: \"missing.otf\"\n}\nsemantics {\nA\n}\n",
        "svg",
    )
    .unwrap_err();
    assert!(err.contains("font file"), "{err}");
    let err = build(
        &dir,
        "settings {\ntheme: neon\n}\nsemantics {\nA\n}\n",
        "svg",
    )
    .unwrap_err();
    assert!(err.contains("invalid theme"), "{err}");
    assert!(!dir.join("diagram.svg").exists());
    fs::remove_dir_all(dir).unwrap();
}
