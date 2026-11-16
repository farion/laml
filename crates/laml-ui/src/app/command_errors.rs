//! Human-readable reasons first; unchanged diagnostics remain available below.
use super::*;

pub(super) struct Explanation {
    pub reason: String,
    pub suggestion: &'static str,
}

/// Present known compiler/editor failures without guessing at unknown causes.
pub(super) fn explain(error: &str) -> Explanation {
    let message = error
        .strip_prefix("Line ")
        .and_then(|text| text.split_once(':'))
        .filter(|(line, _)| !line.is_empty() && line.chars().all(|c| c.is_ascii_digit()))
        .map_or(error, |(_, text)| text.trim());
    let subject = message
        .split_once('\'')
        .and_then(|(_, rest)| rest.split_once('\''))
        .map_or(crate::i18n::tr("this element"), |(id, _)| id);
    let (reason, suggestion) = if message.contains("visible relation targets hidden") {
        (
            format!("{subject} cannot be hidden because a visible arrow is still connected to it."),
            "Hide the connected arrows as well, or leave this element visible.",
        )
    } else if (message.contains("mandatory layout")
        || message.contains("mandatory alignment")
        || message.contains("mandatory end-label layout")
        || message.contains("mandatory label layout"))
        && message.contains("targets hidden")
    {
        (
            format!("{subject} cannot be hidden because a layout rule still requires it."),
            "Remove or adjust the referencing layout rule before hiding the element.",
        )
    } else if message.contains("visible edge attachment targets hidden") {
        (
            format!("A visible attachment still depends on {subject}, so it cannot be hidden."),
            "Hide or remove the attachment as well, or keep its target visible.",
        )
    } else if message.contains("expected an unadorned numeric value") {
        (
            "This field needs a number, but the entered value is not numeric.".into(),
            "Enter a number such as 14 or 2.5, without a unit such as px.",
        )
    } else if message.contains("style size must be finite and nonnegative") {
        (
            "This size is outside the allowed range.".into(),
            "Enter a finite, non-negative size. Font size must be greater than zero.",
        )
    } else if message.contains("shadowed") || message.contains("transformed by another rule") {
        (
            "Another style rule overrides or changes the requested value.".into(),
            "Adjust the higher-priority style rule, or reset the local override.",
        )
    } else if message.contains("icons requires icon-set: phosphor") {
        (
            "Icon rendering needs the Phosphor icon set, but it is missing for this element."
                .into(),
            "Use text rendering, or add icon-set: phosphor to the relevant style rule.",
        )
    } else if message.contains("Document changed")
        || message.contains("Document or selection changed")
        || message.contains("out of date")
        || message.contains("StaleRevision")
    {
        (
            "The document or selection changed while this action was being prepared.".into(),
            "Wait for the updated preview, then repeat the action or reopen its dialog.",
        )
    } else if message.contains("worker stopped") {
        (
            "The preview service is no longer responding, so the change could not be checked."
                .into(),
            "Save your source and restart the editor.",
        )
    } else {
        // Keep the actual cause for unfamiliar failures; do not invent an explanation.
        (
            message.to_owned(),
            "Review the reported cause and correct the affected value or rule.",
        )
    };
    Explanation { reason, suggestion }
}

pub(super) fn details(ui: &mut egui::Ui, error: &str) {
    let explanation = explain(error);
    ui.add(
        egui::Label::new(egui::RichText::new(crate::i18n::message(&(explanation.reason))).strong())
            .wrap(),
    );
    ui.add(egui::Label::new(crate::i18n::message(explanation.suggestion)).wrap());
    ui.separator();
    ui.colored_label(
        appearance::color(ui.ctx(), appearance::MUTED),
        crate::i18n::tr("LAML-Error"),
    );
    ui.add(
        egui::Label::new(
            egui::RichText::new(error)
                .monospace()
                .size(12.0)
                .color(appearance::color(ui.ctx(), appearance::MUTED)),
        )
        .wrap(),
    );
}

impl Editor {
    pub(super) fn command_error_window(&mut self, ctx: &egui::Context) {
        let Some(error) = self.command_error.clone() else {
            return;
        };
        // An open chooser owns its inline validation instead of stacking modals.
        if self.graph.icon_picker.is_some() {
            return;
        }
        let mut close = false;
        let dismissed = dialogs::show(
            ctx,
            ("edit-error", self.id),
            "Change could not be applied",
            480.0,
            |ui| {
                details(ui, &error);
                close = ui.button(crate::i18n::tr("Close")).clicked();
            },
        );
        if dismissed || close {
            self.command_error = None;
            self.graph.report_errors = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explains_hidden_endpoints_layout_and_attachments_without_losing_the_target() {
        let explanation = explain("Line 4: visible relation targets hidden node or member 'B'");
        assert_eq!(
            explanation.reason,
            "B cannot be hidden because a visible arrow is still connected to it."
        );
        assert!(explanation.suggestion.contains("Hide the connected arrows"));
        assert!(
            explain("Line 4: visible relation targets hidden member 'A::id'")
                .reason
                .contains("A::id")
        );
        assert!(
            explain("Line 7: mandatory alignment targets hidden node 'B'")
                .reason
                .contains("layout rule")
        );
        assert!(
            explain("Line 8: visible edge attachment targets hidden relation 'r'")
                .reason
                .contains("attachment")
        );
    }
    #[test]
    fn explains_invalid_values_and_preserves_unfamiliar_causes() {
        assert!(
            explain("expected an unadorned numeric value")
                .reason
                .contains("not numeric")
        );
        assert!(
            explain("Local relation style is shadowed or transformed")
                .reason
                .contains("Another style rule")
        );
        assert!(
            explain("Document changed. Source unchanged.")
                .reason
                .contains("document or selection changed")
        );
        assert_eq!(
            explain("Line 9: unsupported custom constraint").reason,
            "unsupported custom constraint"
        );
        assert_eq!(
            explain("Line not-a-number: a new failure").reason,
            "Line not-a-number: a new failure"
        );
    }
    #[test]
    fn dialog_puts_the_reason_and_remedy_above_the_unchanged_technical_diagnostic() {
        let (mut editor, ctx) = super::super::inspector_tests::ready();
        let error = "Line 4: visible relation targets hidden node or member 'B'";
        editor.command_error = Some(error.into());
        let source = editor.document.source().to_owned();
        let mut out = None;
        for _ in 0..3 {
            let mut frame = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ui| editor.command_error_window(ui.ctx()),
            );
            frame.textures_delta.clear();
            out = Some(frame);
        }
        let out = out.unwrap();
        let y = |label: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => {
                        assert!(
                            s.clip_rect
                                .contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size()))
                        );
                        Some(t.pos.y)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing dialog text: {label}"))
        };
        let explanation = explain(error);
        assert!(y(&explanation.reason) < y(explanation.suggestion));
        assert!(y(explanation.suggestion) < y("LAML-Error"));
        assert!(y("LAML-Error") < y(error));
        assert!(dialogs::blocked(&ctx));
        assert_eq!(editor.document.source(), source);
        assert_eq!(editor.document.revision(), 0);
    }
}
