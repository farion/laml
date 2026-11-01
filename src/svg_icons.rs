//! Full vendored Phosphor regular icon catalogue; no remote assets or font metrics.
use std::fmt::Write;
mod catalog0;
mod catalog1;
mod catalog10;
mod catalog11;
mod catalog12;
mod catalog13;
mod catalog14;
mod catalog15;
mod catalog2;
mod catalog3;
mod catalog4;
mod catalog5;
mod catalog6;
mod catalog7;
mod catalog8;
mod catalog9;

pub(crate) fn source(name: &str) -> Option<&'static str> {
    catalog0::source(name)
        .or_else(|| catalog1::source(name))
        .or_else(|| catalog2::source(name))
        .or_else(|| catalog3::source(name))
        .or_else(|| catalog4::source(name))
        .or_else(|| catalog5::source(name))
        .or_else(|| catalog6::source(name))
        .or_else(|| catalog7::source(name))
        .or_else(|| catalog8::source(name))
        .or_else(|| catalog9::source(name))
        .or_else(|| catalog10::source(name))
        .or_else(|| catalog11::source(name))
        .or_else(|| catalog12::source(name))
        .or_else(|| catalog13::source(name))
        .or_else(|| catalog14::source(name))
        .or_else(|| catalog15::source(name))
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
