//! Write text during the browser's trusted copy/cut event, including HTTP hosts.
//!
//! eframe runs egui synchronously inside those events, but its default output
//! uses navigator.clipboard, which requires a secure context. ClipboardEvent's
//! writable data store does not require that API or clipboard permissions.
use eframe::egui;
use std::{cell::RefCell, sync::Arc};
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(module = "/web/clipboard.js")]
extern "C" {
    #[wasm_bindgen(js_name = copyText)]
    fn copy_text(text: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = pasteText)]
    fn paste_text() -> js_sys::Promise;
}

pub(crate) fn text(
    write: Option<String>,
    ctx: &egui::Context,
) -> std::sync::mpsc::Receiver<Result<Option<String>, String>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    // Start from the menu's click, not from a later render or timer.
    let promise = write
        .as_ref()
        .map_or_else(paste_text, |text| copy_text(text));
    wasm_bindgen_futures::spawn_local(async move {
        let result = wasm_bindgen_futures::JsFuture::from(promise).await;
        let result = match result {
            Ok(value) if write.is_none() => value.as_string().map(Some).ok_or_else(|| {
                "Clipboard operation failed. Use the keyboard shortcut instead.".into()
            }),
            Ok(_) => Ok(None),
            Err(_) if write.is_none() => {
                Err("Clipboard access is blocked. Paste with Ctrl+V / Cmd+V instead.".into())
            }
            Err(_) => Err("Clipboard operation failed. Use the keyboard shortcut instead.".into()),
        };
        let _ = sender.send(result);
        ctx.request_repaint();
    });
    receiver
}

thread_local! {
    static EVENT: RefCell<Option<web_sys::ClipboardEvent>> = const { RefCell::new(None) };
}

pub(crate) fn install(ctx: &egui::Context) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("No browser document"))?;
    for kind in ["copy", "cut"] {
        let capture = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(|event| {
            EVENT.with(|current| *current.borrow_mut() = Some(event));
        });
        // Capture before eframe's document listener runs the synchronous UI pass.
        document.add_event_listener_with_callback_and_bool(
            kind,
            capture.as_ref().unchecked_ref(),
            true,
        )?;
        capture.forget();
    }
    ctx.on_end_pass(
        "laml browser clipboard",
        Arc::new(|ui| {
            let Some(event) = EVENT.with(|current| current.borrow_mut().take()) else {
                return;
            };
            let Some(data) = event.clipboard_data() else {
                return;
            };
            ui.output_mut(|output| {
                output.commands.retain(|command| {
                    if let egui::OutputCommand::CopyText(text) = command
                        && data.set_data("text/plain", text).is_ok()
                    {
                        event.prevent_default();
                        // Do not also invoke eframe's HTTPS-only asynchronous API.
                        return false;
                    }
                    true
                });
            });
        }),
    );
    Ok(())
}
