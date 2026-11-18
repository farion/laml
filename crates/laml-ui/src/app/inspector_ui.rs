//! Flat inspector categories and consistent, confirmation-free field editing.
use super::*;

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum Tab {
    #[default]
    General,
    Style,
}

pub(super) const INSET: i8 = 16;
pub(super) const KEY_WIDTH: f32 = 94.0;

/// Categories span the pane; only attribute rows have horizontal padding.
pub(super) fn content<R>(ui: &mut egui::Ui, draw: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(INSET, 0))
        .show(ui, draw)
        .inner
}

pub(super) fn row<R>(ui: &mut egui::Ui, key: &str, draw: impl FnOnce(&mut egui::Ui) -> R) -> R {
    content(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            caption(ui, key, KEY_WIDTH).on_hover_text(crate::i18n::tr(key));
            let width = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(width, appearance::FIELD_HEIGHT),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_min_width(width);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    draw(ui)
                },
            )
            .inner
        })
        .inner
    })
}

pub(super) fn category(ui: &mut egui::Ui, title: &str) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        0,
        appearance::color(ui.ctx(), appearance::HEADER).gamma_multiply(0.45),
    );
    for y in [rect.top(), rect.bottom()] {
        ui.painter().hline(
            rect.x_range(),
            y,
            egui::Stroke::new(
                1.0,
                appearance::color(ui.ctx(), appearance::BORDER).gamma_multiply(0.7),
            ),
        );
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(f32::from(INSET), 0.0),
        egui::Align2::LEFT_CENTER,
        crate::i18n::tr(title),
        egui::FontId::proportional(13.0),
        appearance::color(ui.ctx(), appearance::MUTED),
    );
}

pub(super) fn caption(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, appearance::FIELD_HEIGHT),
        egui::Sense::hover(),
    );
    let mut label = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    label.add(egui::Label::new(crate::i18n::tr(text)).truncate());
    response
}

pub(super) struct Field {
    pub response: egui::Response,
    pub commit: bool,
}
pub(super) fn field(
    ui: &mut egui::Ui,
    key: impl std::hash::Hash + std::fmt::Debug,
    draft: &mut String,
    multiline: bool,
) -> Field {
    mixed_field(ui, key, draft, multiline, false)
}

pub(super) fn mixed_text() -> egui::RichText {
    egui::RichText::new(crate::i18n::tr("Mixed"))
        .weak()
        .italics()
}

pub(super) fn value_text(value: &str) -> egui::RichText {
    if value == "Mixed" {
        mixed_text()
    } else {
        egui::RichText::new(value)
    }
}

/// A notation/style choice is UI text; endpoint names use `value_text` instead.
pub(super) fn choice_text(value: &str) -> egui::RichText {
    if value == "Mixed" {
        mixed_text()
    } else {
        egui::RichText::new(crate::i18n::tr(value))
    }
}

pub(super) fn mixed_field(
    ui: &mut egui::Ui,
    key: impl std::hash::Hash + std::fmt::Debug,
    draft: &mut String,
    multiline: bool,
    mixed: bool,
) -> Field {
    let id = ui.make_persistent_id(key);
    let modified_id = id.with("mixed-field-modified");
    let focused = ui.memory(|m| m.has_focus(id));
    let shift_enter = multiline
        && ui.memory(|m| m.has_focus(id))
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter));
    if shift_enter {
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        let cursor = state
            .cursor
            .char_range()
            .map(|c| {
                let range = c.as_sorted_char_range();
                range.start.0..range.end.0
            })
            .unwrap_or(draft.chars().count()..draft.chars().count());
        let byte = |n| draft.char_indices().nth(n).map_or(draft.len(), |(i, _)| i);
        let range = byte(cursor.start)..byte(cursor.end);
        draft.replace_range(range, "\n");
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cursor.start + 1),
            )));
        state.store(ui.ctx(), id);
    }
    let rows = draft.split('\n').count().clamp(1, 10);
    let edit = if multiline && draft.contains('\n') {
        egui::TextEdit::multiline(draft)
            .desired_rows(rows)
            .return_key(None)
    } else {
        egui::TextEdit::singleline(draft)
    };
    let edit = if mixed && !focused {
        edit.hint_text(mixed_text())
    } else {
        edit
    };
    let response = ui.add(
        edit.id(id)
            .margin(egui::Margin::symmetric(10, 7))
            .align(egui::Align2::LEFT_TOP)
            .desired_width(ui.available_width()),
    );
    let response = response.on_hover_ui(|ui| {
        hover_help::show(
            ui,
            if multiline {
                crate::i18n::tr("Enter: confirm · Shift+Enter: new line")
            } else {
                crate::i18n::tr("Enter: confirm")
            },
        )
    });
    let enter = response.has_focus()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let modified = ui.ctx().data_mut(|data| {
        let changed = data.get_temp::<bool>(modified_id).unwrap_or(false)
            || response.changed()
            || shift_enter;
        data.insert_temp(modified_id, changed);
        changed
    });
    let commit = ui.is_enabled()
        && !dialogs::blocked(ui.ctx())
        && !shift_enter
        && (enter || response.lost_focus())
        && (!mixed || (modified && !draft.is_empty()));
    if commit || response.lost_focus() {
        ui.ctx().data_mut(|data| data.remove::<bool>(modified_id));
    }
    if enter {
        response.surrender_focus();
    }
    Field { response, commit }
}
