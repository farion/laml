//! CLI for validation and SVG, PNG, and PDF builds.

mod mcp;

use laml::geometry::place;
use laml::layout::validate_ordering;
use laml::parser::parse;
use laml::route::route;
use laml::styles::load;
use laml::svg::{TextMode, render_with_theme};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn usage() -> String {
    "usage: laml [--version | update | mcp | icons [FILTER] | import SOURCE [OUTPUT] [--from=mermaid|plantuml|structurizr-dsl|structurizr-json] [--force] | check FILE.laml | build FILE.laml [OUTPUT.svg|OUTPUT.png|OUTPUT.pdf] [--svg-text=embedded|system|paths]]".into()
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [command] if command == "mcp" => return mcp::run(),
        [command, ..] if command == "import" => return laml::import::cli::run(&args[1..]),
        [version] if version == "--version" => {
            println!("laml {}", laml::update::VERSION);
            return Ok(());
        }
        [command] if command == "update" => {
            println!("{}", laml::update::update()?);
            return Ok(());
        }
        [command] | [command, _] if command == "icons" => {
            let filter = args.get(1).map_or("", String::as_str);
            for icon in include_str!("../assets/phosphor/icons.txt")
                .lines()
                .filter(|i| i.contains(filter))
            {
                println!("{icon}");
            }
            return Ok(());
        }
        [command] if command == "__update-check" => {
            laml::update::refresh_cache();
            return Ok(());
        }
        _ => {}
    }
    if args.len() < 2
        || args.len() > 4
        || !matches!(args[0].as_str(), "check" | "build")
        || (args[0] == "check" && args.len() != 2)
    {
        return Err(usage());
    }
    let text_arg = args.last().and_then(|arg| arg.strip_prefix("--svg-text="));
    let mode = match text_arg {
        None | Some("embedded") => TextMode::Embedded,
        Some("system") => TextMode::System,
        Some("paths") => TextMode::Paths,
        _ => return Err(usage()),
    };
    let output_arg = if text_arg.is_some() {
        args.get(2).filter(|_| args.len() == 4)
    } else {
        args.get(2)
    };
    laml::update::notify_in_background();
    let path = Path::new(&args[1]);
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (special_text, theme, transparent, font_file, font_italic_file) =
        laml::theme::preprocess_settings(&text).map_err(|e| format!("{}:{e}", path.display()))?;
    laml::text::with_fonts(
        path,
        theme.modern(),
        font_file.as_deref(),
        font_italic_file.as_deref(),
        || {
            compile_document(
                &args,
                path,
                &text,
                &special_text,
                theme,
                transparent,
                mode,
                output_arg,
                text_arg.is_some(),
            )
        },
    )?
}

