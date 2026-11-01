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
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let started = fs::read_to_string(root.join("docs/getting-started.md")).unwrap();
    assert!(readme.contains("docs/getting-started.md"));
    let features = readme
        .split_once("## Features")
        .unwrap()
        .1
        .split_once("## License")
        .unwrap()
        .0;
    assert_eq!(features.matches("  - [").count(), GUIDES.len());
    for name in GUIDES {
        assert!(
            started.contains(&format!("diagrams/{name}.md")),
            "getting started must link to {name}"
        );
        assert!(
            features.contains(&format!("docs/diagrams/{name}.md")),
            "README features must link to {name}"
        );
    }
}

#[test]
fn user_guide_links_resolve_locally() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = vec![root.join("README.md"), root.join("docs/getting-started.md")];
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
            let target = page.parent().unwrap().join(target);
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
        let image = guide[basic..].find("![").expect("missing basic image") + basic;
        assert!(code < image, "{name}: code must precede image");
        let snippet = guide[code + "```laml".len()..].split("```").next().unwrap();
        let first_image = guide[image..]
            .split_once("](")
            .and_then(|(_, tail)| tail.split(')').next())
            .unwrap();
        let filename = Path::new(first_image)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let source = fs::read_to_string(root.join(format!("examples/{filename}.laml"))).unwrap();
        assert_eq!(
            source_lines(snippet),
            source_lines(&source),
            "{name}: basic code must build the shown image"
        );
        for image_link in guide.split("![").skip(1) {
            let path = image_link
                .split_once("](")
                .and_then(|(_, tail)| tail.split(')').next())
                .expect("malformed image");
            let filename = Path::new(path).file_stem().unwrap().to_str().unwrap();
            assert!(
                guide.contains(&format!("../../examples/{filename}.laml")),
                "{name}: {path} needs a link to its full source"
            );
        }
    }
}
