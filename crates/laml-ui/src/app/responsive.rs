//! Presentation-only responsive state; document, undo and canvas state stay intact.
use super::*;
#[cfg(test)]
#[path = "responsive/tests.rs"]
mod ui_tests;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) enum Pane {
    Source,
    #[default]
    Diagram,
}
#[derive(Default)]
pub(super) struct State {
    pub single: bool,
    pub docked: bool,
    pub pane: Pane,
    pub properties_hidden: bool,
    pub flyout: bool,
    pub touch: bool,
}
impl State {
    pub fn layout(&mut self, width: f32) {
        self.single = width < 720.0;
        self.docked = width >= 960.0;
        if self.docked {
            self.flyout = false;
        }
    }
    pub fn input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Touch { .. } => self.touch = true,
                    egui::Event::MouseMoved(_) => self.touch = false,
                    _ => {}
                }
            }
        });
    }
    pub fn properties_open(&self) -> bool {
        if self.docked {
            !self.properties_hidden
        } else {
            self.flyout
        }
    }
    pub fn toggle_properties(&mut self) {
        if self.docked {
            self.properties_hidden = !self.properties_hidden;
        } else {
            self.flyout = !self.flyout;
        }
    }
}
impl Editor {
    pub(super) fn pane_title(&mut self, ui: &mut egui::Ui, pane: Pane) {
        let title = |p| {
            crate::i18n::tr(if p == Pane::Source {
                "Source"
            } else {
                "Diagram"
            })
        };
        if self.responsive.single {
            egui::ComboBox::from_id_salt("responsive-pane")
                .selected_text(title(self.responsive.pane))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for p in [Pane::Source, Pane::Diagram] {
                        ui.selectable_value(&mut self.responsive.pane, p, title(p));
                    }
                });
        } else {
            ui.label(title(pane));
        }
    }
    pub(super) fn properties_button(&mut self, ui: &mut egui::Ui) {
        let item = toolbar::Item {
            label: "Properties",
            icon: Some(egui_phosphor::regular::SIDEBAR_SIMPLE),
            enabled: true,
            separator_before: false,
        };
        if toolbar::selected_button(ui, &item, false, Some(self.responsive.properties_open()))
            .clicked()
        {
            self.responsive.toggle_properties();
        }
    }
    /// Touch keeps controls on the selected object, without pretending it is a mouse hover.
    pub(super) fn graph_control_node<'a>(
        &self,
        preview: &'a Preview,
        canvas: egui::Rect,
        pointer: Option<egui::Pos2>,
    ) -> Option<&'a laml::geometry::BoxNode> {
        if self.responsive.touch {
            self.selection
                .ids
                .last()
                .and_then(|id| preview.nodes.iter().find(|n| &n.id == id))
        } else {
            pointer.and_then(|p| {
                connection_geometry::hover_node(
                    preview,
                    canvas,
                    p,
                    self.graph.hover_node.as_deref(),
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_preserves_chosen_pane_and_desktop_properties_preference() {
        let mut s = State::default();
        s.layout(1500.0);
        assert!(s.properties_open());
        s.toggle_properties();
        s.layout(390.0);
        assert!(s.single && !s.properties_open());
        assert_eq!(s.pane, Pane::Diagram);
        s.pane = Pane::Source;
        s.toggle_properties();
        assert!(s.properties_open());
        s.layout(800.0);
        assert!(!s.single && !s.docked);
        s.layout(1500.0);
        assert!(!s.properties_open() && !s.flyout);
        s.layout(390.0);
        assert_eq!(s.pane, Pane::Source);
    }
}
