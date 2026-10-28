//! CLI for validation and SVG, PNG, and PDF builds.

use laml::geometry::place;
use laml::layout::validate_ordering;
use laml::parser::parse;
use laml::route::route;
use laml::styles::load;
use laml::svg::{TextMode, render_with_mode};
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
    "usage: laml check FILE.laml | laml build FILE.laml [OUTPUT.svg|OUTPUT.png|OUTPUT.pdf] [--svg-text=embedded|system|paths]".into()
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
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
    let path = Path::new(&args[1]);
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let diagram =
        parse(&text).map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
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
    if text_arg.is_some() && format != "svg" {
        return Err("--svg-text is only available for SVG output".into());
    }
    let boxes = place(&diagram, &styles)
        .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
    let routes = route(&diagram, &boxes, &styles)
        .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
    let svg = render_with_mode(&boxes, &routes, mode)
        .map_err(|e| format!("{}: SVG rendering failed: {e}", path.display()))?;
    let content = match format {
        "svg" => svg.into_bytes(),
        "png" => laml::export::png(&svg).map_err(|e| format!("{}: {e}", path.display()))?,
        "pdf" => laml::export::pdf(&svg).map_err(|e| format!("{}: {e}", path.display()))?,
        _ => unreachable!(),
    };
    // Do not overwrite an existing image until every validation phase succeeds.
    let temporary = output.with_extension(format!("{format}.{}.tmp", std::process::id()));
    fs::write(&temporary, content).map_err(|e| format!("{}: {e}", temporary.display()))?;
    if let Err(e) = fs::rename(&temporary, &output) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("{}: {e}", output.display()));
    }
    println!("wrote {}", output.display());
    Ok(())
}
