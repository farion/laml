//! Small allowlist of vendored Phosphor paths; no remote assets or icon-font metrics.
use std::fmt::Write;

pub(crate) fn source(name: &str) -> Option<&'static str> {
    match name {
        "folder" => Some(include_str!("../assets/phosphor/regular/folder.svg")),
        "lock" => Some(include_str!("../assets/phosphor/regular/lock.svg")),
        "globe" => Some(include_str!("../assets/phosphor/regular/globe.svg")),
        "shield" => Some(include_str!("../assets/phosphor/regular/shield.svg")),
        "package" => Some(include_str!("../assets/phosphor/regular/package.svg")),
        "user" => Some(include_str!("../assets/phosphor/regular/user.svg")),
        "database" => Some(include_str!("../assets/phosphor/regular/database.svg")),
        _ => None,
    }
}

pub(crate) fn visibility(glyph: char) -> Option<&'static str> {
    match glyph {
        '+' => Some("globe"),
        '-' => Some("lock"),
        '#' => Some("shield"),
        '~' => Some("package"),
        _ => None,
    }
}

pub(crate) fn draw(
    svg: &mut String,
    name: &str,
    x: f64,
    y: f64,
    size: f64,
    color: &str,
) -> Result<(), String> {
    let asset = source(name).ok_or_else(|| format!("unsupported Phosphor icon '{name}'"))?;
    let start = asset.find('>').ok_or("invalid bundled icon SVG")? + 1;
    let end = asset.rfind("</svg>").ok_or("invalid bundled icon SVG")?;
    writeln!(svg,"<g data-phosphor-icon=\"{name}\" transform=\"translate({x:.2} {y:.2}) scale({:.6})\" fill=\"{color}\">{}</g>",
        size / 256.0, &asset[start..end]).unwrap();
    Ok(())
}
