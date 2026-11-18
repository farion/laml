//! Relative drag/drop with a revision-bound, fully compiled hover proposal.
use super::*;
use laml::syntax::Placement;
use laml_ui::placement_edit::{self, PlacementPlan};
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Key {
    pub revision: u64,
    pub from: String,
    /// Empty only for a root-level membership drop; source IDs are nonempty.
    pub target: String,
    pub side: Option<Placement>,
}
impl Key {
    pub fn root(revision: u64, from: String) -> Self {
        Self {
            revision,
            from,
            target: String::new(),
            side: None,
        }
    }
    pub fn is_root(&self) -> bool {
        self.side.is_none() && self.target.is_empty()
    }
    fn membership(&self, source: &str) -> Result<laml_ui::membership_edit::Move, String> {
        if self.is_root() {
            laml_ui::membership_edit::plan(source, &self.from, None)
        } else {
            laml_ui::membership_edit::plan_drop(source, &self.from, &self.target)
        }
    }
}
pub(super) struct PlacementUi {
    pub drag: Option<(String, u64)>,
    pub key: Option<Key>,
    pub plan: Option<Result<PlacementPlan, String>>,
    pub compiled: Option<Result<Preview, String>>,
    pub released: bool,
    pub moved_id: Option<String>,
    pub drag_origin: Option<egui::Pos2>,
    checks: Vec<(Key, bool)>,
    pub rejected: Vec<Key>,
    since: Instant,
    submitted: bool,
    compiler: worker::Client,
}
impl PlacementUi {
    pub fn with_backend(document: u64, backend: laml_ui::services::Services) -> Self {
        Self {
            drag: None,
            key: None,
            plan: None,
            compiled: None,
            released: false,
            moved_id: None,
            drag_origin: None,
            checks: Vec::new(),
            rejected: Vec::new(),
            since: Instant::now(),
            submitted: false,
            compiler: worker::Client::with_client(document, backend.client()),
        }
    }
}
impl PlacementUi {
    pub fn set_presentation(&mut self, presentation: laml_ui::backend::Presentation) {
        self.compiler.presentation = presentation;
    }
    pub fn cancel(&mut self) {
        self.compiler.forget_current();
        self.drag = None;
        self.released = false;
        self.moved_id = None;
        self.drag_origin = None;
        self.key = None;
        self.plan = None;
        self.compiled = None;
        self.submitted = false;
        self.checks.clear();
        self.rejected.clear();
    }
    pub fn allowed(&mut self, key: &Key, source: &str) -> bool {
        if self.rejected.contains(key) {
            return false;
        }
        if let Some((_, valid)) = self.checks.iter().find(|(old, _)| old == key) {
            return *valid;
        }
        let valid = match key.side {
            Some(side) => placement_edit::plan(source, &key.from, &key.target, side).is_ok(),
            None => key.membership(source).is_ok(),
        };
        self.checks.push((key.clone(), valid));
        valid
    }
    pub fn hover(&mut self, key: Option<Key>, source: &str, path: &std::path::Path) {
        if key != self.key {
            self.compiler.forget_current();
            self.key = key.clone();
            self.compiled = None;
            self.submitted = false;
            self.since = Instant::now();
            self.moved_id = None;
            self.plan = key.map(|key| {
                if let Some(side) = key.side {
                    placement_edit::plan(source, &key.from, &key.target, side)
                } else {
                    let moved = key.membership(source)?;
                    self.moved_id = Some(moved.id);
                    let candidate = laml_ui::notation_edit::apply(source, &moved.edits)?;
                    Ok(PlacementPlan {
                        edits: moved.edits,
                        removed: vec![],
                        preserved: vec![],
                        added: if key.is_root() {
                            "Move out of container".into()
                        } else {
                            format!("Move into {}", key.target)
                        },
                        source: candidate,
                    })
                }
            });
        }
        loop {
            match self.compiler.try_recv() {
                Ok((revision, result, _))
                    if self
                        .key
                        .as_ref()
                        .is_some_and(|key| key.revision == revision) =>
                {
                    self.compiled = Some(result.map_err(|e| e.to_string()));
                }
                Ok(_) => {}
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.submitted {
                        self.compiled = Some(Err("Placement backend stopped".into()));
                    }
                    break;
                }
            }
        }
        if !self.submitted
            && (self.released || self.since.elapsed() >= Duration::from_millis(60))
            && let (Some(key), Some(Ok(plan))) = (&self.key, &self.plan)
        {
            if let Err(error) =
                self.compiler
                    .send((key.revision, plan.source.clone(), path.into(), None))
            {
                self.compiled = Some(Err(error.to_string()));
            }
            self.submitted = true;
        }
    }
    pub fn ready(&self) -> bool {
        matches!(&self.compiled, Some(Ok(_)))
    }
}

impl Editor {
    pub(super) fn poll_placement(&mut self, ctx: &egui::Context) {
        if !self.placement.released {
            return;
        }
        if !self.can_edit_current_revision()
            || self
                .placement
                .key
                .as_ref()
                .is_none_or(|k| k.revision != self.document.revision())
            || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.placement.cancel();
            return;
        }
        let key = self.placement.key.clone();
        self.placement
            .hover(key, self.document.source(), &self.path);
        if self.placement.compiled.is_some() {
            self.finish_placement(ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    pub(super) fn finish_placement(&mut self, ctx: &egui::Context) {
        let valid = self.can_edit_current_revision()
            && self
                .placement
                .key
                .as_ref()
                .is_some_and(|key| key.revision == self.document.revision());
        if valid
            && self.placement.plan.as_ref().is_some_and(|p| p.is_ok())
            && self.placement.compiled.is_none()
        {
            self.placement.released = true;
            self.placement.drag = None;
            return;
        }
        if valid
            && self.placement.ready()
            && let (Some(Ok(plan)), Some(Ok(preview))) =
                (self.placement.plan.take(), self.placement.compiled.take())
        {
            let revision = self.document.revision();
            match self
                .document
                .apply(revision, "Relative placement", plan.edits)
            {
                Ok(_) => {
                    if let Some(id) = self.placement.moved_id.take() {
                        self.selection.click(Some(&id), false);
                    }
                    self.publish_preview(ctx, preview);
                    self.command_error = None;
                }
                Err(error) => self.error_toast(format!("Placement cancelled: {error:?}")),
            }
        } else {
            let error = match (&self.placement.plan, &self.placement.compiled) {
                (Some(Err(error)), _) | (_, Some(Err(error))) => error.clone(),
                _ => "Move cancelled: no valid drop area. Source unchanged.".into(),
            };
            self.error_toast(error);
        }
        self.placement.cancel();
    }
}
