//! GNE palette creates language elements and optional relative layout intent.
use crate::{
    document::Edit, node_edit, relation_edit::quote, source::SourceIndex, source_patch, style_edit,
};
use laml::syntax::{Diagram, NodeKind, Placement};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementKind {
    #[default]
    Rectangle,
    Ellipse,
    Diamond,
    Note,
    Package,
    Class,
    Interface,
    Enumeration,
    DataType,
    Object,
    Junction,
}
impl ElementKind {
    pub const ALL: [Self; 11] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Diamond,
        Self::Note,
        Self::Package,
        Self::Class,
        Self::Interface,
        Self::Enumeration,
        Self::DataType,
        Self::Object,
        Self::Junction,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Diamond => "Diamond",
            Self::Note => "Note",
            Self::Package => "Package",
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Enumeration => "Enumeration",
            Self::DataType => "Data type",
            Self::Object => "Object",
            Self::Junction => "Junction",
        }
    }
    pub fn shape(self) -> Option<&'static str> {
        match self {
            Self::Rectangle | Self::Package => Some("rectangle"),
            Self::Ellipse => Some("ellipse"),
            Self::Diamond => Some("diamond"),
            Self::Note => Some("note"),
            _ => None,
        }
    }
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Class => "class ",
            Self::Interface => "interface ",
            Self::Enumeration => "enum ",
            Self::DataType => "datatype ",
            Self::Object => "object ",
            Self::Junction => "junction ",
            _ => "",
        }
    }
    pub fn node_kind(self) -> NodeKind {
        match self {
            Self::Class => NodeKind::Class,
            Self::Interface => NodeKind::Interface,
            Self::Enumeration => NodeKind::Enumeration,
            Self::DataType => NodeKind::DataType,
            Self::Object => NodeKind::Object,
            Self::Junction => NodeKind::Junction,
            Self::Package => NodeKind::Package,
            Self::Note => NodeKind::Note,
            _ => NodeKind::Generic,
        }
    }
}

pub fn add(
    source: &str,
    kind: ElementKind,
    label: &str,
    parent: Option<&str>,
    relative: Option<(&str, Placement)>,
) -> Result<(String, Vec<Edit>), String> {
    let diagram = if source.trim().is_empty() {
        Diagram::default()
    } else {
        node_edit::diagram(source)?
    };
    if let Some(parent) = parent
        && !diagram
            .nodes
            .iter()
            .any(|n| n.id == parent && n.kind == NodeKind::Package)
    {
        return Err("Destination must be a package".into());
    }
    let prefix = match kind {
        ElementKind::Package => "Group",
        ElementKind::Note => "Note",
        _ => "Node",
    };
    let (name, id) = (1..).map(|i| {
        let name = format!("{prefix}{i}");
        let id = parent.map_or_else(|| name.clone(), |p| format!("{p}.{name}"));
        (name, id)
    }).find(|(name, id)| !diagram.nodes.iter().any(|n| &n.id == id || &n.id == name || n.id.rsplit('.').next() == Some(name.as_str()))
        && !diagram.relations.iter().any(|r| r.name.as_ref() == Some(name))
        && !diagram.attachments.iter().any(|r| r.name.as_ref() == Some(name))
        && !diagram.nodes.iter().any(|n| n.ports.iter().any(|p| p.explicit && &p.id == name)
            || n.parts.iter().any(|p| matches!(p, laml::syntax::NodePart::Member { id: Some(id), .. } if id == name))))
        .unwrap();
    let newline = newline(source);
    let attrs = if label.is_empty() || kind == ElementKind::Junction {
        String::new()
    } else {
        format!("(label={})", quote(label))
    };
    let declaration = match kind {
        ElementKind::Package => format!("package {name}{attrs} {{{newline}}}"),
        ElementKind::Note => format!(
            "note {name} {}",
            quote(if label.trim().is_empty() {
                "Note"
            } else {
                label
            })
        ),
        _ => format!("{}{name}{attrs}", kind.keyword()),
    };
    let mut edits = vec![insert(source, &diagram, parent, &declaration)?];
    if let Some(shape) = kind.shape() {
        edits.extend(style_edit::local_property(
            source,
            &[format!("node[{id}]")],
            "shape",
            Some(shape),
        )?);
    }
    if let Some((reference, placement)) = relative {
        let reference_node = diagram
            .nodes
            .iter()
            .find(|n| n.id == reference)
            .ok_or("Missing placement reference")?;
        if reference_node.parent.as_deref() != parent {
            return Err("Relative insertion requires the same container as its reference".into());
        }
        let operator = match placement {
            Placement::Left => "<",
            Placement::Right => ">",
            Placement::Above => "above",
            Placement::Below => "below",
        };
        edits.push(source_patch::insert_section(
            source,
            &diagram,
            "layout",
            &format!("{id} {operator} {reference}"),
        )?);
    }
    let edits = merge_insertions(edits);
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate)?;
    if diagram.relations.len() != updated.relations.len()
        || diagram
            .relations
            .iter()
            .zip(&updated.relations)
            .any(|(before, after)| before.from != after.from || before.to != after.to)
    {
        return Err(
            "Created element would capture an existing scoped reference. Source unchanged.".into(),
        );
    }
    if !updated.nodes.iter().any(|n| n.id == id) {
        return Err("Created node did not receive the intended ID".into());
    }
    Ok((id, edits))
}

pub(crate) fn newline(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
pub(crate) fn closing_start(source: &str, end: usize) -> usize {
    source[..end]
        .trim_end_matches(['\r', '\n'])
        .rfind('\n')
        .map_or(0, |i| i + 1)
}
pub(crate) fn insert(
    source: &str,
    diagram: &Diagram,
    parent: Option<&str>,
    declaration: &str,
) -> Result<Edit, String> {
    if let Some(parent) = parent {
        let node = diagram
            .nodes
            .iter()
            .find(|n| n.id == parent)
            .ok_or("Missing container")?;
        let blocks = node_edit::blocks(source, diagram)?;
        let block = blocks.get(&node.line).ok_or("Missing destination block")?;
        let position = closing_start(source, block.end);
        let range = SourceIndex::new(source)
            .line(node.line)
            .ok_or("Missing container line")?;
        let line = &source[range];
        let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
        let newline = newline(source);
        let content = declaration
            .replace("\r\n", "\n")
            .lines()
            .map(|line| format!("{indent}    {line}{newline}"))
            .collect();
        Ok(Edit {
            range: position..position,
            replacement: content,
        })
    } else {
        source_patch::insert_section(
            source,
            diagram,
            "semantics",
            &declaration.replace(newline(source), &format!("{}    ", newline(source))),
        )
    }
}

/// Distinct section creations can share EOF. Join them in transaction order.
pub(crate) fn merge_insertions(edits: Vec<Edit>) -> Vec<Edit> {
    let mut result: Vec<Edit> = Vec::new();
    for edit in edits {
        if edit.range.is_empty()
            && let Some(previous) = result.iter_mut().find(|old| old.range == edit.range)
        {
            previous.replacement.push_str(&edit.replacement);
        } else {
            result.push(edit);
        }
    }
    result
}
#[cfg(test)]
mod tests;
