//! UML communication diagrams: numbered messages on declared lifeline links.
mod render;
pub use render::render;

use crate::lex::{split_once_top_level, valid_identifier, without_comment};
use crate::parser::{self, ParseError};
use crate::syntax::{Diagram, DiagramType, NodeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub number: Vec<u32>,
    pub from: String,
    pub to: String,
    pub label: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub graph: Diagram,
    pub interaction: String,
    pub messages: Vec<Message>,
}

fn error(line: usize, text: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: text.into(),
    }
}

pub fn is_communication_source(source: &str) -> bool {
    let mut settings = false;
    for line in source.lines() {
        let text = without_comment(line).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings && text == "type: communication" {
            return true;
        }
    }
    false
}

fn message_label_valid(label: &str) -> bool {
    let name = label.split_once('(').map_or(label, |(name, _)| name);
    if !valid_identifier(name) || label.contains(['[', ']']) {
        return false;
    }
    let Some(args) = label.strip_prefix(name) else {
        return false;
    };
    if args.is_empty() {
        return true;
    }
    if !args.starts_with('(') || !args.ends_with(')') {
        return false;
    }
    let mut depth = 0_i32;
    let mut quoted = false;
    let mut escaped = false;
    for ch in args.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        } else if !quoted && ch == '(' {
            depth += 1;
        } else if !quoted && ch == ')' {
            depth -= 1;
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0 && !quoted && !escaped
}

fn message(text: &str, line: usize) -> Result<Message, ParseError> {
    let rest = text
        .strip_prefix("message ")
        .ok_or_else(|| error(line, "expected message"))?;
    let (head, label) = split_once_top_level(rest, ':')
        .ok_or_else(|| error(line, "numbered message needs ': name()'"))?;
    let words: Vec<_> = head.split_whitespace().collect();
    if words.len() != 4
        || words[2] != "-->"
        || !valid_identifier(words[1])
        || !valid_identifier(words[3])
    {
        return Err(error(
            line,
            "message requires number Sender --> Receiver : name()",
        ));
    }
    let mut number = Vec::new();
    for component in words[0].split('.') {
        if component.starts_with('0')
            || component.is_empty()
            || !component.bytes().all(|ch| ch.is_ascii_digit())
        {
            return Err(error(
                line,
                "sequence expression requires positive dot-separated integers",
            ));
        }
        let value = component
            .parse::<u32>()
            .map_err(|_| error(line, "sequence number exceeds supported range"))?;
        if value == 0 {
            return Err(error(line, "sequence terms must be positive"));
        }
        number.push(value);
    }
    let label = label.trim();
    if !message_label_valid(label) {
        return Err(error(
            line,
            "message needs a name or name(arguments); guard and iteration syntax is not supported",
        ));
    }
    Ok(Message {
        number,
        from: words[1].into(),
        to: words[3].into(),
        label: label.into(),
        line,
    })
}

pub fn parse(source: &str) -> Result<Document, ParseError> {
    let mut stripped = String::new();
    let mut section = "";
    let mut interaction = None;
    let mut messages = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(raw).trim();
        if section.is_empty() && matches!(text, "settings {" | "semantics {") {
            section = if text == "settings {" {
                "settings"
            } else {
                "semantics"
            };
        } else if !section.is_empty() && text == "}" {
            section = "";
        } else if section == "settings" && text.starts_with("interaction:") {
            let name = text["interaction:".len()..].trim();
            if !valid_identifier(name) || interaction.replace(name.to_owned()).is_some() {
                return Err(error(
                    line,
                    "interaction setting requires one unique simple name",
                ));
            }
            stripped.push('\n');
            continue;
        } else if section == "semantics" && text.starts_with("message ") {
            messages.push(message(text, line)?);
            stripped.push('\n');
            continue;
        } else if section == "semantics" && text.ends_with('{') {
            return Err(error(
                line,
                "communication subset has no nested lifeline or note blocks",
            ));
        }
        stripped.push_str(raw);
        stripped.push('\n');
    }
    let graph = parser::parse_interaction_graph(&stripped)?;
    if graph.settings.diagram_type != DiagramType::Communication {
        return Err(error(
            1,
            "communication diagram requires type: communication",
        ));
    }
    let interaction =
        interaction.ok_or_else(|| error(1, "communication diagram requires interaction: Name"))?;
    if messages.is_empty() || graph.relations.is_empty() {
        return Err(error(
            1,
            "communication diagram requires links and numbered messages",
        ));
    }
    let mut numbers = std::collections::HashSet::new();
    for item in &messages {
        if !numbers.insert(item.number.clone()) {
            return Err(error(
                item.line,
                "duplicate communication sequence expression",
            ));
        }
        if item.from == item.to {
            return Err(error(
                item.line,
                "self messages need a separately checked loop notation",
            ));
        }
        if ![&item.from, &item.to].iter().all(|id| {
            graph
                .nodes
                .iter()
                .any(|n| &n.id == *id && n.kind == NodeKind::Object)
        }) {
            return Err(error(
                item.line,
                "message endpoints must be declared lifelines",
            ));
        }
        let count = graph
            .relations
            .iter()
            .filter(|link| {
                (link.from == item.from && link.to == item.to)
                    || (link.from == item.to && link.to == item.from)
            })
            .count();
        if count != 1 {
            return Err(error(
                item.line,
                "message requires exactly one undirected link between its lifelines",
            ));
        }
    }
    for item in &messages {
        if item.number.len() > 1 && !numbers.contains(&item.number[..item.number.len() - 1]) {
            return Err(error(
                item.line,
                "nested sequence expression requires its parent message",
            ));
        }
    }
    for (index, link) in graph.relations.iter().enumerate() {
        if graph.relations[..index].iter().any(|prev| {
            (prev.from == link.from && prev.to == link.to)
                || (prev.from == link.to && prev.to == link.from)
        }) {
            return Err(error(
                link.line,
                "duplicate communication link between lifelines",
            ));
        }
    }
    Ok(Document {
        graph,
        interaction,
        messages,
    })
}
