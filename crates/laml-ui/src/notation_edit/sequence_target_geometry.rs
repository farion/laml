//! Shared font/style context and compiler geometry, without final SVG export work.
use super::sequence_targets::{self, Action, Choice, Target};
use crate::backend::Presentation;
use std::path::Path;

pub(crate) fn build(
    source: &str,
    path: &Path,
    presentation: Presentation,
    action: &Action,
    targets: &[Target],
) -> Vec<Choice> {
    with_geometry(source, path, presentation, |geometry| {
        sequence_targets::build_with(source, action, targets, geometry)
    })
    .unwrap_or_default()
}
type Geometry = (
    laml::sequence::SequenceDiagram,
    Vec<laml::authoring::Region>,
);
pub(crate) fn with_geometry<R>(
    source: &str,
    path: &Path,
    presentation: Presentation,
    build: impl FnOnce(&mut dyn FnMut(&str) -> Option<Geometry>) -> R,
) -> Option<R> {
    let Ok((_, theme, transparent, font, italic)) = laml::theme::preprocess_settings(source) else {
        return None;
    };
    let theme = presentation.theme_override().unwrap_or(theme);
    let transparent = if presentation.theme_override().is_some() {
        true
    } else {
        transparent
    };
    let Ok(doc) = laml::sequence::parse(source) else {
        return None;
    };
    let mut diagram = laml::syntax::Diagram::default();
    diagram.settings.theme = theme;
    diagram.settings.transparent = transparent;
    diagram.styling = doc.styling;
    // Load assets once at the backend-owned (possibly sandbox-staged) source path.
    let Ok(styles) = laml::styles::load(&diagram, path) else {
        return None;
    };
    laml::text::with_fonts(
        path,
        theme.modern(),
        font.as_deref(),
        italic.as_deref(),
        || {
            build(&mut |text| {
                let mut doc = laml::sequence::parse(text).ok()?;
                doc.theme = theme;
                // This is the same geometry/routing/label validation as the final
                // preview, but skips font embedding, glyph outlining and SVG cleanup.
                let (_, regions) = laml::sequence::render::render_for_editor(
                    &doc,
                    &styles,
                    laml::svg::TextMode::System,
                )
                .ok()?;
                Some((doc, regions))
            })
        },
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notation_edit::sequence_controls::{Boundary, Dock};
    #[test]
    fn fast_preflight_matches_full_preview_validity_and_endpoint_geometry() {
        for body in [
            " work: B+ : \"Work\"\n old: A ->> B: first\n loop: B ->> B\n B-\n",
            " old: A -> B+: first\n C ->> A\n B-\n",
            " alt \"yes\" {\n old: A ->> B: first\n }\n else \"no\" {\n B ->> A: second\n }\n",
        ] {
            let source = format!(
                "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n C\n{body}}}\nstyling {{\n node[B] {{\n font-size: 19\n }}\n relation {{\n font-size: 16\n }}\n}}\n"
            );
            let path = Path::new("target-parity.laml");
            let preview = crate::preview::build(&source, path).unwrap();
            let line = source.lines().position(|s| s.contains("old:")).unwrap() + 1;
            let mut targets = preview
                .regions
                .iter()
                .filter(|r| r.kind == "sequence-slot")
                .flat_map(|r| {
                    ["A", "B", "C"].map(|owner| Target {
                        dock: Dock {
                            owner: owner.into(),
                            before: r.line,
                            boundary: None,
                        },
                        same_row: false,
                        straighten: false,
                    })
                })
                .collect::<Vec<_>>();
            for bar in preview
                .regions
                .iter()
                .filter(|r| r.kind == "sequence-execution")
            {
                targets.push(Target {
                    dock: Dock {
                        owner: bar.id.clone(),
                        before: bar.line,
                        boundary: Some(Boundary::Start(bar.line)),
                    },
                    same_row: false,
                    straighten: false,
                });
                targets.push(Target {
                    dock: Dock {
                        owner: bar.id.clone(),
                        before: bar.end_line.unwrap_or(0),
                        boundary: Some(Boundary::End {
                            start: bar.line,
                            end: bar.end_line,
                        }),
                    },
                    same_row: false,
                    straighten: false,
                });
            }
            for action in [
                Action::Create {
                    origin: Dock {
                        owner: "C".into(),
                        before: line,
                        boundary: None,
                    },
                    operator: "->".into(),
                },
                Action::Reconnect {
                    line,
                    source: false,
                },
                Action::Reconnect { line, source: true },
            ] {
                for presentation in [
                    Presentation::Document,
                    Presentation::EditorLight,
                    Presentation::Editor,
                ] {
                    let fast = build(&source, path, presentation, &action, &targets);
                    let full = sequence_targets::build(
                        crate::backend::local().as_ref(),
                        &source,
                        path,
                        presentation,
                        &action,
                        &targets,
                    );
                    assert_eq!(
                        serde_json::to_value(&fast).unwrap(),
                        serde_json::to_value(&full).unwrap(),
                        "Preflight must preserve the exact valid set, edits and coordinates"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod font_tests {
    use super::*;
    use crate::notation_edit::sequence_controls::Dock;
    #[test]
    fn target_batch_keeps_custom_font_measurement_and_rejects_escaped_fonts() {
        let dir = std::env::temp_dir().join(format!(
            "laml-target-font-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("regular.otf"), laml::text::FONT).unwrap();
        std::fs::write(dir.join("italic.otf"), laml::text::ITALIC_FONT).unwrap();
        let source = "settings {\n type: sequence\n font-file: \"regular.otf\"\n font-italic-file: \"italic.otf\"\n}\nsemantics {\n A\n B\n A ->> B: Existing label\n}\n";
        let action = Action::Create {
            origin: Dock {
                owner: "A".into(),
                before: 10,
                boundary: None,
            },
            operator: "->".into(),
        };
        let targets = vec![Target {
            dock: Dock {
                owner: "B".into(),
                before: 0,
                boundary: None,
            },
            same_row: false,
            straighten: false,
        }];
        let path = dir.join("diagram.laml");
        let fast = build(source, &path, Presentation::Editor, &action, &targets);
        let full = sequence_targets::build(
            crate::backend::local().as_ref(),
            source,
            &path,
            Presentation::Editor,
            &action,
            &targets,
        );
        assert!(!fast.is_empty());
        assert_eq!(
            serde_json::to_value(&fast).unwrap(),
            serde_json::to_value(&full).unwrap()
        );
        let outside = dir.with_extension("otf");
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&outside)
            .unwrap()
            .write_all(laml::text::FONT)
            .unwrap();
        let escaped = source.replace(
            "regular.otf",
            &format!("../{}", outside.file_name().unwrap().to_str().unwrap()),
        );
        assert!(build(&escaped, &path, Presentation::Editor, &action, &targets).is_empty());
        std::fs::remove_file(outside).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
