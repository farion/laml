use super::*;

impl Editor {
    pub(in crate::app) fn reset_view(&mut self, ctx: &egui::Context) {
        self.graph = graph::State::default();
        self.path = self
            .file
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.laml"));
        if let Some((uri, _)) = &self.image {
            ctx.forget_image(uri);
        }
        self.image = None;
        self.preview = None;
        self.rendered = None;
        self.submitted = None;
        self.retry_after = None;
        self.selection.ids.clear();
        self.selection.relations.clear();
        self.selection.ports.clear();
        self.relation_drag = None;
        self.deletion = None;
        self.membership = None;
        self.source_jump = None;
        if let Some(id) = self.source_ui.id {
            egui::text_edit::TextEditState::default().store(ctx, id);
        }
        self.source_ui = Default::default();
        self.advanced = Default::default();
        self.view = Default::default();
        self.workflow.invalidate_watch();
        self.structure = Default::default();
        self.property_drafts.clear();
        self.pending_command = None;
        self.creation.drag = None;
        #[cfg(test)]
        {
            self.creation.palette.parent.clear();
            self.creation.palette.relative = 0;
        }
        self.placement.cancel();
        self.alignment = None;
        self.command_error = None;
        self.error = None;
        self.changed = Instant::now() - Duration::from_secs(1);
    }
}
