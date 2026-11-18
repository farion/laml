//! Hover-local insertion controls for every shared node-edge notation.
use super::*;
use laml::syntax::Placement;
use laml_ui::graph_insert::{self, Item};
mod empty;
mod geometry;
#[cfg(test)]
mod tests;

pub(super) struct Menu {
    revision: u64,
    from: String,
    side: Placement,
    at: egui::Pos2,
    expanded: bool,
    inside: bool,
    choices: Vec<Item>,
}
fn pluses(rect: egui::Rect) -> [(Placement, egui::Pos2); 4] {
    [
        (Placement::Left, rect.left_center() + egui::vec2(-27.0, 0.0)),
        (
            Placement::Right,
            rect.right_center() + egui::vec2(27.0, 0.0),
        ),
        (Placement::Above, rect.center_top() + egui::vec2(0.0, -27.0)),
        (
            Placement::Below,
            rect.center_bottom() + egui::vec2(0.0, 27.0),
        ),
    ]
}
pub(in crate::app) fn icon(ui: &egui::Ui, at: egui::Pos2, hot: bool) {
    use egui::emath::GuiRounding as _;
    let color = appearance::color(ui.ctx(), appearance::ACCENT);
    let rect = egui::Rect::from_center_size(at, egui::Vec2::splat(20.0))
        .round_to_pixels(ui.pixels_per_point());
    ui.painter()
        .rect_filled(rect, 4.0, appearance::color(ui.ctx(), appearance::PANEL));
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(if hot { 2.0 } else { 1.0 }, color),
        egui::StrokeKind::Inside,
    );
    static SVG: std::sync::OnceLock<egui::load::Bytes> = std::sync::OnceLock::new();
    let bytes = SVG.get_or_init(|| {
        include_str!("../../../../assets/phosphor/regular/plus.svg")
            .replace("currentColor", "#ffffff")
            .into_bytes()
            .into()
    });
    let ppp = ui.pixels_per_point();
    let edge = |extent: f32| {
        let parity = (extent * ppp).round().rem_euclid(2.0);
        (((15.0 * ppp - parity) / 2.0).round() * 2.0 + parity) / ppp
    };
    egui::Image::from_bytes("bytes://graph-inline-phosphor-plus.svg", bytes.clone())
        .tint(color)
        .paint_at(
            ui,
            egui::Rect::from_center_size(
                rect.center(),
                egui::vec2(edge(rect.width()), edge(rect.height())),
            ),
        );
}
fn menu_icon(item: Item) -> Option<&'static str> {
    use egui_phosphor::regular::*;
    use laml_ui::element_edit::ElementKind as K;
    // No generic placeholders: distinct constructs must not share an icon.
    match item {
        Item::Element(K::Rectangle) => Some(SQUARE),
        Item::Element(K::Ellipse) => Some(CIRCLE),
        Item::Element(K::Diamond) => Some(DIAMOND),
        Item::Element(K::Note) => Some(NOTE),
        Item::Element(K::Package) => Some(FOLDER),
        Item::Element(K::Actor) => Some(USER),
        _ => None,
    }
}
fn label(ui: &egui::Ui, text: &str, icon: Option<&str>) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let color = ui.visuals().text_color();
    let mut text_indent = 28.0;
    if let Some(icon) = icon {
        let prefix = format!("{icon}  ");
        let font = egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into()));
        let width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(prefix.clone(), font.clone(), color)
                .size()
                .x
        });
        text_indent -= width;
        job.append(
            &prefix,
            0.0,
            egui::TextFormat {
                font_id: font,
                color,
                ..Default::default()
            },
        );
    }
    job.append(
        crate::i18n::tr(text),
        text_indent,
        egui::TextFormat {
            font_id: egui::FontId::proportional(15.0),
            color,
            ..Default::default()
        },
    );
    job
}
impl Editor {
    pub(super) fn graph_insert_overlay(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        navigation: bool,
    ) {
        if !self.can_edit_visually() || navigation || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.graph.insertion = None;
            return;
        }
        if self.graph.insertion.is_none()
            && !self.graph.dialog_open()
            && self.graph.gesture.is_none()
            && self.placement.drag.is_none()
        {
            self.empty_container_pluses(ui, canvas);
            let pointer = ui
                .input(|i| i.pointer.hover_pos())
                .filter(|p| ui.clip_rect().contains(*p));
            let preview = self.preview.as_ref().unwrap();
            let node = self.graph_control_node(preview, canvas, pointer);
            if let Some(node) = node {
                let from = node.id.clone();
                let positions = geometry::positions(preview, canvas, node);
                self.graph.hover_node = Some(from.clone());
                for (side, at) in positions {
                    let hit = egui::Rect::from_center_size(at, egui::Vec2::splat(22.0));
                    let r = ui.interact(
                        hit,
                        ui.make_persistent_id(("graph-inline-plus", &from, format!("{side:?}"))),
                        egui::Sense::click(),
                    );
                    icon(ui, at, r.hovered());
                    if r.clicked() {
                        let choices = graph_insert::items(self.document.source())
                            .into_iter()
                            .filter(|item| {
                                graph_insert::plan(
                                    self.document.source(),
                                    &from,
                                    side,
                                    *item,
                                    &self.graph.operator,
                                )
                                .is_ok()
                            })
                            .collect();
                        self.graph.insertion = Some(Menu {
                            revision: self.document.revision(),
                            from: from.clone(),
                            side,
                            at,
                            expanded: false,
                            inside: false,
                            choices,
                        });
                    }
                }
            }
        }
        self.graph_insert_menu(ui);
    }
    fn graph_insert_menu(&mut self, ui: &mut egui::Ui) {
        let Some(mut menu) = self.graph.insertion.take() else {
            return;
        };
        if menu.revision != self.document.revision() {
            return;
        }
        let mut chosen = None;
        let mut open = true;
        let anchor = egui::Rect::from_center_size(menu.at, egui::Vec2::splat(20.0));
        icon(ui, menu.at, true);
        egui::Popup::new(
            ui.make_persistent_id(("graph-insert-menu", self.id)),
            ui.ctx().clone(),
            anchor,
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .style(egui::containers::menu::menu_style)
        .gap(0.0)
        .open_bool(&mut open)
        .show(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            let frame = egui::Frame::popup(ui.style());
            let height = (ui.ctx().content_rect().height() - frame.total_margin().sum().y).max(1.0);
            // An Area remembers its compact size. Let “More…” grow beyond that
            // cached viewport; only the actual screen bounds should cause scrolling.
            ui.set_max_height(height);
            egui::ScrollArea::vertical()
                .max_height(height)
                .show(ui, |ui| {
                    for (i, item) in menu.choices.iter().enumerate() {
                        if !menu.expanded && i >= 6 {
                            break;
                        }
                        if ui
                            .add(
                                egui::Button::new(label(ui, item.title(), menu_icon(*item)))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .clicked()
                        {
                            chosen = Some(*item);
                        }
                    }
                    if !menu.expanded && menu.choices.len() > 6 {
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(2.0);
                        if ui
                            .add(
                                egui::Button::new(label(
                                    ui,
                                    "More…",
                                    Some(egui_phosphor::regular::DOTS_THREE),
                                ))
                                .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .clicked()
                        {
                            menu.expanded = true;
                        }
                    }
                    if menu.choices.is_empty() {
                        ui.label(crate::i18n::tr("No applicable elements"));
                    }
                });
        });
        if let Some(item) = chosen {
            let plan = if menu.inside {
                graph_insert::plan_inside(self.document.source(), &menu.from, item)
            } else {
                graph_insert::plan(
                    self.document.source(),
                    &menu.from,
                    menu.side,
                    item,
                    &self.graph.operator,
                )
            };
            match plan {
                Ok(p) => self.submit_command(Command {
                    edits: p.edits,
                    label: if menu.inside {
                        "Insert container content"
                    } else {
                        "Insert connected element"
                    }
                    .into(),
                    expectation: p.id.map_or(
                        worker::Expectation::Notation {
                            line: None,
                            id: None,
                        },
                        worker::Expectation::Node,
                    ),
                }),
                Err(e) => self.error_toast(e),
            }
        } else if open {
            self.graph.insertion = Some(menu);
        }
    }
}
