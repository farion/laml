use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "laml-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn check_example_accepts_syntax_and_imported_styles() {
    let output = Command::new(env!("CARGO_BIN_EXE_laml"))
        .arg("check")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/architecture.laml"
        ))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("3 nodes, 2 relations, 2 layout rules, 3 style rules"));
    assert!(text.contains("full geometry not checked"));
}

#[test]
fn build_example_produces_svg_with_selectable_text_and_embedded_font() {
    let dir = temp_dir();
    let destination = dir.join("architecture.svg");
    let output = Command::new(env!("CARGO_BIN_EXE_laml"))
        .arg("build")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/architecture.laml"
        ))
        .arg(&destination)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = fs::read_to_string(&destination).unwrap();
    assert!(svg.contains("<svg "));
    assert!(svg.contains("data:font/otf;base64,"));
    assert_eq!(svg.matches("<polyline ").count(), 2);
    assert_eq!(svg.matches("<rect ").count(), 3);
    assert!(svg.contains("<text "));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn parallel_routes_render_as_distinct_paths() {
    let dir = temp_dir();
    let source = dir.join("parallel.laml");
    let destination = dir.join("parallel.svg");
    fs::write(&source, "semantics {\nA\nB\nA -> B\nA -> B\n}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_laml"))
        .args([
            "build",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = fs::read_to_string(&destination).unwrap();
    let lines: Vec<&str> = svg
        .lines()
        .filter(|line| line.starts_with("<polyline"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_ne!(lines[0], lines[1]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn impossible_requested_label_mode_fails_at_layout_line_without_image() {
    let dir = temp_dir();
    let source = dir.join("label-failure.laml");
    let output = dir.join("label-failure.svg");
    let label =
        "This self loop label is deliberately much too long for any available exterior segment";
    fs::write(&source,format!("semantics {{\nA\nloop: A -> A : {label}\n}}\nlayout {{\nrelation[loop] : label=along\n}}\n")).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .args(["build", source.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains(".laml:6:"));
    assert!(!output.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn fixed_width_truncates_text_but_preserves_full_svg_title() {
    let dir = temp_dir();
    let source = dir.join("narrow.laml");
    let output = dir.join("narrow.svg");
    fs::write(&source,"semantics {\nBackend(label=\"Long Backend Node Label\")\n}\nstyling {\nnode[Backend] {\nwidth: 80\n}\n}\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .args(["build", source.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains("<title>Long Backend Node Label</title>"));
    assert!(svg.contains('…'));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_exports_png_pdf_and_both_other_svg_text_modes() {
    let dir = temp_dir();
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/architecture.laml");
    for extension in ["png", "pdf", "svg"] {
        let path = dir.join(format!("architecture.{extension}"));
        let result = Command::new(env!("CARGO_BIN_EXE_laml"))
            .args(["build", source, path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = fs::read(&path).unwrap();
        match extension {
            "png" => assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n")),
            "pdf" => assert!(bytes.starts_with(b"%PDF-")),
            _ => assert!(bytes.starts_with(b"<svg ")),
        }
    }
    for mode in ["system", "paths"] {
        let output = dir.join(format!("{mode}.svg"));
        let result = Command::new(env!("CARGO_BIN_EXE_laml"))
            .args([
                "build",
                source,
                output.to_str().unwrap(),
                &format!("--svg-text={mode}"),
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = fs::read_to_string(output).unwrap();
        assert!(!text.contains("data:font/"));
        assert_eq!(text.contains("<text"), mode == "system");
    }
    let output = dir.join("unsupported.png");
    fs::write(&output, b"unchanged").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .args([
            "build",
            source,
            output.to_str().unwrap(),
            "--svg-text=paths",
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read(&output).unwrap(), b"unchanged");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn png_canvas_limit_fails_without_overwriting_existing_image() {
    let dir = temp_dir();
    let source = dir.join("oversize.laml");
    let destination = dir.join("oversize.png");
    fs::write(
        &source,
        "semantics {\nA\n}\nstyling {\nnode[A] {\nwidth: 10000\nheight: 10000\n}\n}\n",
    )
    .unwrap();
    fs::write(&destination, b"keep existing image").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_laml"))
        .args([
            "build",
            source.to_str().unwrap(),
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("64 million pixels"));
    assert_eq!(fs::read(destination).unwrap(), b"keep existing image");
    fs::remove_dir_all(dir).unwrap();
}
