use std::{fs, path::Path};

const GUIDES: &[&str] = &[
    "gne",
    "class",
    "object",
    "component",
    "deployment",
    "state",
    "activity",
    "usecase",
    "package",
    "sequence",
    "communication",
    "interaction-overview",
    "timing",
    "composite",
    "profile",
    "bpmn",
];

fn links(markdown: &str) -> impl Iterator<Item = &str> {
    markdown
        .split("](")
        .skip(1)
        .filter_map(|tail| tail.split(')').next())
}

fn source_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect()
}

fn heading_anchor(heading: &str) -> String {
    heading
        .trim_start_matches('#')
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[test]
fn entry_points_link_to_all_diagram_guides() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(root.join("README.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let started = fs::read_to_string(root.join("docs/getting-started.md"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(readme.contains("docs/getting-started.md"));
    let contents = readme
        .split_once("## Contents\n")
        .unwrap()
        .1
        .split_once("## LAML - Diagrams for People")
        .unwrap()
        .0;
    for heading in [
        "What is LAML?",
        "Why LAML?",
        "Installing & getting started",
        "Basic examples",
        "Themes",
        "Features",
        "AI agents via MCP",
        "License",
    ] {
        assert!(
            contents.contains(&format!("(#{})", heading_anchor(heading))),
            "missing TOC link to {heading}"
        );
        assert!(readme.contains(&format!("## {heading}\n")));
    }
    for tool in [
        "laml_types",
        "laml_guide",
        "laml_example",
        "laml_relations",
        "laml_check",
        "laml_build",
    ] {
        assert!(readme.contains(tool), "README must describe {tool}");
    }
    assert!(readme.contains("docs/mcp.md"));
    let features = readme
        .split_once("## Features")
        .unwrap()
        .1
        .split_once("## License")
        .unwrap()
        .0;
    assert!(readme.starts_with("# LAML\n\nLayout and Markup Language\n"));
    for name in ["generic", "uml", "bpmn", "c4"] {
        for suffix in ["", "-dark"] {
            let filename = format!("docs/images/badge-{name}{suffix}.svg");
            assert!(readme.contains(&filename));
            let badge = fs::read_to_string(root.join(filename)).unwrap();
            assert!(
                badge.contains("M0 0h12v40H0z"),
                "{name}{suffix}: inset accent"
            );
        }
    }
    assert_eq!(features.matches("- [").count(), GUIDES.len() + 5);
    for heading in ["Generic", "UML", "C4", "BPMN"] {
        assert!(features.contains(&format!("### {heading}\n")));
    }
    for level in ["system-context", "container", "component", "code"] {
        assert!(features.contains(&format!("docs/c4-diagrams.md#{level}")));
    }
    assert!(features.contains("docs/bpmn-diagrams.md#first-process"));
    assert!(features.contains("docs/bpmn-diagrams.md#collaboration-lanes-and-messages"));
    assert!(started.contains("c4-diagrams.md"));
    for name in GUIDES {
        assert!(
            started.contains(&format!("diagrams/{name}.md")),
            "getting started must link to {name}"
        );
        if *name != "bpmn" {
            assert!(
                features.contains(&format!("docs/diagrams/{name}.md")),
                "README features must link to {name}"
            );
        }
    }
}

#[test]
fn documentation_avoids_release_status_and_absence_pitch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = vec![root.join("README.md")];
    let mut directories = vec![root.join("docs")];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                pages.push(path);
            }
        }
    }
    for page in pages {
        let text = fs::read_to_string(&page).unwrap().to_lowercase();
        for phrase in [
            "laml account",
            "not yet",
            "deferred",
            "latest tagged binary",
            "future work",
            "not implemented",
            "remaining release gates",
        ] {
            assert!(!text.contains(phrase), "{}: {phrase}", page.display());
        }
    }
}

#[test]
fn user_guide_links_resolve_locally() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = vec![
        root.join("README.md"),
        root.join("docs/getting-started.md"),
        root.join("docs/c4-diagrams.md"),
    ];
    pages.extend(
        GUIDES
            .iter()
            .map(|name| root.join(format!("docs/diagrams/{name}.md"))),
    );
    for page in pages {
        let content = fs::read_to_string(&page).unwrap();
        for link in links(&content) {
            if link.starts_with("https://") || link.starts_with("http://") || link.starts_with('#')
            {
                continue;
            }
            let (target, anchor) = link.split_once('#').unwrap_or((link, ""));
            let target = page
                .parent()
                .unwrap()
                .join(target.split('?').next().unwrap());
            assert!(
                target.is_file(),
                "{}: missing target {link}",
                page.display()
            );
            if !anchor.is_empty() {
                let content = fs::read_to_string(&target).unwrap();
                assert!(
                    content
                        .lines()
                        .filter(|line| line.starts_with('#'))
                        .any(|line| heading_anchor(line) == anchor),
                    "{}: missing heading {link}",
                    page.display()
                );
            }
        }
    }
}

#[test]
fn every_diagram_guide_starts_with_code_image_and_complete_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in GUIDES {
        let guide = fs::read_to_string(root.join(format!("docs/diagrams/{name}.md"))).unwrap();
        let basic = guide
            .find("## Basic example")
            .expect("missing basic example");
        let code = guide[basic..].find("```laml").expect("missing basic code") + basic;
        let image = guide[basic..]
            .find("<picture>")
            .expect("missing responsive basic image")
            + basic;
        assert!(code < image, "{name}: code must precede image");
        let snippet = guide[code + "```laml".len()..].split("```").next().unwrap();
        let first_image = guide[image..]
            .split_once("<img src=\"")
            .and_then(|(_, tail)| tail.split('"').next())
            .unwrap();
        let filename = Path::new(first_image.split('?').next().unwrap())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix("-modern")
            .unwrap();
        let source = fs::read_to_string(root.join(format!("examples/{filename}.laml"))).unwrap();
        assert_eq!(
            source_lines(snippet),
            source_lines(&source),
            "{name}: basic code must build the shown image"
        );
        for picture in guide.split("<picture>").skip(1) {
            let block = picture
                .split("</picture>")
                .next()
                .expect("missing picture end");
            let light = block
                .split_once("<img src=\"")
                .and_then(|(_, tail)| tail.split('"').next())
                .expect("missing light image");
            let dark = block
                .split_once("srcset=\"")
                .and_then(|(_, tail)| tail.split('"').next())
                .expect("missing dark image");
            assert_eq!(dark, light.replace("-modern.png", "-modern-dark.png"));
            for path in [light, dark] {
                let file = path.split('?').next().unwrap();
                assert!(
                    root.join("docs/diagrams").join(file).is_file(),
                    "{name}: missing {path}"
                );
            }
            let filename = Path::new(light.split('?').next().unwrap())
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .strip_suffix("-modern")
                .unwrap();
            assert!(
                guide.contains(&format!("../../examples/{filename}.laml")),
                "{name}: {light} needs a link to its full source"
            );
        }
    }
}
