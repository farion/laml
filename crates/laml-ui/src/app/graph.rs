//! Per-document graphical editing and properties-dialog state.
use super::*;

pub(super) struct State {
    pub operator: String,
    pub insertion: Option<super::graph_insert::Menu>,
    pub toast: Option<super::toast::Toast>,
    pub icon_picker: Option<icon_picker::Picker>,
    pub navigator: bool,
    pub addresses: bool,
    pub motion: connection_preview::Cache,
    pub gesture: Option<connection_gesture::Gesture>,
    pub connection_press_shift: bool,
    pub drag_image: Option<(String, Vec<u8>)>,
    pub node_dialog: Option<graph_dialogs::NodeDialog>,
    pub port_dialog: Option<graph_dialogs::PortDialog>,
    pub deletion: Option<graph_dialogs::DeleteDialog>,
    pub context: Option<(graph_context::Target, egui::Pos2, u64)>,
    pub hover_node: Option<String>,
    pub hover_align: Option<laml::syntax::FreeAlignment>,
    pub flash: Option<alignment_guides::Flash>,
    pub report_errors: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            operator: "->".into(),
            insertion: None,
            toast: None,
            icon_picker: None,
            navigator: false,
            addresses: false,
            motion: Default::default(),
            gesture: None,
            connection_press_shift: false,
            drag_image: None,
            node_dialog: None,
            port_dialog: None,
            deletion: None,
            context: None,
            hover_node: None,
            hover_align: None,
            flash: None,
            report_errors: false,
        }
    }
}
impl State {
    pub fn cancel_gesture(&mut self) {
        self.motion = Default::default();
        self.gesture = None;
        self.connection_press_shift = false;
        self.drag_image = None;
    }
    pub fn dialog_open(&self) -> bool {
        self.insertion.is_some()
            || self.navigator
            || self.icon_picker.is_some()
            || self.addresses
            || self.node_dialog.is_some()
            || self.port_dialog.is_some()
            || self.deletion.is_some()
    }
}
