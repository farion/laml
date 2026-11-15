//! Shared relation picker with compiler-rendered swatches and textual descriptions.
use super::*;

// Same order as relation_edit::OPERATORS; default choices are matched by operator.
const SWATCHES: [&[u8]; 20] = [
    include_bytes!("../../assets/relation-types/00.svg"),
    include_bytes!("../../assets/relation-types/01.svg"),
    include_bytes!("../../assets/relation-types/02.svg"),
    include_bytes!("../../assets/relation-types/03.svg"),
    include_bytes!("../../assets/relation-types/04.svg"),
    include_bytes!("../../assets/relation-types/05.svg"),
    include_bytes!("../../assets/relation-types/06.svg"),
    include_bytes!("../../assets/relation-types/07.svg"),
    include_bytes!("../../assets/relation-types/08.svg"),
    include_bytes!("../../assets/relation-types/09.svg"),
    include_bytes!("../../assets/relation-types/10.svg"),
    include_bytes!("../../assets/relation-types/11.svg"),
    include_bytes!("../../assets/relation-types/12.svg"),
    include_bytes!("../../assets/relation-types/13.svg"),
    include_bytes!("../../assets/relation-types/14.svg"),
    include_bytes!("../../assets/relation-types/15.svg"),
    include_bytes!("../../assets/relation-types/16.svg"),
    include_bytes!("../../assets/relation-types/17.svg"),
    include_bytes!("../../assets/relation-types/18.svg"),
    include_bytes!("../../assets/relation-types/19.svg"),
];

fn description(operator: &str) -> &str {
    match laml_ui::relation_edit::relation_type(operator) {
        "->" => "Arrow",
        "<->" => "Bidirectional",
        "--" => "Solid line",
        ".." => "Dashed line",
        "..>" => "Dashed arrow",
        "--^>" => "Filled triangle",
        "--|>" => "Hollow triangle",
        "..|>" => "Dashed triangle",
        "--*" => "Filled diamond",
        "--o" => "Hollow diamond",
        "*-->" => "Filled diamond + arrow",
        "o-->" => "Hollow diamond + arrow",
        _ => operator,
    }
}

fn swatch(ui: &egui::Ui, operator: &str, rect: egui::Rect) {
    if let Some(index) = laml_ui::relation_edit::OPERATORS.iter().position(|op| *op == operator) {
        let dark = ui.visuals().dark_mode;
        let svg = if dark {
            SWATCHES[index].to_vec()
        } else {
            String::from_utf8_lossy(SWATCHES[index])
                .replace("#aebccf", "#384c66")
                .replace("#202d37", "#f4f6fa").into_bytes()
        };
        egui::Image::from_bytes(
            format!("bytes://relation-type-v3-{dark}-{index}.svg"), svg,
        ).paint_at(ui, rect);
    }
}

