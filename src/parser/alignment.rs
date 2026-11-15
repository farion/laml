use super::{ParseError, error};
use crate::{
    lex::valid_reference,
    syntax::{AlignmentRule, Diagram, FreeAlignment},
};

pub(super) fn parse(text: &str, line: usize) -> Result<AlignmentRule, ParseError> {
    let (pair, attribute) = text.split_once(':').ok_or_else(|| {
        error(
            line,
            "expected A ? B : align=left|right|top|bottom|center-x|center-y|center",
        )
    })?;
    let words: Vec<_> = pair.split_whitespace().collect();
    let [first, "?", second] = words.as_slice() else {
        return Err(error(line, "expected alignment pair A ? B"));
    };
    if !valid_reference(first) || !valid_reference(second) {
        return Err(error(line, "invalid alignment node reference"));
    }
    let (name, value) = attribute
        .trim()
        .split_once('=')
        .ok_or_else(|| error(line, "independent alignment requires align=value"))?;
    if name.trim() != "align" {
        return Err(error(
            line,
            "independent alignment only accepts align=value",
        ));
    }
    let align = FreeAlignment::parse(value.trim()).ok_or_else(|| error(line, "invalid independent alignment; use left, right, top, bottom, center-x, center-y or center"))?;
    Ok(AlignmentRule {
        first: (*first).into(),
        second: (*second).into(),
        align,
        line,
    })
}

pub(super) fn validate(diagram: &Diagram) -> Result<(), ParseError> {
    for rule in &diagram.alignments {
        let node = |id: &str| {
            diagram
                .nodes
                .iter()
                .find(|node| node.id == id)
                .ok_or_else(|| error(rule.line, format!("unknown alignment node '{id}'")))
        };
        let a = node(&rule.first)?;
        let b = node(&rule.second)?;
        if a.id == b.id {
            return Err(error(
                rule.line,
                "independent alignment needs two distinct nodes",
            ));
        }
        if rule.align == FreeAlignment::Center && a.parent != b.parent {
            return Err(error(
                rule.line,
                "shared-center overlap requires nodes in the same container",
            ));
        }
    }
    Ok(())
}
