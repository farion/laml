//! One recovery chooser per workspace; document stores still protect each live tab.
use super::*;
use laml_ui::recovery::Record;
use std::collections::{BTreeMap, BTreeSet};
#[path = "recovery_details.rs"]
mod details;

pub(super) struct State {
    discovery: recovery_ui::RecoveryUi,
    selected: BTreeSet<PathBuf>,
    details: BTreeMap<PathBuf, details::Details>,
    handled: BTreeSet<PathBuf>,
    open: bool,
    confirm_discard: bool,
    checked: Instant,
    discarding: Vec<Record>,
}
impl Default for State {
    fn default() -> Self {
        let mut discovery = recovery_ui::RecoveryUi::default();
        if let Some(store) = &discovery.store {
            match store.pending() {
                Ok(records) => discovery.records = records,
                Err(error) => discovery.error = Some(error),
            }
        }
        let selected = discovery.records.iter().map(|r| r.file.clone()).collect();
        let details = discovery
            .records
            .iter()
            .map(|r| (r.file.clone(), details::Details::new(&r.source)))
            .collect();
        let open = !discovery.records.is_empty() || discovery.error.is_some();
        let now = Instant::now();
        Self {
            discovery,
            selected,
            details,
            handled: Default::default(),
            open,
            confirm_discard: false,
            checked: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            discarding: Vec::new(),
        }
    }
}
impl State {
    fn collect(&mut self, records: Vec<Record>) {
        let mut added = false;
        for record in records {
            if self.handled.contains(&record.file)
                || self.discovery.records.iter().any(|r| r.file == record.file)
            {
                continue;
            }
            self.selected.insert(record.file.clone());
            self.details
                .insert(record.file.clone(), details::Details::new(&record.source));
            self.discovery.records.push(record);
            added = true;
        }
        if added {
            self.open = true;
        }
    }
    fn chosen(&self) -> Vec<Record> {
        self.discovery
            .records
            .iter()
            .filter(|r| self.selected.contains(&r.file))
            .cloned()
            .collect()
    }
    fn remove(&mut self, file: &PathBuf) {
        self.handled.insert(file.clone());
        self.selected.remove(file);
        self.details.remove(file);
        self.discovery.records.retain(|r| &r.file != file);
        if self.discovery.records.is_empty() && self.discovery.error.is_none() {
            self.open = false;
        }
    }
}
impl Workspace {
    pub(super) fn recovery_button(&mut self, ui: &mut egui::Ui) {
        let count = self.recovery.discovery.records.len();
        if count == 0 {
            return;
        }
        let badge = (count > 0).then(|| {
            ui.painter().layout_no_wrap(
                count.to_string(),
                egui::FontId::new(10.0, egui::FontFamily::Name("bold".into())),
                egui::Color32::WHITE,
            )
        });
        let badge_width = badge
            .as_ref()
            .map_or(0.0, |text| (text.size().x + 8.0).max(16.0));
        let width = (badge_width + 24.0).max(40.0);
        let label = crate::i18n::format(
            "Restore unsaved work ({count})",
            &[("count", count.to_string())],
        );
        let response = ui.add_sized(
            [width, 32.0],
            egui::Button::new(
                egui::RichText::new(egui_phosphor::regular::CLOCK_COUNTER_CLOCKWISE).font(
                    egui::FontId::new(19.0, egui::FontFamily::Name("phosphor".into())),
                ),
            )
            .frame_when_inactive(false),
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &label));
        if let Some(text) = badge {
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    response.rect.right() - badge_width - 1.0,
                    response.rect.top() + 1.0,
                ),
                egui::vec2(badge_width, 16.0),
            );
            ui.painter()
                .rect_filled(rect, 8.0, egui::Color32::from_rgb(205, 43, 57));
            ui.painter().galley(
                rect.center() - text.size() / 2.0,
                text,
                egui::Color32::WHITE,
            );
        }
        if response.on_hover_text(crate::i18n::tr(&(label))).clicked() {
            self.recovery.open = true;
        }
    }
    pub(super) fn open_recovery(&mut self) {
        self.recovery.open = true;
    }
    pub(super) fn recovery_count(&self) -> usize {
        self.recovery.discovery.records.len()
    }
    pub(super) fn recovery_tick(&mut self) {
        self.poll_recovery_discard();
        // Existing editor instances may already hold initial discovery results.
        for tab in &mut self.tabs {
            self.recovery
                .collect(std::mem::take(&mut tab.recovery.records));
            // Backup/restore-completion failures belong to the live document,
            // not to discovery. Keep them visible in its footer without reopening
            // an empty chooser after Restore.
        }
        if self.recovery.checked.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.recovery.checked = Instant::now();
        // Browser discovery is asynchronous and requested once by the store.
        #[cfg(target_arch = "wasm32")]
        if let Some(store) = &self.recovery.discovery.store {
            match store.pending() {
                Ok(records) => self.recovery.collect(records),
                Err(error) => {
                    self.recovery.discovery.error = Some(error);
                    self.recovery.open = true;
                }
            }
        }
    }
    fn restore_recovery(&mut self, ctx: &egui::Context) {
        let chosen = self.recovery.chosen();
        self.recovery.discovery.error = None;
        let mut first = None;
        for record in chosen {
            // A detached buffer protects both saved files and already-open dirty tabs.
            // Its origin keeps relative resources and Save As suggestions meaningful.
            let mut editor = Editor::with_services(String::new(), None, self.backend.clone());
            editor.recovery.records.clear();
            editor.apply_recovery(None, record.clone(), ctx);
            if record.path.is_none() {
                editor.path = PathBuf::from(format!("untitled-{}.laml", self.next_untitled));
                self.next_untitled += 1;
            }
            self.add_tab(editor);
            first.get_or_insert(self.active);
            self.recovery.remove(&record.file);
        }
        if let Some(index) = first {
            self.active = index;
            self.reveal_active = true;
        }
        self.recovery.open = self.recovery.discovery.error.is_some();
    }
    fn discard_recovery(&mut self) {
        self.recovery.discovery.error = None;
        self.recovery.discarding = self.recovery.chosen();
        self.poll_recovery_discard();
    }
    fn poll_recovery_discard(&mut self) {
        for record in self.recovery.discarding.clone() {
            let result = self
                .recovery
                .discovery
                .store
                .as_ref()
                .ok_or_else(|| "Recovery storage is unavailable".to_owned())
                .and_then(|store| store.discard_confirmed(&record));
            match result {
                Ok(true) => {
                    self.recovery.remove(&record.file);
                    self.recovery.discarding.retain(|r| r.file != record.file);
                }
                Ok(false) => {}
                Err(error) => {
                    self.recovery.discovery.error = Some(error);
                    self.recovery.discarding.retain(|r| r.file != record.file);
                }
            }
        }
    }
    pub(super) fn recovery_window(&mut self, ctx: &egui::Context) {
        if !self.recovery.open
            || self.closing_all
            || self.settings_open
            || self.new_diagram.is_some()
        {
            return;
        }
        if self.recovery.confirm_discard {
            self.recovery_discard_window(ctx);
            return;
        }
        let busy = !self.recovery.discarding.is_empty();
        let mut restore = false;
        let mut later = false;
        let mut discard = false;
        let dismissed = dialogs::show(
            ctx,
            "workspace-recovery",
            "Restore unsaved work?",
            520.0,
            |ui| {
                if self.recovery.discovery.records.is_empty() {
                    ui.label(crate::i18n::tr("No unsaved work to restore."));
                } else {
                    ui.label(crate::i18n::tr(
                        "Choose the drafts you want to continue working on.",
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(crate::i18n::format(
                                "{0} of {1} selected",
                                &[
                                    ("0", format!("{}", self.recovery.selected.len())),
                                    ("1", format!("{}", self.recovery.discovery.records.len())),
                                ],
                            ))
                            .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !busy,
                                    egui::Button::new(crate::i18n::tr("Select none")),
                                )
                                .clicked()
                            {
                                self.recovery.selected.clear();
                            }
                            if ui
                                .add_enabled(
                                    !busy,
                                    egui::Button::new(crate::i18n::tr("Select all")),
                                )
                                .clicked()
                            {
                                self.recovery.selected = self
                                    .recovery
                                    .discovery
                                    .records
                                    .iter()
                                    .map(|r| r.file.clone())
                                    .collect();
                            }
                        });
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for (index, record) in
                                self.recovery.discovery.records.iter().enumerate()
                            {
                                ui.push_id(&record.file, |ui| {
                                    let mut selected =
                                        self.recovery.selected.contains(&record.file);
                                    let name = record
                                        .path
                                        .as_ref()
                                        .and_then(|p| p.file_name())
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| {
                                            crate::i18n::format(
                                                "Untitled diagram {count}",
                                                &[("count", (index + 1).to_string())],
                                            )
                                        });
                                    let fill = if selected {
                                        ui.visuals().selection.bg_fill.gamma_multiply(0.22)
                                    } else {
                                        ui.visuals().faint_bg_color
                                    };
                                    egui::Frame::new()
                                        .fill(fill)
                                        .corner_radius(6)
                                        .inner_margin(egui::Margin {
                                            left: 12,
                                            right: 12,
                                            top: 6,
                                            bottom: 8,
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if selected {
                                                ui.visuals()
                                                    .selection
                                                    .stroke
                                                    .color
                                                    .gamma_multiply(0.6)
                                            } else {
                                                ui.visuals().widgets.noninteractive.bg_stroke.color
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.spacing_mut().item_spacing.y = 3.0;
                                            ui.add_enabled(
                                                !busy,
                                                egui::Checkbox::new(
                                                    &mut selected,
                                                    egui::RichText::new(name).size(16.0).strong(),
                                                ),
                                            );
                                            ui.horizontal_top(|ui| {
                                                let inset = ui.spacing().icon_width
                                                    + ui.spacing().icon_spacing
                                                    - ui.spacing().item_spacing.x;
                                                ui.add_space(inset.max(0.0));
                                                ui.vertical(|ui| {
                                                    if let Some(detail) =
                                                        self.recovery.details.get(&record.file)
                                                    {
                                                        ui.horizontal_wrapped(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(detail.kind())
                                                                    .size(13.0)
                                                                    .strong(),
                                                            );
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "· {}",
                                                                    detail.count()
                                                                ))
                                                                .size(12.0)
                                                                .weak(),
                                                            );
                                                        });
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(
                                                                    detail.contents(),
                                                                )
                                                                .size(12.0),
                                                            )
                                                            .wrap(),
                                                        );
                                                    }
                                                    if let Some(parent) = record
                                                        .path
                                                        .as_ref()
                                                        .and_then(|p| p.parent())
                                                        .filter(|p| !p.as_os_str().is_empty())
                                                    {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(
                                                                    parent.display().to_string(),
                                                                )
                                                                .size(11.0)
                                                                .weak(),
                                                            )
                                                            .wrap(),
                                                        );
                                                    }
                                                });
                                            });
                                        });
                                    if selected {
                                        self.recovery.selected.insert(record.file.clone());
                                    } else {
                                        self.recovery.selected.remove(&record.file);
                                    }
                                    ui.add_space(2.0);
                                });
                            }
                        });
                }
                if let Some(error) = &self.recovery.discovery.error {
                    command_errors::details(ui, error);
                }
                ui.add_space(12.0);
                if busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(crate::i18n::tr("Discarding selected changes…"));
                    });
                }
                ui.separator();
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !self.recovery.discovery.records.is_empty() {
                        discard = ui
                            .add_enabled(
                                !busy && !self.recovery.selected.is_empty(),
                                egui::Button::new(crate::i18n::tr("Discard selected…")),
                            )
                            .clicked();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.recovery.discovery.records.is_empty() {
                            restore = ui
                                .add_enabled(
                                    !busy && !self.recovery.selected.is_empty(),
                                    egui::Button::new(crate::i18n::format(
                                        "Restore ({0})",
                                        &[("0", format!("{}", self.recovery.selected.len()))],
                                    ))
                                    .fill(ui.visuals().selection.bg_fill),
                                )
                                .clicked();
                        }
                        later = ui
                            .button(if self.recovery.discovery.records.is_empty() {
                                crate::i18n::tr("Close")
                            } else {
                                crate::i18n::tr("Not now")
                            })
                            .clicked();
                    });
                });
            },
        );
        if dismissed || later {
            self.recovery.open = false;
        }
        if restore {
            self.restore_recovery(ctx);
        }
        if discard {
            self.recovery.confirm_discard = true;
        }
    }
    fn recovery_discard_window(&mut self, ctx: &egui::Context) {
        let mut discard = false;
        let mut cancel = false;
        let dismissed = dialogs::show(
            ctx,
            "discard-unsaved-work",
            "Discard unsaved changes?",
            440.0,
            |ui| {
                ui.label(crate::i18n::tr(
                    "Discard these unsaved changes? Saved files remain unchanged.",
                ));
                ui.label(crate::i18n::format(
                    "Selected diagrams: {0}",
                    &[("0", format!("{}", self.recovery.selected.len()))],
                ));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    discard = ui.button(crate::i18n::tr("Discard changes")).clicked();
                    cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
                });
            },
        );
        if discard {
            self.discard_recovery();
        }
        if discard || cancel || dismissed {
            self.recovery.confirm_discard = false;
        }
    }
}