pub(super) fn combo(
    ui: &mut egui::Ui,
    id: &str,
    operator: &mut String,
    operators: &[&str],
    width: f32,
) {
    let has_swatch = laml_ui::relation_edit::OPERATORS.contains(&operator.as_str());
    let font = egui::FontId::proportional(13.0);
    let text_width = ui.fonts_mut(|fonts| operators.iter().map(|op| {
        fonts.layout_no_wrap(description(op).into(), font.clone(), appearance::color(ui.ctx(), appearance::TEXT)).size().x
    }).fold(0.0_f32, f32::max));
    let popup_width = (92.0 + text_width + 10.0)
        .min((ui.ctx().content_rect().width() - 60.0).max(120.0));
    let response = ui.push_id(id, |ui| {
        ui.add_sized([width, ui.spacing().interact_size.y], egui::Button::new(""))
    }).inner;
    let name_rect = egui::Rect::from_min_max(
        response.rect.left_top() + egui::vec2(if has_swatch { 92.0 } else { 10.0 }, 0.0),
        response.rect.right_bottom() - egui::vec2(26.0, 0.0),
    );
    let text = if operator == "Mixed" { inspector_ui::mixed_text() }
        else { egui::RichText::new(description(operator)) };
    let mut name = ui.new_child(egui::UiBuilder::new().max_rect(name_rect)
        .layout(egui::Layout::left_to_right(egui::Align::Center)));
    name.set_clip_rect(name_rect.intersect(ui.clip_rect()));
    name.add(egui::Label::new(text.size(13.0))
        .truncate()
        .selectable(false)
        .sense(egui::Sense::hover()));
    let center = response.rect.right_center() - egui::vec2(13.0, 0.0);
    ui.painter().add(egui::Shape::convex_polygon(vec![
        center + egui::vec2(-4.0, -2.0), center + egui::vec2(4.0, -2.0),
        center + egui::vec2(0.0, 3.0),
    ], ui.visuals().text_color(), egui::Stroke::NONE));
    egui::Popup::menu(&response)
        .width(popup_width + 18.0)
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height((operators.len() as f32 * 30.0 + 16.0)
                    .min((ui.ctx().content_rect().height() - 80.0).max(80.0)))
                .show(ui, |ui| {
                for &value in operators {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(popup_width, 26.0), egui::Sense::click(),
                    );
                    if operator == value || response.hovered() {
                        ui.painter().rect_filled(rect, 3.0, if operator == value {
                            ui.visuals().selection.bg_fill
                        } else {
                            ui.visuals().widgets.hovered.bg_fill
                        });
                    }
                    swatch(ui, value, egui::Rect::from_center_size(
                        rect.left_center() + egui::vec2(44.0, 0.0),
                        egui::vec2(80.0, 20.0),
                    ));
                    ui.painter().text(
                        rect.left_center() + egui::vec2(92.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        description(value),
                        egui::FontId::proportional(13.0), appearance::color(ui.ctx(), appearance::TEXT),
                    );
                    if response.clicked() {
                        *operator = value.into();
                        ui.close();
                    }
                }
            });
        });
    if has_swatch {
        swatch(ui, operator, egui::Rect::from_center_size(
            response.rect.left_center() + egui::vec2(46.0, 0.0),
            egui::vec2(80.0, 20.0),
        ));
    }
    response.on_hover_text(description(operator));
}

impl Editor {
    pub(super) fn relation_direction_control(
        &mut self, ui: &mut egui::Ui, operators: &[String], lines: &[usize],
    ) {
        use laml_ui::relation_edit;
        let directions: Vec<_> = lines.iter()
            .filter_map(|line| relation_edit::direction(self.document.source(), *line).ok())
            .collect();
        if directions.len() != lines.len() || directions.is_empty() {
            return;
        }
        let active = operators.iter().all(|op| relation_edit::directional(relation_edit::relation_type(op)));
        let first = directions[0].0 <= directions[0].1;
        let original = if directions.iter().all(|(from, to)| (from <= to) == first) {
            Some(first)
        } else {
            None
        };
        let mut chosen = original;
        // Names make the direction independent of canvas placement and remain
        // stable even when a type needs its endpoint expressions exchanged.
        let (a, b) = &directions[0];
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        let same_ends = directions.iter().all(|(a, b)|
            (a == low && b == high) || (a == high && b == low));
        let labels = if same_ends {
            [format!("{low} → {high}"), format!("{high} → {low}")]
        } else {
            ["Forward (A → Z)".into(), "Reverse (Z → A)".into()]
        };
        ui.add_enabled_ui(active, |ui| {
            inspector_ui::row(ui, "Direction", |ui| {
                let text = if !active { egui::RichText::new("Not directional") }
                    else { match chosen {
                        Some(true) => egui::RichText::new(&labels[0]),
                        Some(false) => egui::RichText::new(&labels[1]),
                        None => inspector_ui::mixed_text(),
                    }};
                egui::ComboBox::from_id_salt("relation-direction")
                    .width(ui.available_width())
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut chosen, Some(true), &labels[0]);
                        ui.selectable_value(&mut chosen, Some(false), &labels[1]);
                    });
            });
        });
        if chosen != original && let Some(ascending) = chosen {
            self.relation_edits("Change relation direction", |source, line| {
                relation_edit::change_direction(source, line, ascending)
            });
        }
    }

    pub(super) fn arrow_type_control(&mut self, ui: &mut egui::Ui) {
        let enabled = self.can_edit_visually()
            && !self.graph.dialog_open()
            && self.graph.gesture.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            let mut kind = laml_ui::relation_edit::relation_type(&self.graph.operator).to_owned();
            combo(ui, "new-arrow-type", &mut kind,
                laml_ui::relation_edit::TYPES, 270.0);
            self.graph.operator = kind;
        });
    }
}
