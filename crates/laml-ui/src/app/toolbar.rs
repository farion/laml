//! Shared toolbar rule: preserve order, move a trailing suffix into an overflow
//! menu, and always pair icons with action labels inside that menu.
use super::*;

const GAP: f32 = 6.0;
const SEPARATOR: f32 = 9.0;

pub(super) struct Item<'a> {
    pub label: &'a str,
    pub icon: Option<&'a str>,
    pub enabled: bool,
    pub separator_before: bool,
}

impl Item<'_> {
    fn width(&self, ui: &egui::Ui) -> f32 {
        if self.icon.is_some() {
            return ui.spacing().interact_size.y;
        }
        ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(
                    self.label.into(),
                    egui::FontId::proportional(15.0),
                    appearance::color(ui.ctx(), appearance::TEXT),
                )
                .size()
                .x
                + 16.0
        })
        .max(ui.spacing().interact_size.y)
    }
}

fn used(items: &[Item<'_>], widths: &[f32], count: usize) -> f32 {
    widths[..count].iter().sum::<f32>()
        + (1..count)
            .map(|i| {
                GAP + if items[i].separator_before {
                    SEPARATOR + GAP
                } else {
                    0.0
                }
            })
            .sum::<f32>()
}

fn visible(items: &[Item<'_>], widths: &[f32], available: f32, more: f32) -> usize {
    if used(items, widths, items.len()) <= available {
        return items.len();
    }
    (0..items.len())
        .rev()
        .find(|&n| used(items, widths, n) + if n == 0 { more } else { GAP + more } <= available)
        .unwrap_or(0)
}

fn icon_font(height: f32) -> egui::FontId {
    egui::FontId::new(
        if height <= 24.0 { 16.0 } else { 18.0 },
        egui::FontFamily::Name("phosphor".into()),
    )
}

pub(super) fn show(
    ui: &mut egui::Ui,
    items: &[Item<'_>],
    draw: impl FnMut(&mut egui::Ui, usize, bool),
) {
    show_aligned(ui, items, false, draw);
}

pub(super) fn show_right(
    ui: &mut egui::Ui,
    items: &[Item<'_>],
    draw: impl FnMut(&mut egui::Ui, usize, bool),
) {
    show_aligned(ui, items, true, draw);
}

fn show_aligned(
    ui: &mut egui::Ui,
    items: &[Item<'_>],
    right: bool,
    mut draw: impl FnMut(&mut egui::Ui, usize, bool),
) {
    let height = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size = egui::Vec2::splat(height);
    ui.spacing_mut().item_spacing.x = GAP;
    ui.spacing_mut().button_padding = if height <= 24.0 {
        egui::vec2(4.0, 2.0)
    } else {
        egui::vec2(6.0, 6.0)
    };
    let widths: Vec<_> = items.iter().map(|item| item.width(ui)).collect();
    let count = visible(items, &widths, ui.available_width(), height);
    if right {
        let required = used(items, &widths, count)
            + if count < items.len() {
                height + if count > 0 { GAP } else { 0.0 }
            } else {
                0.0
            };
        ui.add_space((ui.available_width() - required).max(0.0));
    }
    for (index, item) in items[..count].iter().enumerate() {
        if index > 0 && item.separator_before {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(SEPARATOR, height), egui::Sense::hover());
            ui.painter().vline(
                rect.center().x,
                rect.top() + 6.0..=rect.bottom() - 6.0,
                egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
            );
        }
        ui.push_id(index, |ui| {
            let rect =
                egui::Rect::from_min_size(ui.cursor().min, egui::vec2(widths[index], height));
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.set_min_width(widths[index]);
                draw(ui, index, false)
            });
        });
    }
    if count < items.len() {
        let more = egui::RichText::new(egui_phosphor::regular::DOTS_THREE).font(icon_font(height));
        let response = ui.add_sized(egui::Vec2::splat(height), egui::Button::new(more));
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(200.0);
            for (index, item) in items.iter().enumerate().skip(count) {
                if index > count && item.separator_before {
                    ui.separator();
                }
                ui.push_id(index, |ui| draw(ui, index, true));
            }
        });
        response.on_hover_text("More actions");
    }
}

pub(super) fn button(ui: &mut egui::Ui, item: &Item<'_>, in_menu: bool) -> egui::Response {
    selected_button(ui, item, in_menu, None)
}

pub(super) fn selected_button(
    ui: &mut egui::Ui,
    item: &Item<'_>,
    in_menu: bool,
    selected: Option<bool>,
) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    let color = if item.enabled {
        appearance::color(ui.ctx(), appearance::TEXT)
    } else {
        appearance::color(ui.ctx(), appearance::MUTED)
    };
    let text: egui::WidgetText = if in_menu {
        let mut job = egui::text::LayoutJob::default();
        if let Some(icon) = item.icon {
            job.append(
                &format!("{icon}  "),
                0.0,
                egui::TextFormat {
                    font_id: icon_font(height),
                    color,
                    ..Default::default()
                },
            );
        }
        job.append(
            item.label,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(15.0),
                color,
                ..Default::default()
            },
        );
        job.into()
    } else if let Some(icon) = item.icon {
        egui::RichText::new(icon).font(icon_font(height)).into()
    } else {
        item.label.into()
    };
    let mut button = egui::Button::new(text).min_size(egui::vec2(
        if in_menu {
            ui.available_width()
        } else {
            item.width(ui)
        },
        height,
    ));
    if let Some(selected) = selected {
        button = button.selected(selected);
    }
    ui.add_enabled(item.enabled, button)
        .on_hover_text(item.label)
}
