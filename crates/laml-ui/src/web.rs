//! Browser entry for the exact same Editor/Workspace used natively.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use wasm_bindgen::{JsCast, prelude::*};
static BROWSER: AtomicBool = AtomicBool::new(false);
pub(crate) fn browser_mode() -> bool {
    BROWSER.load(Ordering::Relaxed)
}
static LIMITS: AtomicBool = AtomicBool::new(false);
pub(crate) fn limits() -> bool {
    LIMITS.load(Ordering::Relaxed)
}
static DIRTY: AtomicBool = AtomicBool::new(false);
static CLIENT: AtomicU64 = AtomicU64::new(0);
pub(crate) fn client_id() -> u64 {
    CLIENT.load(Ordering::Relaxed)
}
pub(crate) fn set_dirty(dirty: bool) {
    DIRTY.store(dirty, Ordering::Relaxed);
}
pub(crate) fn set_document(
    name: &str,
    ready: bool,
    dirty: bool,
    revision: u64,
    recovery_count: usize,
) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(&format!(
            "{name}{} — LAML Editor",
            if dirty { "*" } else { "" }
        ));
        if let Some(canvas) = document.get_element_by_id("editor") {
            let _ =
                canvas.set_attribute("data-laml-preview", if ready { "ready" } else { "stale" });
            let _ = canvas.set_attribute("data-laml-revision", &revision.to_string());
            let _ = canvas.set_attribute("data-laml-recovery-count", &recovery_count.to_string());
        }
    }
}
#[wasm_bindgen]
pub async fn start_editor(
    canvas: web_sys::HtmlCanvasElement,
    limits: bool,
    browser: bool,
) -> Result<(), JsValue> {
    BROWSER.store(browser, Ordering::Relaxed);
    if browser {
        crate::browser_storage::initialize().await?;
    }
    LIMITS.store(limits, Ordering::Relaxed);
    CLIENT.store(
        (js_sys::Math::random() * 4_503_599_627_370_496.0) as u64 + 1,
        Ordering::Relaxed,
    );
    canvas.set_attribute("data-laml-client", &client_id().to_string())?;
    let before = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
        |event: web_sys::BeforeUnloadEvent| {
            if !browser_mode() && DIRTY.load(Ordering::Relaxed) {
                event.prevent_default();
                event.set_return_value("Unsaved changes");
            }
        },
    );
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("No browser window"))?
        .add_event_listener_with_callback("beforeunload", before.as_ref().unchecked_ref())?;
    before.forget();
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| {
                egui_extras::install_image_loaders(&cc.egui_ctx);
                crate::app::Editor::install_appearance(&cc.egui_ctx);
                let editor = crate::app::Editor::with_services(
                    crate::app::Editor::new_source(&cc.egui_ctx),
                    None,
                    crate::services::Services::browser("/api/editor".into(), limits),
                );
                let mut workspace = crate::app::Workspace::empty(editor);
                if browser {
                    workspace.load_browser_tabs();
                }
                Ok(Box::new(workspace))
            }),
        )
        .await?;
    // The runner owns event listeners and stays alive for this page's lifetime.
    std::mem::forget(runner);
    Ok(())
}
