//! Stable semantic roles, with independently tuned light/dark palettes.
use eframe::egui::{self, Color32, FontFamily, FontId, TextFormat};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Plain,
    Section,
    Keyword,
    Id,
    ExplicitId,
    Property,
    Value,
    Text,
    Number,
    Member,
    Type,
    StyleClass,
    Operator,
    Punctuation,
    Comment,
}

impl Role {
    pub(super) fn format(self, dark: bool, normal: Color32) -> TextFormat {
        use Role::*;
        let rgb = match (self, dark) {
            (Plain | Operator, _) => {
                return TextFormat {
                    font_id: FontId::monospace(14.0),
                    color: normal,
                    ..Default::default()
                };
            }
            (Section | Keyword, true) => (197, 157, 245),
            (Section | Keyword, false) => (116, 58, 168),
            (Id | ExplicitId, true) => (114, 180, 251),
            (Id | ExplicitId, false) => (30, 96, 174),
            (Property, true) => (88, 207, 204),
            (Property, false) => (0, 113, 116),
            (Value, true) => (226, 203, 123),
            (Value, false) => (128, 94, 12),
            (Text, true) => (157, 205, 139),
            (Text, false) => (51, 112, 42),
            (Number, true) => (244, 175, 112),
            (Number, false) => (163, 77, 17),
            (Member | StyleClass, true) => (231, 159, 192),
            (Member | StyleClass, false) => (161, 57, 111),
            (Type, true) => (119, 205, 222),
            (Type, false) => (28, 112, 137),
            (Punctuation, true) => (159, 166, 182),
            (Punctuation, false) => (101, 108, 121),
            (Comment, true) => (139, 148, 160),
            (Comment, false) => (106, 114, 125),
        };
        TextFormat {
            font_id: FontId::new(
                14.0,
                if matches!(self, Section | ExplicitId) {
                    FontFamily::Name("source-bold".into())
                } else {
                    FontFamily::Monospace
                },
            ),
            color: Color32::from_rgb(rgb.0, rgb.1, rgb.2),
            ..Default::default()
        }
    }
}

pub(super) fn install_fonts(fonts: &mut egui::FontDefinitions) {
    for (name, bytes) in [
        (
            "source-regular",
            include_bytes!("../../../../../assets/DejaVuSansMono.ttf").as_slice(),
        ),
        (
            "source-bold",
            include_bytes!("../../../../../assets/DejaVuSansMono-Bold.ttf").as_slice(),
        ),
    ] {
        fonts.font_data.insert(
            name.into(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    }
    let mut fallback = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    fallback.insert(0, "source-regular".into());
    fonts
        .families
        .insert(FontFamily::Monospace, fallback.clone());
    fallback.insert(0, "source-bold".into());
    fonts
        .families
        .insert(FontFamily::Name("source-bold".into()), fallback);
}
