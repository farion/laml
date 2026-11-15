//! Resolve source lines to diagram elements using compiler-normalized references.
use crate::selection::Selection;
use laml::syntax::{Diagram, Property, StyleEntry};
use std::path::Path;
pub fn targets(diagram: &Diagram, _path: &Path, line: usize) -> Option<Selection> {
    targets_in_diagram(diagram, line)
}
/// Resolve already parsed compiler references without filesystem access.
pub fn targets_in_diagram(diagram: &Diagram, line: usize) -> Option<Selection> {
    let mut selected = Selection::default();
    for r in &diagram.layout {
        if r.line == line {
            selected.ids.extend([r.first.clone(), r.second.clone()]);
        }
    }
    for r in &diagram.alignments {
        if r.line == line {
            selected.ids.extend([r.first.clone(), r.second.clone()]);
        }
    }
    for n in &diagram.nodes {
        if n.parts
            .iter()
            .any(|p| matches!(p,laml::syntax::NodePart::Member{line:l,..} if *l==line))
        {
            selected.ids.push(n.id.clone());
        }
    }
    for r in &diagram.scoped_alignments {
        if r.line == line {
            selected.ids.extend([r.first.clone(), r.second.clone()]);
        }
    }
    for a in &diagram.attachments {
        if a.line == line {
            selected.relations.push(a.line);
        }
    }
    for r in &diagram.port_layout {
        if r.line == line {
            selected.ports.push(r.id.clone());
        }
    }
    for r in &diagram.relation_layout {
        if r.line == line {
            selected.relations.extend(
                diagram
                    .relations
                    .iter()
                    .filter(|e| e.name.as_ref() == Some(&r.name))
                    .map(|e| e.line),
            );
        }
    }
    for r in &diagram.relation_end_layout {
        if r.line == line {
            selected.relations.extend(
                diagram
                    .relations
                    .iter()
                    .filter(|e| e.name.as_ref() == Some(&r.name))
                    .map(|e| e.line),
            );
        }
    }
    if let Some(mut rule) = diagram.styling.iter().find_map(|e| match e {
        StyleEntry::Rule(r) if r.line == line || r.properties.iter().any(|p| p.line == line) => {
            Some(r.clone())
        }
        _ => None,
    }) {
        if !rule.properties.iter().any(|p| p.name == "display") {
            rule.properties.push(Property {
                name: "display".into(),
                value: "block".into(),
                line,
            });
        }
        // Only selector matching is needed. Re-loading LSS here was unnecessary
        // IO and is unavailable on WASM. The current preview already validated it.
        let mut sheet = laml::styles::StyleSheet::default();
        sheet.rules.push(laml::styles::Rule {
            selector: laml::styles::Selector::parse(&rule.selector),
            declarations: vec![laml::styles::Declaration {
                name: "display".into(),
                value: laml::styles::Value::Keyword("block".into()),
                line,
                path: Default::default(),
            }],
        });
        {
            for n in &diagram.nodes {
                let scoped = sheet.for_node(diagram, &n.id);
                if scoped
                    .get(
                        &laml::styles::Element::Node {
                            id: &n.id,
                            classes: &n.classes,
                        },
                        "display",
                    )
                    .is_some()
                {
                    selected.ids.push(n.id.clone());
                }
                let scoped = scoped.with_node_ancestor(&n.id, &n.classes);
                for part in &n.parts {
                    if let laml::syntax::NodePart::Member { classes, kind, .. } = part {
                        let id = crate::structure::member_address(&n.id, part);
                        if scoped
                            .get(
                                &laml::styles::Element::Member {
                                    id: id.as_deref(),
                                    classes,
                                    kind: *kind,
                                },
                                "display",
                            )
                            .is_some()
                        {
                            selected.ids.push(n.id.clone());
                        }
                    }
                }
                for p in &n.ports {
                    let id = crate::properties::port_id(&n.id, p);
                    if scoped
                        .get(
                            &laml::styles::Element::Port {
                                id: &id,
                                classes: &p.classes,
                            },
                            "display",
                        )
                        .is_some()
                    {
                        selected.ports.push(id);
                    }
                }
            }
            for r in &diagram.relations {
                if sheet
                    .get(
                        &laml::styles::Element::Relation {
                            name: r.name.as_deref(),
                            classes: &r.classes,
                        },
                        "display",
                    )
                    .is_some()
                {
                    selected.relations.push(r.line);
                }
            }
        }
    }
    selected.ids.sort();
    selected.ids.dedup();
    selected.relations.sort();
    selected.relations.dedup();
    selected.ports.sort();
    selected.ports.dedup();
    (!selected.ids.is_empty() || !selected.relations.is_empty() || !selected.ports.is_empty())
        .then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_and_contextual_property_lines_link_to_compiler_targets() {
        let source = "semantics {\n package P {\n A\n B\n }\n @r P.A -> P.B\n}\nlayout {\n P.A < P.B\n relation[r].target : side=left\n}\nstyling {\n node[P] node[A] {\n fill: #123456\n }\n}\n";
        let diagram = laml::parser::parse(source).unwrap();
        let path = Path::new("test.laml");
        assert_eq!(targets(&diagram, path, 9).unwrap().ids, ["P.A", "P.B"]);
        assert_eq!(targets(&diagram, path, 10).unwrap().relations, [6]);
        assert_eq!(targets(&diagram, path, 14).unwrap().ids, ["P.A"]);
        assert!(targets(&diagram, path, 1).is_none());
    }
}

#[cfg(test)]
mod attachment_link_tests {
    use super::*;
    #[test]
    fn attachment_source_selects_the_link_not_its_note_and_target() {
        let d = laml::parser::parse(
            "semantics {\n A\n B\n note N \"text\"\n @r A -> B\n N .. relation[r]\n}\n",
        )
        .unwrap();
        let selected = targets(&d, Path::new("test.laml"), 6).unwrap();
        assert_eq!(selected.relations, [6]);
        assert!(selected.ids.is_empty());
    }
}
