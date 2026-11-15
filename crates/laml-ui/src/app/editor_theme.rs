//! Editor preferences are independent of authored diagram settings and exports.
use super::*;
use laml_ui::backend::Presentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum EditorTheme {
    #[default]
    Auto,
    Dark,
    Light,
}
impl EditorTheme {
    pub fn label(self) -> &'static str {
        match self { Self::Auto => "Auto", Self::Dark => "Dark", Self::Light => "Light" }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub editor_theme: EditorTheme,
    pub fit_preview: bool,
    pub welcome_seen: bool,
}
impl Preferences {
    pub fn apply(self, ctx: &egui::Context) {
        ctx.set_theme(match self.editor_theme {
            EditorTheme::Auto => egui::ThemePreference::System,
            EditorTheme::Dark => egui::ThemePreference::Dark,
            EditorTheme::Light => egui::ThemePreference::Light,
        });
    }
    pub fn resolved(self, ctx: &egui::Context) -> egui::Theme {
        match self.editor_theme {
            EditorTheme::Dark => egui::Theme::Dark,
            EditorTheme::Light => egui::Theme::Light,
            EditorTheme::Auto => {
                #[cfg(target_arch = "wasm32")]
                let detected = eframe::web::system_theme().or(ctx.system_theme());
                #[cfg(not(target_arch = "wasm32"))]
                let detected = ctx.system_theme();
                detected.unwrap_or(egui::Theme::Dark)
            }
        }
    }
    pub fn presentation(self, theme: egui::Theme) -> Presentation {
        if !self.fit_preview { Presentation::Document }
        else if theme == egui::Theme::Dark { Presentation::Editor }
        else { Presentation::EditorLight }
    }
    pub fn load() -> Self {
        load().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }
    pub fn save(self) -> Result<(), String> {
        save(&serde_json::to_string(&self).map_err(|e| e.to_string())?)
    }
}

pub(super) fn new_source(theme: egui::Theme) -> String {
    let theme = if theme == egui::Theme::Dark { "modern-dark" } else { "modern" };
    format!("settings {{\n    theme: {theme}\n    background: transparent\n}}\n\nsemantics {{\n}}\n")
}

impl Workspace {
    pub(super) fn sync_editor_theme(&mut self, ctx: &egui::Context) {
        self.preferences.apply(ctx);
        let theme = ctx.theme();
        self.editor_theme = theme;
        let presentation = self.preferences.presentation(theme);
        for editor in &mut self.tabs {
            editor.editor_theme = theme;
            if editor.compiler.presentation != presentation && editor.pending_command.is_none() {
                // A presentation change invalidates in-flight replies even if source
                // revision is unchanged. Text, undo and file dirty state are untouched.
                editor.compiler.forget_current();
                editor.compiler.presentation = presentation;
                editor.placement.cancel();
                editor.placement.set_presentation(presentation);
                editor.end_connection(ctx);
                editor.submitted = None;
                editor.rendered = None;
                editor.retry_after = None;
                // Browser instants start at page load and cannot go below zero.
                let now = Instant::now();
                editor.changed = now.checked_sub(Duration::from_millis(300)).unwrap_or(now);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let root = std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    root.map(|p| p.join("laml/editor-settings.json"))
}
#[cfg(not(target_arch = "wasm32"))]
fn load() -> Option<String> {
    std::fs::read_to_string(settings_path()?).ok()
}
#[cfg(not(target_arch = "wasm32"))]
fn save(text: &str) -> Result<(), String> {
    let path = settings_path().ok_or("Cannot locate the editor settings directory")?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}
#[cfg(target_arch = "wasm32")]
fn load() -> Option<String> {
    web_sys::window()?.local_storage().ok()??.get_item("laml.editor.settings").ok()?
}
#[cfg(target_arch = "wasm32")]
fn save(text: &str) -> Result<(), String> {
    let storage = web_sys::window().ok_or("No browser window")?
        .local_storage().map_err(|_| "Browser storage unavailable")?
        .ok_or("Browser storage unavailable")?;
    storage.set_item("laml.editor.settings", text).map_err(|_| "Could not save editor settings".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_follow_system_and_use_document_preview() {
        let prefs = Preferences::default();
        assert_eq!(prefs.editor_theme, EditorTheme::Auto);
        assert!(!prefs.fit_preview);
        let ctx = egui::Context::default();
        prefs.apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        assert_eq!(prefs.presentation(ctx.theme()), Presentation::Document);
    }
    #[test]
    fn switching_editor_preferences_keeps_document_and_undo_untouched() {
        let ctx = egui::Context::default();
        let source = "settings {\n theme: plain\n}\nsemantics {\n A\n}\n";
        let mut workspace = Workspace::new(Editor::new(source.into(), None));
        workspace.preferences = Preferences { editor_theme: EditorTheme::Light, fit_preview: true, ..Default::default() };
        workspace.sync_editor_theme(&ctx);
        let editor = &workspace.tabs[0];
        assert_eq!(editor.compiler.presentation, Presentation::EditorLight);
        assert_eq!(editor.document.source(), source);
        assert_eq!(editor.document.revision(), 0);
        assert!(!editor.document.is_dirty());
        workspace.preferences.fit_preview = false;
        workspace.sync_editor_theme(&ctx);
        assert_eq!(workspace.tabs[0].compiler.presentation, Presentation::Document);
        assert_eq!(workspace.tabs[0].document.source(), source);
    }

    #[test]
    fn new_files_author_explicit_settings_for_each_editor_theme() {
        for (editor, diagram) in [(egui::Theme::Dark, laml::theme::Theme::ModernDark),
            (egui::Theme::Light, laml::theme::Theme::Modern)] {
            let source = new_source(editor);
            assert!(source.starts_with("settings {"));
            let parsed = laml::parser::parse(&source).unwrap();
            assert_eq!(parsed.settings.theme, diagram);
            assert!(parsed.settings.transparent);
            let prefs = Preferences { fit_preview: true, ..Default::default() };
            assert_eq!(prefs.presentation(editor).theme_override(), Some(diagram));
        }
    }
}
