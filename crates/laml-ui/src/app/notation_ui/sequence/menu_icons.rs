//! Stable semantic icons shared by insertion, wrapping and object-action menus.
use egui_phosphor::regular::*;
const ICONS: &[(&str, &str)] = &[
    ("Participant", USER),
    ("Activation", RECTANGLE),
    ("Destroy participant", X_CIRCLE),
    ("Alternative", GIT_FORK),
    ("Loop", ARROWS_CLOCKWISE),
    ("Optional fragment", QUESTION),
    ("Parallel fragment", COLUMNS),
    ("Critical fragment", LOCK),
    ("Break fragment", HAND_PALM),
    ("Negative fragment", PROHIBIT),
    ("Assertion fragment", CHECK_SQUARE),
    ("Strict fragment", LIST_NUMBERS),
    ("Weak sequencing fragment", FLOW_ARROW),
    ("Interaction reference", LINK),
    ("Participant order", ARROWS_HORIZONTAL),
    ("Ignore fragment", EYE_SLASH),
    ("Consider fragment", EYE),
    ("Nested fragment shorthand", STACK),
    ("Named fragment", TAG),
    ("Note", NOTE),
    ("Style rule", PAINT_BRUSH),
    ("Continuation", BOOKMARK_SIMPLE),
    ("Time observation", CLOCK),
    ("Time constraint", CLOCK_COUNTDOWN),
    ("State annotation", TOGGLE_LEFT),
    ("Invariant", SHIELD_CHECK),
    ("Action annotation", LIGHTNING),
    ("Duration observation", TIMER),
    ("Duration constraint", HOURGLASS_MEDIUM),
    ("General ordering", SORT_ASCENDING),
    ("Coregion", INTERSECT),
    ("Inline decomposition", TREE_STRUCTURE),
    ("Interaction reference with gates", DOOR_OPEN),
    ("Occurrence order", ARROWS_VERTICAL),
    ("Frame coverage", BOUNDING_BOX),
    ("Note placement", NOTE_PENCIL),
    ("Continuation placement", ALIGN_CENTER_HORIZONTAL),
    ("Synchronous message", ARROW_RIGHT),
    ("Asynchronous message", PAPER_PLANE_RIGHT),
    ("Reply", ARROW_U_UP_LEFT),
    ("Creation message", USER_PLUS),
    ("Deletion message", USER_MINUS),
    ("Fragments", SQUARES_FOUR),
    ("Annotations", TEXT_T),
    ("Layout", RULER),
    ("Wrap in fragment", FRAME_CORNERS),
    ("Add branch", GIT_BRANCH),
    ("Elements…", LIST_BULLETS),
    ("More…", CARET_DOWN),
    ("Delete…", TRASH),
];
pub(super) fn icon(title: &str) -> &'static str {
    ICONS
        .iter()
        .find(|(name, _)| *name == title)
        .map_or(QUESTION_MARK, |(_, icon)| *icon)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_construct_and_message_type_has_an_explicit_semantic_icon() {
        let titles = crate::app::notation_ui::sequence::recipes()
            .into_iter()
            .map(|r| r.title)
            .chain(
                laml_ui::notation::Notation::Sequence
                    .relation_types()
                    .into_iter()
                    .map(|r| r.title),
            );
        for title in titles {
            assert!(
                ICONS.iter().any(|(name, _)| *name == title),
                "Missing icon for {title}"
            );
            assert_ne!(
                icon(title),
                PLUS,
                "Adding a construct must show its type, not the generic add action"
            );
        }
    }
    #[test]
    fn different_meanings_never_reuse_the_same_icon() {
        let mut used = std::collections::HashMap::new();
        for (title, glyph) in ICONS {
            assert!(
                used.insert(glyph, title).is_none(),
                "{title} duplicates an icon already assigned to another meaning"
            );
            assert_eq!(glyph.chars().count(), 1);
        }
    }
    #[test]
    fn menu_text_uses_the_registered_phosphor_glyph_for_every_entry() {
        let ctx = eframe::egui::Context::default();
        crate::app::appearance::install(&ctx);
        let mut output = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            for (title, glyph) in ICONS {
                let text = super::super::actions::menu_text(ui, title);
                assert!(text.text.starts_with(glyph));
                assert_eq!(
                    text.sections[0].format.font_id.family,
                    eframe::egui::FontFamily::Name("phosphor".into())
                );
            }
        });
        output.textures_delta.clear();
    }
}
