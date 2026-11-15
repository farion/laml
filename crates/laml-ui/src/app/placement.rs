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
    pub target: String,
    pub side: Placement,
}
pub(super) struct PlacementUi {
    pub drag: Option<(String, u64)>,
    pub key: Option<Key>,
    pub plan: Option<Result<PlacementPlan, String>>,
    pub compiled: Option<Result<Preview, String>>,
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
        self.key = None;
        self.plan = None;
        self.compiled = None;
        self.submitted = false;
    }
    pub fn hover(&mut self, key: Option<Key>, source: &str, path: &std::path::Path) {
        if key != self.key {
            self.compiler.forget_current();
            self.key = key.clone();
            self.compiled = None;
            self.submitted = false;
            self.since = Instant::now();
            self.plan =
                key.map(|key| placement_edit::plan(source, &key.from, &key.target, key.side));
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
            && self.since.elapsed() >= Duration::from_millis(120)
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
    pub(super) fn placement_status(&self, ui: &mut egui::Ui) {
        if self.placement.drag.is_none() {
            return;
        }
        ui.heading("Relative placement");
        match &self.placement.plan {
            None => {
                ui.label(
                    "Hover over a side of another node.",
                );
            }
            Some(Err(error)) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            Some(Ok(plan)) => {
                ui.label(format!("Add: {}", plan.added));
                for removed in &plan.removed {
                    ui.label(format!("Remove: {removed}"));
                }
                for preserved in &plan.preserved {
                    ui.label(format!("Preserve: {preserved}"));
                }
                if plan.removed.is_empty() {
                    ui.label("No existing rule removed.");
                }
                match &self.placement.compiled {
                    None => {
                        ui.label("Checking layout and routing… Hold here until green.");
                    }
                    Some(Err(error)) => {
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            format!("Drop blocked: {error}"),
                        );
                    }
                    Some(Ok(_)) => {
                        ui.colored_label(
                            egui::Color32::LIGHT_GREEN,
                            "Release to apply. One undo step.",
                        );
                    }
                }
            }
        }
        ui.separator();
    }
    pub(super) fn finish_placement(&mut self, ctx: &egui::Context) {
        let valid = self.can_edit_visually()
            && self
                .placement
                .key
                .as_ref()
                .is_some_and(|key| key.revision == self.document.revision());
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
                    self.publish_preview(ctx, preview);
                    self.command_error = None;
                }
                Err(error) => self.command_error = Some(format!("Placement cancelled: {error:?}")),
            }
        } else if self.placement.key.is_some() {
            self.command_error = Some(match (&self.placement.plan, &self.placement.compiled) {
                (Some(Err(error)), _) | (_, Some(Err(error))) => format!("Drop blocked: {error}"),
                _ => "Drop cancelled: placement was not validated. Hold over a zone until it turns green.".into(),
            });
        }
        self.placement.cancel();
    }
}
