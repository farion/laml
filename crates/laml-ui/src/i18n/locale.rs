//! Browser preference order and native UI locale detection, with no network access.
#[cfg(target_arch = "wasm32")]
pub(super) fn languages() -> Vec<String> {
    use wasm_bindgen::JsValue;
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let navigator = js_sys::Reflect::get(&window, &JsValue::from_str("navigator")).ok();
    let Some(navigator) = navigator else {
        return Vec::new();
    };
    let languages = js_sys::Reflect::get(&navigator, &JsValue::from_str("languages"))
        .ok()
        .filter(js_sys::Array::is_array)
        .map(|v| {
            js_sys::Array::from(&v)
                .iter()
                .filter_map(|s| s.as_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !languages.is_empty() {
        return languages;
    }
    js_sys::Reflect::get(&navigator, &JsValue::from_str("language"))
        .ok()
        .and_then(|v| v.as_string())
        .into_iter()
        .collect()
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn languages() -> Vec<String> {
    static LANGUAGES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LANGUAGES.get_or_init(native_languages).clone()
}
#[cfg(not(target_arch = "wasm32"))]
fn native_languages() -> Vec<String> {
    // Explicit locale environment overrides take precedence on every platform.
    let get = |key: &str| std::env::var(key).ok().filter(|s| !s.is_empty());
    if let Some(locale) = get("LC_ALL") {
        return vec![locale];
    }
    if let Some(languages) = get("LANGUAGE") {
        return languages.split(':').map(str::to_owned).collect();
    }
    if let Some(locale) = get("LC_MESSAGES") {
        return vec![locale];
    }
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultLocaleName(name: *mut u16, count: i32) -> i32;
        }
        let mut name = [0u16; 85];
        // Windows writes at most count UTF-16 units and returns the length including NUL.
        let size = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
        if size > 1 {
            return vec![String::from_utf16_lossy(&name[..size as usize - 1])];
        }
    }
    #[cfg(target_os = "macos")]
    if let Ok(output) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        && output.status.success()
    {
        let locales: Vec<_> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().trim_matches([',', '"', ' ']))
            .filter(|s| !s.is_empty() && !matches!(*s, "(" | ")"))
            .map(str::to_owned)
            .collect();
        if !locales.is_empty() {
            return locales;
        }
    }
    get("LANG").into_iter().collect()
}
pub(super) fn set_document_language(language: super::Language) {
    #[cfg(target_arch = "wasm32")]
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = element.set_attribute(
            "lang",
            if language == super::Language::German {
                "de"
            } else {
                "en"
            },
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = language;
}
