mod activation_layout;
mod anchors;
mod annotations;
mod arrows;
mod authoring;
mod bar_attachments;
mod continuations;
mod coregions;
mod decompositions;
mod destruction_layout;
mod executions;
mod frames;
mod gates;
mod geometry;
mod lane_gaps;
mod lifetimes;
pub mod message_geometry;
mod messages;
mod notes;
mod order;
mod reflow;
mod retime;
mod styles;
mod text_collision;
use super::{Event, Message, MessageKind, SequenceDiagram};
use crate::styles::StyleSheet;
use crate::svg::TextMode;
use crate::text::FONT;
use base64::Engine;
use executions::{DrawnExecution, OpenExecution};
use geometry::{frame_right, head_label, label_width_at, lanes, x_of};
use std::fmt::Write;
use styles::escape;
const HEADER_Y: f64 = 32.0;
const HEADER_HEIGHT: f64 = 48.0;
type Separator = (String, f64);
struct OpenFrame {
    line: usize,
    operator: String,
    id: Option<String>,
    guard: String,
    top: f64,
    separators: Vec<Separator>,
    incoming_execution: Vec<OpenExecution>,
    messages: Vec<usize>,
}
struct DrawnFrame {
    line: usize,
    operator: String,
    id: Option<String>,
    guard: String,
    top: f64,
    bottom: f64,
    separators: Vec<Separator>,
    messages: Vec<usize>,
}
use styles::styled_text;
pub fn render(diagram: &SequenceDiagram, mode: TextMode) -> Result<String, String> {
    render_with_styles(diagram, &StyleSheet::default(), mode)
}
pub fn render_with_styles(
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    mode: TextMode,
) -> Result<String, String> {
    render_for_editor(diagram, sheet, mode).map(|(svg, _)| svg)
}
mod document;
pub use document::render_for_editor;
