//! Generated lookup for vendored Phosphor regular icons.
pub(super) fn source(name: &str) -> Option<&'static str> {
    match name {
        "wind" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/wind.svg"
        ))),
        "windmill" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/windmill.svg"
        ))),
        "windows-logo" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/windows-logo.svg"
        ))),
        "wine" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/wine.svg"
        ))),
        "wrench" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/wrench.svg"
        ))),
        "x" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/x.svg"
        ))),
        "x-circle" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/x-circle.svg"
        ))),
        "x-logo" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/x-logo.svg"
        ))),
        "x-square" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/x-square.svg"
        ))),
        "yarn" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/yarn.svg"
        ))),
        "yin-yang" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/yin-yang.svg"
        ))),
        "youtube-logo" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/phosphor/regular/youtube-logo.svg"
        ))),
        _ => None,
    }
}
