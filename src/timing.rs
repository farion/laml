//! Checked UML discrete timing diagrams; time values are explicit, never inferred from order.
mod render;
pub use render::render;

use crate::lex::{valid_identifier, valid_reference, without_comment};
use crate::parser::ParseError;
use crate::syntax::StyleEntry;

#[derive(Clone, Debug)]
pub struct Change {
    pub time: f64,
    pub state: String,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Event {
    pub time: f64,
    pub label: String,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Lifeline {
    pub id: String,
    pub classifier: Option<String>,
    pub initial: String,
    pub changes: Vec<Change>,
    pub events: Vec<Event>,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Tick {
    pub time: f64,
    pub label: String,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Document {
    pub theme: crate::theme::Theme,
    pub interaction: String,
    pub unit: Option<String>,
    pub lifelines: Vec<Lifeline>,
    pub ticks: Vec<Tick>,
    pub scale: f64,
    pub lane_gap: f64,
    pub end: f64,
    pub styling: Vec<StyleEntry>,
}
fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}
fn time(text: &str, line: usize) -> Result<f64, ParseError> {
    let text = text.trim();
    if text.is_empty()
        || !text.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        || text.matches('.').count() > 1
    {
        return Err(error(
            line,
            "time must be a nonnegative finite decimal number",
        ));
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| error(line, "invalid decimal time"))?;
    if !value.is_finite() {
        return Err(error(line, "time must be finite"));
    }
    Ok(value)
}
fn positive(text: &str, line: usize, name: &str) -> Result<f64, ParseError> {
    let value = time(text, line)?;
    if value <= 0.0 {
        return Err(error(line, format!("{name} must be positive")));
    }
    Ok(value)
}
fn state_name(text: &str, line: usize) -> Result<String, ParseError> {
    let name = text.trim();
    if !valid_identifier(name) || name.len() > 48 {
        return Err(error(line, "state or condition requires a simple name"));
    }
    Ok(name.into())
}
fn split_colon(text: &str, line: usize) -> Result<(&str, &str), ParseError> {
    text.split_once(':')
        .map(|(a, b)| (a.trim(), b.trim()))
        .ok_or_else(|| error(line, "expected ':' before state or event name"))
}
fn lifeline(text: &str, line: usize) -> Result<Lifeline, ParseError> {
    let (head, state) = split_colon(text, line)?;
    let initial = state_name(state, line)?;
    let (id, classifier) = if let Some((id, tail)) = head.split_once('(') {
        let target = tail
            .strip_prefix("of=")
            .and_then(|s| s.strip_suffix(')'))
            .filter(|s| valid_reference(s) && s.len() <= 80)
            .ok_or_else(|| error(line, "lifeline type needs (of=Classifier)"))?;
        (id.trim(), Some(target.to_owned()))
    } else {
        (head, None)
    };
    if !valid_identifier(id) || id.len() > 48 {
        return Err(error(line, "lifeline needs a simple instance ID"));
    }
    Ok(Lifeline {
        id: id.into(),
        classifier,
        initial,
        changes: Vec::new(),
        events: Vec::new(),
        line,
    })
}
fn occurrence(text: &str, line: usize) -> Result<(&str, f64, &str), ParseError> {
    let (head, value) = split_colon(text, line)?;
    let (id, at) = head
        .split_once(" at ")
        .ok_or_else(|| error(line, "expected Lifeline at Time : Name"))?;
    if !valid_identifier(id.trim()) || value.is_empty() {
        return Err(error(line, "expected Lifeline at Time : Name"));
    }
    Ok((id.trim(), time(at, line)?, value))
}

pub fn is_timing_source(source: &str) -> bool {
    let mut settings = false;
    for raw in source.lines() {
        let text = without_comment(raw).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings && text == "type: timing" {
            return true;
        }
    }
    false
}

pub fn parse(source: &str) -> Result<Document, ParseError> {
    let (stripped, theme, _, _, _) =
        crate::theme::preprocess_settings(source).map_err(|message| error(1, message))?;
    let (source, styling) = crate::sequence::styling::extract(&stripped)?;
    let mut section = "";
    let mut seen = [false; 3];
    let mut ty = false;
    let mut interaction = None;
    let mut unit = None;
    let mut scale = None;
    let mut gap = None;
    let mut end = None;
    let mut lifelines: Vec<Lifeline> = Vec::new();
    let mut ticks: Vec<Tick> = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(raw).trim();
        if text.is_empty() {
            continue;
        }
        if section.is_empty() {
            let slot = match text {
                "settings {" => 0,
                "semantics {" => 1,
                "layout {" => 2,
                _ => {
                    return Err(error(
                        line,
                        "expected settings, semantics or layout section",
                    ));
                }
            };
            if seen[slot] {
                return Err(error(line, "duplicate timing section"));
            }
            seen[slot] = true;
            section = match slot {
                0 => "settings",
                1 => "semantics",
                _ => "layout",
            };
            continue;
        }
        if text == "}" {
            section = "";
            continue;
        }
        match section {
            "settings" => {
                let (name, value) = split_colon(text, line)?;
                match name {
                    "type" if value == "timing" && !ty => ty = true,
                    "interaction"
                        if valid_identifier(value)
                            && value.len() <= 80
                            && interaction.is_none() =>
                    {
                        interaction = Some(value.to_owned())
                    }
                    "time-unit"
                        if valid_identifier(value) && value.len() <= 16 && unit.is_none() =>
                    {
                        unit = Some(value.to_owned())
                    }
                    _ => return Err(error(line, "unknown, invalid or duplicate timing setting")),
                }
            }
            "semantics" => {
                if let Some(rest) = text.strip_prefix("lifeline ") {
                    let participant = lifeline(rest, line)?;
                    if lifelines.iter().any(|p| p.id == participant.id) {
                        return Err(error(line, "duplicate timing lifeline"));
                    }
                    lifelines.push(participant);
                } else if let Some(rest) = text.strip_prefix("state ") {
                    let (id, at, name) = occurrence(rest, line)?;
                    let new = state_name(name, line)?;
                    let participant =
                        lifelines.iter_mut().find(|p| p.id == id).ok_or_else(|| {
                            error(
                                line,
                                "state change requires an earlier lifeline declaration",
                            )
                        })?;
                    if at <= participant.changes.last().map_or(0.0, |c| c.time) {
                        return Err(error(
                            line,
                            "state changes must have strictly increasing positive times per lifeline",
                        ));
                    }
                    if participant
                        .changes
                        .last()
                        .map_or(participant.initial.as_str(), |c| &c.state)
                        == new
                    {
                        return Err(error(
                            line,
                            "unchanged state must not be drawn as a transition",
                        ));
                    }
                    participant.changes.push(Change {
                        time: at,
                        state: new,
                        line,
                    });
                } else if let Some(rest) = text.strip_prefix("event ") {
                    let (id, at, label) = occurrence(rest, line)?;
                    if label.contains(['\n', '\r', '<', '>']) || label.len() > 80 {
                        return Err(error(
                            line,
                            "event label requires readable plain text of at most 80 characters",
                        ));
                    }
                    let participant =
                        lifelines.iter_mut().find(|p| p.id == id).ok_or_else(|| {
                            error(line, "event requires an earlier lifeline declaration")
                        })?;
                    if participant.events.iter().any(|event| event.time == at) {
                        return Err(error(line, "only one event label may mark one transition"));
                    }
                    participant.events.push(Event {
                        time: at,
                        label: label.into(),
                        line,
                    });
                } else {
                    return Err(error(
                        line,
                        "unsupported timing declaration; use lifeline, state or event",
                    ));
                }
            }
            "layout" => {
                if let Some(value) = text.strip_prefix("scale:") {
                    if scale.is_some() {
                        return Err(error(line, "duplicate time scale"));
                    }
                    scale = Some(positive(value, line, "scale")?);
                } else if let Some(value) = text.strip_prefix("lane-gap:") {
                    if gap.is_some() {
                        return Err(error(line, "duplicate lane-gap"));
                    }
                    gap = Some(positive(value, line, "lane-gap")?);
                } else if let Some(value) = text.strip_prefix("end:") {
                    if end.is_some() {
                        return Err(error(line, "duplicate end"));
                    }
                    end = Some(positive(value, line, "end")?);
                } else if let Some(value) = text.strip_prefix("tick ") {
                    let value = value.trim();
                    let at = time(value, line)?;
                    if ticks.iter().any(|tick| tick.time == at) {
                        return Err(error(line, "duplicate timing ruler tick"));
                    }
                    ticks.push(Tick {
                        time: at,
                        label: value.into(),
                        line,
                    });
                } else {
                    return Err(error(line, "unsupported timing layout rule"));
                }
            }
            _ => unreachable!(),
        }
    }
    if !section.is_empty() {
        return Err(error(
            source.lines().count().max(1),
            "unclosed timing section",
        ));
    }
    if !ty || !seen[1] {
        return Err(error(1, "timing diagram needs type: timing and semantics"));
    }
    let interaction =
        interaction.ok_or_else(|| error(1, "timing diagram needs interaction: Name"))?;
    if lifelines.is_empty()
        || lifelines.len() > 12
        || lifelines.iter().all(|p| p.changes.is_empty())
    {
        return Err(error(
            1,
            "timing diagram needs a lifeline and at least one state transition",
        ));
    }
    for participant in &lifelines {
        if participant.changes.len() > 32 {
            return Err(error(
                participant.line,
                "at most 32 state changes per timing lifeline",
            ));
        }
        for event in &participant.events {
            if !participant
                .changes
                .iter()
                .any(|change| change.time == event.time)
            {
                return Err(error(
                    event.line,
                    "event label must mark an explicit state transition on its lifeline",
                ));
            }
        }
    }
    if ticks.len() < 2
        || !ticks.iter().any(|tick| tick.time == 0.0)
        || !ticks.iter().any(|tick| tick.time > 0.0)
    {
        return Err(error(
            1,
            "timing ruler needs tick 0 and at least one positive tick",
        ));
    }
    let maximum = lifelines
        .iter()
        .flat_map(|p| p.changes.iter().map(|c| c.time))
        .chain(ticks.iter().map(|t| t.time))
        .fold(0.0_f64, f64::max);
    let end = end.unwrap_or(maximum + 2.0);
    if !end.is_finite() || end <= maximum {
        return Err(error(
            1,
            "end must exceed every state change and ruler tick",
        ));
    }
    let scale = scale.unwrap_or(70.0);
    let lane_gap = gap.unwrap_or(62.0);
    if !(20.0..=300.0).contains(&scale)
        || !(36.0..=300.0).contains(&lane_gap)
        || end * scale > 6000.0
    {
        return Err(error(
            1,
            "timing scale, lane-gap or canvas is outside supported bounds",
        ));
    }
    ticks.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(Document {
        theme,
        interaction,
        unit,
        lifelines,
        ticks,
        scale,
        lane_gap,
        end,
        styling,
    })
}
