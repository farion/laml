//! Contextual sequence authoring state; all persistent intent stays in LAML.
use super::*;
use edit::sequence_insert::{self as insert, Action};
mod canvas;
mod windows;
mod connections;
mod targets;
mod paint;
mod destruction;
mod drag;
mod participants;

#[derive(Clone, Debug)]
pub(super) struct Anchor {
    pub owner: String,
    pub before: usize,
    pub x: f64,
    pub y: f64,
    pub kind: Kind,
    pub end_line: Option<usize>,
    pub right: Option<String>,
    pub activation_line: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind { Head, Line, Execution, End }
#[derive(Default)]
pub(super) struct State {
    anchor: Option<Anchor>,
    menu_open: bool,
    revision: Option<u64>,
    connect: Option<(Anchor, Action)>,
    reconnect: Option<Reconnect>,
    destruction_drag: Option<destruction::Drag>,
    event_drag: Option<drag::Drag>,
    participant_drag: Option<participants::Drag>,
    dialog: Option<Form>,
}
impl State {
    pub(super) fn interacting(&self) -> bool {
        self.menu_open || self.connect.is_some() || self.reconnect.is_some() || self.dialog.is_some() || self.destruction_drag.is_some() || self.event_drag.is_some() || self.participant_drag.is_some()
    }
}
#[derive(Clone)]
struct Reconnect {
    line: usize,
    moving_source: bool,
    message: laml::sequence::Message,
    send: (f64, f64),
    receive: (f64, f64),
}
struct Form {
    anchor: Anchor,
    action: Action,
    target: String,
    id: String,
    label: String,
    revision: u64,
    error: Option<String>,
}
impl Editor {
    fn sequence_candidate(&self, anchor: &Anchor, action: Action, target: &str, id: &str, label: &str) -> Result<Vec<laml_ui::document::Edit>, String> {
        if action == Action::EndActivation && let Some(preview) = &self.preview {
            let bars: Vec<_> = preview.regions.iter().filter(|r| r.kind == "sequence-execution" && r.id == anchor.owner
                && anchor.y >= r.y && anchor.y <= r.y + r.height).collect();
            if let Some(bar) = bars.iter().find(|r| (r.x + r.width - anchor.x).abs() < 0.01)
                && bars.iter().any(|other| other.x + other.width / 2.0 > bar.x + bar.width / 2.0 + 0.1) {
                return Err("End the inner activation before ending this bar".into());
            }
        }
        insert::create(self.document.source(), anchor.before, &anchor.owner, action, target, id, label,
            anchor.right.as_deref(), anchor.end_line, anchor.activation_line)
    }
    fn sequence_action(&mut self, anchor: Anchor, action: Action) {
        use Action::*;
        self.notation_ui.sequence.menu_open = false;
        match action {
            Sync | SyncActivate | Async | Reply => {
                self.notation_ui.sequence.connect = Some((anchor, action));
                self.notation_ui.drag = None;
            }
            Participant | Note => self.sequence_form(anchor, action, String::new()),
            Activate | EndActivation | Destroy => match self.sequence_candidate(&anchor, action, "", "", "") {
                Ok(edits) => self.notation_submit("Edit sequence lifetime", edits, None),
                Err(e) => self.command_error = Some(e),
            },
        }
    }
    fn sequence_form(&mut self, anchor: Anchor, action: Action, target: String) {
        let prefix = if action == Action::Note { "Note" } else { "Participant" };
        let id = (1..).map(|i| format!("{prefix}{i}")).find(|id| !self.document.source().contains(id)).unwrap();
        self.notation_ui.sequence.dialog = Some(Form { anchor, action, target, label: String::new(), id,
            revision: self.document.revision(), error: None });
    }
}
