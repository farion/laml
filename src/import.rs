//! Explicit, loss-aware conversion of supported external diagram syntax to LAML.
pub mod cli;
mod mermaid;
mod plantuml;
mod structurizr;

use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Imported {
    pub name: String,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mermaid,
    PlantUml,
    StructurizrDsl,
    StructurizrJson,
}

impl Format {
    pub fn detect(path: &std::path::Path) -> Result<Self, String> {
        match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "mmd" | "mermaid" => Ok(Self::Mermaid),
            "puml" | "plantuml" | "pu" => Ok(Self::PlantUml),
            "dsl" => Ok(Self::StructurizrDsl),
            "json" => Ok(Self::StructurizrJson),
            _ => Err("expected .mmd, .puml, .dsl or .json; specify --from to override".into()),
        }
    }
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "mermaid" => Ok(Self::Mermaid),
            "plantuml" => Ok(Self::PlantUml),
            "structurizr-dsl" => Ok(Self::StructurizrDsl),
            "structurizr-json" => Ok(Self::StructurizrJson),
            _ => Err(format!("unknown import format '{name}'")),
        }
    }
}

pub fn convert(text: &str, format: Format) -> Result<Vec<Imported>, String> {
    match format {
        Format::Mermaid => mermaid::convert(text),
        Format::PlantUml => plantuml::convert(text),
        Format::StructurizrDsl => structurizr::dsl(text),
        Format::StructurizrJson => structurizr::json(text),
    }
}

pub(super) fn quoted(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

pub(super) fn identifier(raw: &str) -> Result<String, String> {
    if raw.is_empty()
        || !raw
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("identifier '{raw}' needs an explicit ASCII alias"));
    }
    Ok(raw.to_owned())
}

pub(super) fn line_error(line: usize, message: impl std::fmt::Display) -> String {
    format!("line {line}: {message}")
}

#[derive(Clone, Debug)]
pub(super) struct Item {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub shape: Option<String>,
    pub members: Vec<String>,
    pub group: Option<String>,
}
#[derive(Clone, Debug)]
pub(super) struct Link {
    pub from: String,
    pub to: String,
    pub operator: String,
    pub label: String,
}
#[derive(Default)]
pub(super) struct Graph {
    pub kind: String,
    pub direction: Option<String>,
    pub nodes: Vec<Item>,
    pub edges: Vec<Link>,
    pub index: HashMap<String, usize>,
    pub groups: Vec<(String, String)>,
}
impl Graph {
    pub fn node(&mut self, id: &str, label: Option<&str>, kind: &str) -> Result<(), String> {
        let id = identifier(id)?;
        if let Some(&index) = self.index.get(&id) {
            if let Some(label) = label {
                self.nodes[index].label = label.to_owned();
            }
            if !kind.is_empty() {
                if self.nodes[index].kind.is_empty() {
                    self.nodes[index].kind = kind.to_owned();
                } else if self.nodes[index].kind != kind {
                    return Err(format!("'{id}' has conflicting element kinds"));
                }
            }
        } else {
            self.index.insert(id.clone(), self.nodes.len());
            self.nodes.push(Item {
                id: id.clone(),
                label: label.unwrap_or(&id).to_owned(),
                kind: kind.to_owned(),
                shape: None,
                members: Vec::new(),
                group: None,
            });
        }
        Ok(())
    }
    pub fn edge(
        &mut self,
        from: &str,
        to: &str,
        operator: &str,
        label: &str,
    ) -> Result<(), String> {
        self.node(from, None, "")?;
        self.node(to, None, "")?;
        self.edges.push(Link {
            from: from.to_owned(),
            to: to.to_owned(),
            operator: operator.to_owned(),
            label: label.to_owned(),
        });
        Ok(())
    }
    fn address(&self, id: &str) -> String {
        self.index
            .get(id)
            .and_then(|&i| self.nodes[i].group.as_ref())
            .map_or_else(|| id.to_owned(), |group| format!("{group}.{id}"))
    }
    pub fn render(&self) -> Result<String, String> {
        if self.nodes.is_empty() {
            return Err("diagram contains no elements".into());
        }
        let mut out = String::new();
        if !self.kind.is_empty() || self.direction.is_some() {
            out.push_str("settings {\n");
            if !self.kind.is_empty() {
                out.push_str(&format!("    type: {}\n", self.kind));
            }
            if let Some(dir) = &self.direction {
                out.push_str(&format!("    direction: {dir}\n"));
            }
            out.push_str("}\n");
        }
        out.push_str("semantics {\n");
        let emit_node = |out: &mut String, item: &Item, depth: usize| {
            let indent = "    ".repeat(depth);
            let kind = if item.kind.is_empty() {
                String::new()
            } else {
                format!("{} ", item.kind)
            };
            let label = if item.label == item.id {
                String::new()
            } else {
                format!("(label={})", quoted(&item.label))
            };
            if item.members.is_empty() {
                out.push_str(&format!("{indent}{kind}{}{label}\n", item.id));
            } else {
                out.push_str(&format!("{indent}{kind}{}{label} {{\n", item.id));
                for member in &item.members {
                    out.push_str(&format!("{indent}    {member}\n"));
                }
                out.push_str(&format!("{indent}}}\n"));
            }
        };
        for item in self.nodes.iter().filter(|item| item.group.is_none()) {
            emit_node(&mut out, item, 1);
        }
        for (id, label) in &self.groups {
            let name = if id == label {
                String::new()
            } else {
                format!("(label={})", quoted(label))
            };
            out.push_str(&format!("    package {id}{name} {{\n"));
            for item in self
                .nodes
                .iter()
                .filter(|item| item.group.as_deref() == Some(id))
            {
                emit_node(&mut out, item, 2);
            }
            out.push_str("    }\n");
        }
        for edge in &self.edges {
            let label = if edge.label.is_empty() {
                String::new()
            } else {
                format!(" : {}", edge.label)
            };
            out.push_str(&format!(
                "    {} {} {}{label}\n",
                self.address(&edge.from),
                edge.operator,
                self.address(&edge.to)
            ));
        }
        out.push_str("}\n");
        if self.nodes.iter().any(|item| item.shape.is_some()) {
            out.push_str("styling {\n");
            for item in &self.nodes {
                if let Some(shape) = &item.shape {
                    out.push_str(&format!(
                        "    node[{}] {{\n        shape: {shape}\n    }}\n",
                        self.address(&item.id)
                    ));
                }
            }
            out.push_str("}\n");
        }
        Ok(out)
    }
}

// Split a DSL line into words, preserving whitespace inside quoted strings.
// A brace is a token only outside a quoted string.
pub(super) fn words(line: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if quoted && c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted;
            continue;
        }
        if !quoted && (c.is_whitespace() || c == '{' || c == '}') {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            if c == '{' || c == '}' {
                tokens.push(c.to_string());
            }
        } else {
            current.push(c);
        }
    }
    if quoted || escaped {
        return Err("unclosed quoted string".into());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

pub(super) fn unique_names(items: &[Imported]) -> Result<(), String> {
    let mut names = HashSet::new();
    for item in items {
        if !names.insert(&item.name) {
            return Err(format!(
                "duplicate imported view '{}'; use unique keys",
                item.name
            ));
        }
    }
    Ok(())
}
