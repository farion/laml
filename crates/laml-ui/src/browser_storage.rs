//! Thin browser persistence/download adapter; no editor UI or compiler model in JS.
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/web/browser-storage.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn initialize() -> Result<(), JsValue>;
    #[wasm_bindgen(js_name = sessionId)]
    pub fn session_id() -> String;
    pub fn available() -> bool;
    #[wasm_bindgen(js_name = connectionStatus)]
    pub fn connection_status() -> String;
    #[wasm_bindgen(js_name = storageStatus)]
    pub fn storage_status() -> String;
    #[wasm_bindgen(js_name = storageFailed)]
    pub fn storage_failed() -> bool;
    #[wasm_bindgen(js_name = safeToClose)]
    pub fn safe_to_close() -> bool;
    #[wasm_bindgen(js_name = newId)]
    pub fn new_id() -> String;
    pub fn snapshot() -> String;
    #[wasm_bindgen(js_name = startupTabs)]
    pub fn startup_tabs() -> String;
    #[wasm_bindgen(js_name = rememberTabs)]
    pub fn remember_tabs(ids: &str);
    #[wasm_bindgen(js_name = openRecord)]
    pub fn open_record(id: &str) -> String;
    #[wasm_bindgen(js_name = saveDocument)]
    pub fn save_document(id: &str, name: &str, source: &str);
    #[wasm_bindgen(js_name = libraryAction)]
    pub fn library_action(action: &str, id: &str, name: &str);
    #[wasm_bindgen(js_name = importLaml)]
    pub fn import_laml();
    #[wasm_bindgen(js_name = takeImports)]
    pub fn take_imports() -> String;
    #[wasm_bindgen(js_name = takeActions)]
    pub fn take_actions() -> String;
    pub fn download(name: &str, bytes: &[u8], mime: &str);
}
