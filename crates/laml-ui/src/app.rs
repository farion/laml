//! Shared desktop/browser egui shell; host work uses asynchronous services.
use eframe::egui;
use laml_ui::document::Document;
use std::{path::PathBuf, sync::mpsc, time::Duration};

mod advanced;
mod alignment;
mod alignment_guides;
mod appearance;
#[cfg(test)]
mod backend_tests;
mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
#[cfg(target_arch = "wasm32")]
#[path = "app/capture_web.rs"]
mod capture;
mod command_errors;
mod connection_geometry;
mod connection_gesture;
mod connection_paint;
mod connection_preview;
mod creation;
mod declaration_fields;
mod deletion;
#[cfg(test)]
mod deletion_tests;
mod dialogs;
mod document_settings;
mod editor_theme;
mod element_tree;
mod file_ui;
mod footer_status;
mod graph;
mod graph_context;
mod graph_dialogs;
mod graph_header;
#[cfg(test)]
mod graph_tests;
#[cfg(not(target_arch = "wasm32"))]
mod host_dialogs;
#[cfg(target_arch = "wasm32")]
#[path = "app/host_dialogs_web.rs"]
mod host_dialogs;
mod icon_picker;
#[cfg(test)]
mod icon_picker_tests;
mod inline_text;
mod inspector;
#[cfg(test)]
mod inspector_layout_tests;
#[cfg(test)]
mod inspector_tests;
mod inspector_ui;
mod membership;
#[cfg(test)]
mod membership_tests;
mod notation_ui;
#[cfg(test)]
mod object_tests;
mod objects;
#[cfg(test)]
mod palette;
#[cfg(test)]
mod palette_tests;
mod placement;
mod placement_canvas;
mod ports;
mod property_rows;
mod recovery_ui;
mod relation_canvas;
mod relation_picker;
#[cfg(test)]
mod relation_tests;
mod relations;
mod shell;
mod source_editor;
mod source_header;
#[cfg(test)]
mod source_tests;
mod source_tools;
mod structure_ui;
#[cfg(test)]
mod tests;
mod toolbar;
mod view;
mod worker;
mod workflow;
mod workspace;
use laml_ui::{preview::Preview, selection::Selection};
use web_time::Instant;
use worker::Command;
mod remote_ui;
pub use workspace::Workspace;

pub struct Editor {
    editor_theme: egui::Theme,
    id: u64,
    managed: bool,
    workspace_action: Option<workspace::Action>,
    close_cancelled: bool,
    other_paths: Vec<PathBuf>,
    document: Document,
    graph: graph::State,
    path: PathBuf,
    backend: laml_ui::services::Services,
    compiler: worker::Client,
    changed: Instant,
    submitted: Option<u64>,
    retry_after: Option<Instant>,
    rendered: Option<u64>,
    image: Option<(String, Vec<u8>)>,
    error: Option<String>,
    file: laml_ui::file_state::FileState,
    file_ui: file_ui::FileUi,
    allow_close: bool,
    preview: Option<Preview>,
    selection: Selection,
    source_jump: Option<usize>,
    source_ui: source_tools::SourceUi,
    advanced: advanced::Advanced,
    view: view::View,
    #[cfg(not(target_arch = "wasm32"))]
    capture: Option<capture::Capture>,
    workflow: workflow::Workflow,
    recovery: recovery_ui::RecoveryUi,
    structure: structure_ui::StructureUi,
    inspector_tab: inspector_ui::Tab,
    inspector_next_tab: Option<inspector_ui::Tab>,
    property_drafts: std::collections::BTreeMap<String, String>,
    shell_width: f32,
    command_error: Option<String>,
    pending_command: Option<u64>,
    creation: creation::Creation,
    notation_ui: notation_ui::State,
    placement: placement::PlacementUi,
    alignment: Option<alignment::Proposal>,
    deletion: Option<deletion::Proposal>,
    membership: Option<membership::Proposal>,
    remote: remote_ui::Remote,
    relation_drag: Option<(usize, laml::syntax::EndPosition, u64)>,
    inline_text: Option<inline_text::Draft>,
}

impl Editor {
    pub fn new_source(ctx: &egui::Context) -> String {
        editor_theme::new_source(editor_theme::Preferences::load().resolved(ctx))
    }