#[allow(clippy::too_many_arguments)]
fn compile_document(
    args: &[String],
    path: &Path,
    text: &str,
    special_text: &str,
    theme: laml::theme::Theme,
    transparent: bool,
    mode: TextMode,
    output_arg: Option<&String>,
    svg_text_option: bool,
) -> Result<(), String> {
    let render_mode = if mode == TextMode::Paths {
        TextMode::Embedded
    } else {
        mode
    };
    if laml::bpmn::is_bpmn_source(text) {
        let document = laml::bpmn::parse(text).map_err(|e| format!("{}:{e}", path.display()))?;
        let styles = load(
            &laml::syntax::Diagram {
                settings: laml::syntax::Settings {
                    theme,
                    transparent,
                    ..Default::default()
                },
                styling: document.styling.clone(),
                ..Default::default()
            },
            path,
        )
        .map_err(|e| format!("{}:{}: {}", e.path.display(), e.line, e.message))?;
        laml::bpmn::validate_styles(&document, &styles)?;
        laml::bpmn::validate_geometry(&document)?;
        if args[0] == "check" {
            println!(
                "parsed BPMN: {} nodes, {} flows; geometry validated",
                document.nodes.len(),
                document.edges.len()
            );
            return Ok(());
        }
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            return Err("build input must have a .laml extension".into());
        }
        let output = output_arg
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("svg"));
        let format = output
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("build output must end in .svg, .png or .pdf")?;
        if !matches!(format, "svg" | "png" | "pdf") {
            return Err("build output must end in .svg, .png or .pdf".into());
        }
        if svg_text_option && format != "svg" {
            return Err("--svg-text is only available for SVG output".into());
        }
        let svg = laml::bpmn::render_with_styles(&document, &styles, render_mode)
            .map_err(|e| format!("{}: BPMN rendering failed: {e}", path.display()))?;
        return write_image(&svg, &output, format, theme, transparent, mode, &styles);
    }
    if laml::communication::is_communication_source(text) {
        let mut document = laml::communication::parse(special_text)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        document.graph.settings.theme = theme;
        document.graph.settings.transparent = transparent;
        let styles = load(&document.graph, path)
            .map_err(|e| format!("{}:{}: {}", e.path.display(), e.line, e.message))?;
        validate_ordering(&document.graph)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        if args[0] == "check" {
            println!(
                "parsed communication: {} lifelines, {} links, {} messages; full geometry not checked",
                document.graph.nodes.len(),
                document.graph.relations.len(),
                document.messages.len()
            );
            return Ok(());
        }
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            return Err("build input must have a .laml extension".into());
        }
        let output = output_arg
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("svg"));
        let format = output
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("build output must end in .svg, .png or .pdf")?;
        if !matches!(format, "svg" | "png" | "pdf") {
            return Err("build output must end in .svg, .png or .pdf".into());
        }
        if svg_text_option && format != "svg" {
            return Err("--svg-text is only available for SVG output".into());
        }
        let boxes = place(&document.graph, &styles)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        let routes = route(&document.graph, &boxes, &styles)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        let svg = laml::communication::render(&document, &boxes, &routes, render_mode)
            .map_err(|e| format!("{}: communication rendering failed: {e}", path.display()))?;
        return write_image(&svg, &output, format, theme, transparent, mode, &styles);
    }
    if laml::timing::is_timing_source(text) {
        let timing = laml::timing::parse(text)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        let style_input = laml::syntax::Diagram {
            settings: laml::syntax::Settings {
                theme,
                transparent,
                ..Default::default()
            },
            styling: timing.styling.clone(),
            ..Default::default()
        };
        let styles = load(&style_input, path)
            .map_err(|e| format!("{}:{}: {}", e.path.display(), e.line, e.message))?;
        if args[0] == "check" {
            // Geometry and LSS visibility are checked at render time, as for other image types.
            println!(
                "parsed timing: {} lifelines, {} state changes; full geometry not checked",
                timing.lifelines.len(),
                timing
                    .lifelines
                    .iter()
                    .map(|p| p.changes.len())
                    .sum::<usize>()
            );
            return Ok(());
        }
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            return Err("build input must have a .laml extension".into());
        }
        let output = output_arg
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("svg"));
        let format = output
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("build output must end in .svg, .png or .pdf")?;
        if !matches!(format, "svg" | "png" | "pdf") {
            return Err("build output must end in .svg, .png or .pdf".into());
        }
        if svg_text_option && format != "svg" {
            return Err("--svg-text is only available for SVG output".into());
        }
        let svg = laml::timing::render(&timing, &styles, render_mode)
            .map_err(|e| format!("{}: timing rendering failed: {e}", path.display()))?;
        return write_image(&svg, &output, format, theme, transparent, mode, &styles);
    }
    if laml::sequence::is_sequence_source(text) {
        let sequence = laml::sequence::parse(text)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        let style_input = laml::syntax::Diagram {
            settings: laml::syntax::Settings {
                theme,
                transparent,
                ..Default::default()
            },
            styling: sequence.styling.clone(),
            ..Default::default()
        };
        let styles = load(&style_input, path)
            .map_err(|e| format!("{}:{}: {}", e.path.display(), e.line, e.message))?;
        if args[0] == "check" {
            println!(
                "parsed sequence: {} lifelines, {} events; full geometry not checked",
                sequence.participants.len(),
                sequence.events.len()
            );
            return Ok(());
        }
        if path.extension().and_then(|e| e.to_str()) != Some("laml") {
            return Err("build input must have a .laml extension".into());
        }
        let output = output_arg
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("svg"));
        let format = output
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("build output must end in .svg, .png or .pdf")?;
        if !matches!(format, "svg" | "png" | "pdf") {
            return Err("build output must end in .svg, .png or .pdf".into());
        }
        if svg_text_option && format != "svg" {
            return Err("--svg-text is only available for SVG output".into());
        }
        let svg = laml::sequence::render::render_with_styles(&sequence, &styles, render_mode)
            .map_err(|e| format!("{}: sequence rendering failed: {e}", path.display()))?;
        return write_image(&svg, &output, format, theme, transparent, mode, &styles);
    }
    let overview = if laml::interaction_overview::is_overview_source(text) {
        let mut doc = laml::interaction_overview::parse(special_text)
            .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
        doc.graph.settings.theme = theme;
        doc.graph.settings.transparent = transparent;
        Some(doc)
    } else {
        None
    };
    let diagram = match &overview {
        Some(doc) => doc.graph.clone(),
        None => parse(text).map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?,
    };
    let styles = load(&diagram, path)
        .map_err(|e| format!("{}:{}: {}", e.path.display(), e.line, e.message))?;
    validate_ordering(&diagram)
        .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
    if args[0] == "check" {
        println!(
            "parsed {} nodes, {} relations, {} layout rules, {} style rules; directional cycles checked; full geometry not checked",
            diagram.nodes.len(),
            diagram.relations.len(),
            diagram.layout.len(),
            styles.rules.len()
        );
        return Ok(());
    }
    if path.extension().and_then(|e| e.to_str()) != Some("laml") {
        return Err("build input must have a .laml extension".into());
    }
    let output = output_arg
        .map(PathBuf::from)
        .unwrap_or_else(|| path.with_extension("svg"));
    let format = output
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("build output must end in .svg, .png or .pdf")?;
    if !matches!(format, "svg" | "png" | "pdf") {
        return Err("build output must end in .svg, .png or .pdf".into());
    }
    if svg_text_option && format != "svg" {
        return Err("--svg-text is only available for SVG output".into());
    }
    let boxes = place(&diagram, &styles)
        .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
    let routes = route(&diagram, &boxes, &styles)
        .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
    let svg = if let Some(doc) = &overview {
        laml::interaction_overview::render(doc, &boxes, &routes, render_mode)
    } else if laml::c4::is_c4_source(text) {
        laml::svg::c4::render(&boxes, &routes, render_mode, theme, &diagram)
    } else {
        render_with_theme(&boxes, &routes, render_mode, theme)
    }
    .map_err(|e| format!("{}: SVG rendering failed: {e}", path.display()))?;
    write_image(&svg, &output, format, theme, transparent, mode, &styles)
}

