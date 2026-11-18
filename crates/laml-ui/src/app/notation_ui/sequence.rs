//! Sequence interaction owner. The old independent gesture handlers are gone.
use super::*;
use edit::sequence_controls::{self as controls, Boundary, Dock};
mod actions;
mod canvas;
mod catalog;
mod fragments;
pub(super) use catalog::recipes;
mod gestures;
mod insertion;
mod insertion_menu;
mod insertion_options;
mod menu_icons;
mod message_paint;
mod message_targets;
mod painting;
mod plus;
mod proposals;
mod reference_picker;
mod scene;
mod windows;
use scene::{Hit, Point, Scene};

#[derive(Default)]
pub(super) struct State {
    revision: Option<u64>,
    press: Option<(Hit, egui::Pos2)>,
    gesture: Option<gestures::Gesture>,
    pending_targets: Option<message_targets::Pending>,
    released: Option<egui::Pos2>,
    context: Option<Hit>,
    options: Option<insertion_options::Cache>,
    menu: Option<insertion_menu::Cache>,
    expanded: bool,
    picker: Option<insertion::Picker>,
    selection: Option<Hit>,
    touch_anchor: Option<(f64, f64)>,
    insert: Option<Point>,

    alternatives: Option<Vec<(String, Proposal)>>,
}
#[derive(Clone)]
struct Proposal {
    edits: Vec<laml_ui::document::Edit>,
    line: Option<usize>,
    label: &'static str,
}
#[derive(Clone)]
struct MessagePreview {
    message: laml::sequence::Message,
    send: (f64, f64),
    receive: (f64, f64),
    diagonal: bool,
    loop_width: Option<f64>,
}
#[derive(Clone)]
struct Destination {
    at: egui::Pos2,
    proposal: Proposal,
    alternatives: Vec<(String, Proposal)>,
    dock: Option<Dock>,
    hit_rect: Option<egui::Rect>,
    preview: Option<MessagePreview>,
}
impl Editor {
    pub(in crate::app) fn cancel_sequence_interaction(&mut self) {
        self.notation_ui.sequence.press = None;
        self.notation_ui.sequence.gesture = None;
        self.notation_ui.sequence.pending_targets = None;
        self.notation_ui.sequence.released = None;
    }
    fn sequence_apply(&mut self, p: Proposal) {
        if p.edits.is_empty() {
            return;
        }
        self.notation_ui.sequence.press = None;
        self.notation_ui.sequence.gesture = None;
        self.submit_command(Command {
            edits: p.edits,
            label: p.label.into(),
            expectation: worker::Expectation::Notation {
                line: p.line,
                id: None,
            },
        });
    }
    pub(super) fn sequence_add_participant(&mut self) {
        self.sequence_add_default(None, None);
    }
    fn sequence_add_default(&mut self, neighbor: Option<(String, bool)>, point: Option<Point>) {
        let prefix = if point.is_some() {
            "Note"
        } else {
            "Participant"
        };
        let source = self.document.source();
        let id = edit::sequence_create::fresh(source, prefix);
        let result = if let Some(p) = point {
            edit::sequence_scoped::insert(
                source,
                &p.dock,
                edit::sequence_insert::Action::Note,
                &id,
                "Note",
            )
        } else {
            controls::participant(
                source,
                &id,
                &id,
                neighbor.as_ref().map(|(id, left)| (id.as_str(), *left)),
            )
        };
        self.notation_ui.sequence.insert = None;
        match result {
            Ok(edits) => self.sequence_apply(Proposal {
                edits,
                line: None,
                label: "Add sequence element",
            }),
            Err(e) => self.command_error = Some(e),
        }
    }
}

#[cfg(test)]
mod insertion_tests;
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod touch_tests;

impl State {
    pub(super) fn clear_selection(&mut self) {
        self.selection = None;
    }
}

#[cfg(test)]
#[path = "sequence/horizontal_tests.rs"]
mod horizontal_tests;