    pub fn install_appearance(ctx: &egui::Context) {
        appearance::install(ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(source: String, path: Option<PathBuf>) -> Self {
        Self::with_backend(source, path, laml_ui::backend::local())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_backend(
        source: String,
        path: Option<PathBuf>,
        backend: laml_ui::backend::Handle,
    ) -> Self {
        Self::with_services(source, path, laml_ui::services::Services::native(backend))
    }
    pub fn with_services(
        source: String,
        path: Option<PathBuf>,
        backend: laml_ui::services::Services,
    ) -> Self {
        let file = laml_ui::file_state::FileState::new(path, &source);
        let path = file
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.laml"));
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            editor_theme: egui::Theme::Dark,
            id,
            managed: false,
            workspace_action: None,
            close_cancelled: false,
            other_paths: Vec::new(),
            document: Document::new(source),
            graph: graph::State::default(),
            path,
            placement: placement::PlacementUi::with_backend(id, backend.clone()),
            backend: backend.clone(),
            remote: remote_ui::Remote::new(backend.client()),
            file,
            file_ui: file_ui::FileUi::new(backend.clone()),
            compiler: worker::Client::with_client(id, backend.client()),
            changed: Instant::now(),
            submitted: None,
            retry_after: None,
            rendered: None,
            image: None,
            error: None,
            allow_close: false,
            preview: None,
            selection: Selection::default(),
            source_jump: None,
            source_ui: Default::default(),
            advanced: Default::default(),
            view: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            capture: None,
            workflow: workflow::Workflow::new(id, backend.clone()),
            recovery: Default::default(),
            structure: Default::default(),
            inspector_tab: Default::default(),
            inspector_next_tab: None,
            property_drafts: Default::default(),
            shell_width: 0.0,
            command_error: None,
            pending_command: None,
            creation: creation::Creation::default(),
            notation_ui: Default::default(),
            alignment: None,
            relation_drag: None,
            inline_text: None,
            deletion: None,
            membership: None,
        }
    }

    fn publish_preview(&mut self, ctx: &egui::Context, preview: Preview) {
        let revision = self.document.revision();
        self.submitted = Some(revision);
        self.retry_after = None;
        self.selection.retain_diagram(&preview.diagram);
        // Plain text edits may move declarations. Never retarget a selected arrow by line alone.
        self.selection.relations.retain(|line| {
            self.preview
                .as_ref()
                .and_then(|old| old.diagram.relations.iter().find(|r| r.line == *line))
                == preview.diagram.relations.iter().find(|r| r.line == *line)
                && self
                    .preview
                    .as_ref()
                    .and_then(|p| p.diagram.attachments.iter().find(|a| a.line == *line))
                    == preview.diagram.attachments.iter().find(|a| a.line == *line)
        });
        self.property_drafts.clear();
        if let Some((uri, _)) = &self.image {
            ctx.forget_image(uri);
        }
        self.image = Some((
            format!("bytes://preview-{}-{revision}.svg", self.id),
            preview.svg.as_bytes().to_vec(),
        ));
        if self
            .preview
            .as_ref()
            .is_none_or(|old| old.notation != preview.notation)
        {
            self.graph.operator = match preview.notation {
                laml_ui::notation::Notation::Bpmn => "-->",
                laml_ui::notation::Notation::Composite => "==",
                laml_ui::notation::Notation::Communication => "--",
                laml_ui::notation::Notation::Activity => "control",
                laml_ui::notation::Notation::InteractionOverview
                | laml_ui::notation::Notation::State => "-->",
                laml_ui::notation::Notation::Object | laml_ui::notation::Notation::UseCase => "--",
                laml_ui::notation::Notation::Package | laml_ui::notation::Notation::Component => {
                    "..>"
                }
                laml_ui::notation::Notation::Deployment => "--",
                laml_ui::notation::Notation::Profile => "--^>",
                _ => "->",
            }
            .into();
            self.notation_ui.selected = None;
            self.property_drafts.clear();
            self.graph.cancel_gesture();
            self.placement.cancel();
        }
        if let Some((line, Some(id), revision)) = &mut self.notation_ui.selected
            && *revision == self.document.revision()
        {
            if let Some(region) = preview.regions.iter().find(|r| &r.id == id) {
                *line = region.line;
            } else if let Some(node) = preview.diagram.nodes.iter().find(|n| &n.id == id) {
                *line = node.line;
            }
        }
        self.preview = Some(preview);
        self.rendered = Some(revision);
        self.error = None;
        self.source_ui.diagnostic = None;
    }

    fn compile(&mut self, ctx: &egui::Context) {
        loop {
            let (revision, result, command) = match self.compiler.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error =
                        Some("Preview worker stopped. Save source and restart the editor.".into());
                    self.pending_command = None;
                    break;
                }
            };
            let retryable = self.compiler.take_retryable();
            if revision != self.document.revision() {
                continue;
            }
            if command.is_some() {
                self.pending_command = None;
            }
            match result {
                Ok(preview) => {
                    let selected =
                        command
                            .as_ref()
                            .and_then(|command| match &command.expectation {
                                worker::Expectation::RelationEdit(lines)
                                | worker::Expectation::RelationStyle { lines, .. } => {
                                    Some(lines.clone())
                                }
                                worker::Expectation::Relation(id) => Some(
                                    preview
                                        .diagram
                                        .relations
                                        .iter()
                                        .filter(|r| r.name.as_ref() == Some(id))
                                        .map(|r| r.line)
                                        .collect(),
                                ),
                                _ => None,
                            });
                    if let Some(command) = command {
                        self.source_ui.jump = None;
                        if let Err(error) =
                            self.document.apply(revision, command.label, command.edits)
                        {
                            self.command_error = Some(format!("Edit failed: {error:?}"));
                            continue;
                        }
                        if let worker::Expectation::Notation { line, id } = &command.expectation {
                            self.notation_ui.selected =
                                line.map(|line| (line, id.clone(), self.document.revision()));
                        }
                        if let worker::Expectation::Port(id) = &command.expectation {
                            self.selection.click_port(id, false);
                            self.source_jump =
                                laml_ui::port_edit::find(&preview.diagram, id).map(|(_, p)| p.line);
                        }
                        if let worker::Expectation::Node(id)
                        | worker::Expectation::Element { id, .. }
                        | worker::Expectation::Membership { id, .. } = command.expectation
                        {
                            self.selection.click(Some(&id), false);
                            self.source_jump = preview
                                .diagram
                                .nodes
                                .iter()
                                .find(|node| node.id == id)
                                .map(|node| node.line);
                        }
                    }
                    self.publish_preview(ctx, preview);
                    if let Some(lines) = selected {
                        self.selection.ids.clear();
                        self.selection.ports.clear();
                        self.selection.relations = lines;
                    }
                }
                Err(error) if command.is_some() => self.command_error = Some(error.to_string()),
                Err(error) => {
                    if retryable {
                        self.submitted = None;
                        self.retry_after = Some(Instant::now() + Duration::from_secs(2));
                    }
                    self.error = Some(error.to_string());
                    self.source_ui.diagnostic = Some((revision, error));
                }
            }
        }
        if self.submitted != Some(self.document.revision())
            && self.retry_after.is_none_or(|at| Instant::now() >= at)
            && self.changed.elapsed() >= Duration::from_millis(300)
        {
            let revision = self.document.revision();
            match self.compiler.send((
                revision,
                self.document.source().into(),
                self.path.clone(),
                None,
            )) {
                Ok(()) => self.submitted = Some(revision),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl Editor {
    fn document_ui(&mut self, ui: &mut egui::Ui) {
        self.workflow_tick();
        self.compile(ui.ctx());
        self.poll_remote(ui.ctx());
        self.recovery_tick();
        let ctx = ui.ctx().clone();
        self.history_shortcuts(&ctx);
        self.source_ui.extra_drafts.clear();
        self.file_events(&ctx);
        self.shell(ui);
        self.deletion_shortcuts(&ctx);
        self.deletion_window(&ctx);
        self.file_windows(&ctx);
        self.export_windows(&ctx);
        self.structure_window(&ctx);
        self.graph_windows(&ctx);
        self.notation_windows(&ctx);
        self.icon_picker_window(&ctx);
        self.command_error_window(&ctx);
        self.capture_tick(&ctx);
    }
}