fn write_image(
    svg: &str,
    output: &Path,
    format: &str,
    theme: laml::theme::Theme,
    transparent: bool,
    mode: TextMode,
    styles: &laml::styles::StyleSheet,
) -> Result<(), String> {
    let protected: Vec<String> = styles
        .rules
        .iter()
        .flat_map(|rule| rule.declarations.iter())
        .filter_map(|decl| match &decl.value {
            laml::styles::Value::Color(c) => Some(c.clone()),
            _ => None,
        })
        .collect();
    let svg = laml::theme::tint_svg_defaults(svg, theme, &protected);
    let svg = laml::text::svg_fonts(&svg)?;
    let svg = laml::theme::canvas(&svg, theme, transparent)?;
    let svg = if mode == TextMode::Paths {
        laml::export::outlines(&svg)?
    } else {
        svg
    };
    let svg = svg.as_str();
    let content = match format {
        "svg" => svg.as_bytes().to_vec(),
        "png" => laml::export::png(svg).map_err(|e| format!("{}: {e}", output.display()))?,
        "pdf" => laml::export::pdf(svg).map_err(|e| format!("{}: {e}", output.display()))?,
        _ => return Err("unsupported output format".into()),
    };
    // Do not overwrite an existing image until every validation phase succeeds.
    let temporary = output.with_extension(format!("{format}.{}.tmp", std::process::id()));
    fs::write(&temporary, content).map_err(|e| format!("{}: {e}", temporary.display()))?;
    if let Err(e) = fs::rename(&temporary, output) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("{}: {e}", output.display()));
    }
    println!("wrote {}", output.display());
    Ok(())
}
