//! CLI boundary: fail on unsupported syntax before writing, protect existing files.
use super::{Format, convert};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: &[String]) -> Result<(), String> {
    let mut positional = Vec::new();
    let mut format = None;
    let mut force = false;
    for arg in args {
        if let Some(value) = arg.strip_prefix("--from=") {
            if format.replace(Format::parse(value)?).is_some() {
                return Err("--from specified twice".into());
            }
        } else if arg == "--force" {
            force = true;
        } else if arg.starts_with('-') {
            return Err(format!("unknown import option '{arg}'"));
        } else {
            positional.push(arg.as_str());
        }
    }
    if positional.is_empty() || positional.len() > 2 {
        return Err("usage: laml import SOURCE [OUTPUT] [--from=mermaid|plantuml|structurizr-dsl|structurizr-json] [--force]".into());
    }
    let path = Path::new(positional[0]);
    let format = format.map_or_else(|| Format::detect(path), Ok)?;
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let results = convert(&source, format).map_err(|e| format!("{}: {e}", path.display()))?;
    for item in &results {
        let outcome = if item.source.contains("type: c4-") {
            laml_c4(&item.source)
        } else if item.source.contains("type: sequence") {
            crate::sequence::parse(&item.source)
                .map(|_| ())
                .map_err(|e| format!("{e:?}"))
        } else {
            crate::parser::parse(&item.source)
                .map(|_| ())
                .map_err(|e| format!("{e:?}"))
        };
        outcome.map_err(|e| format!("{}: generated view '{}': {e}", path.display(), item.name))?;
    }
    let destinations: Vec<PathBuf> = if results.len() == 1 {
        let destination = positional
            .get(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("laml"));
        if destination.extension().and_then(|s| s.to_str()) != Some("laml") {
            return Err("single-view output must end in .laml".into());
        }
        vec![destination]
    } else {
        let directory = positional.get(1).map(PathBuf::from).unwrap_or_else(|| {
            path.with_file_name(format!(
                "{}-laml",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ))
        });
        if directory.extension().and_then(|s| s.to_str()) == Some("laml") {
            return Err("multi-view output must be a directory".into());
        }
        results
            .iter()
            .map(|item| directory.join(format!("{}.laml", item.name)))
            .collect()
    };
    for file in &destinations {
        if !force && file.exists() {
            return Err(format!(
                "{} exists; use --force to replace it",
                file.display()
            ));
        }
    }
    for (item, file) in results.iter().zip(destinations) {
        if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&file, &item.source).map_err(|e| format!("{}: {e}", file.display()))?;
        println!("wrote {}", file.display());
    }
    Ok(())
}
fn laml_c4(text: &str) -> Result<(), String> {
    crate::c4::parse(text)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}
