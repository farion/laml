//! Draw UML-style pentagon descriptors on fragment and ref frames.

use super::{
    DrawnFrame,
    geometry::{label_width_at, operator_name},
    styles,
    styles::escape,
};
use crate::styles::StyleSheet;
use std::fmt::Write;

struct DescriptorPaint<'a> {
    color: &'a str,
    stroke: f64,
    font: f64,
}

fn descriptor(
    svg: &mut String,
    left: f64,
    top: f64,
    title: &str,
    width: f64,
    paint: DescriptorPaint<'_>,
) -> Result<f64, String> {
    let DescriptorPaint {
        color,
        stroke,
        font,
    } = paint;
    let tab = label_width_at(title, font)? + 22.0;
    if tab + 12.0 > width {
        return Err("fragment descriptor does not fit frame".into());
    }
    let right = left + tab;
    let tab_height = descriptor_height(font);
    writeln!(svg, "<path d=\"M{left:.2},{top:.2} H{right:.2} v{:.2} l-9,9 H{left:.2} Z\" fill=\"white\" stroke=\"{}\" stroke-width=\"{stroke:.2}\" data-frame-descriptor=\"{}\" />",
        tab_height - 9.0, escape(color), escape(title)).unwrap();
    writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{font:.2}\" fill=\"{}\">{}</text>",
        left + 8.0, top + font + 4.0, escape(color), escape(title)).unwrap();
    Ok(tab)
}

pub(super) fn descriptor_height(font: f64) -> f64 {
    (font * 1.25 + 8.0).max(26.0)
}

pub(super) fn fragment(
    svg: &mut String,
    frame: DrawnFrame,
    left: f64,
    width: f64,
    sheet: &StyleSheet,
) -> Result<(), String> {
    let target = operator_name(&frame.operator);
    let paint = styles::fragment(sheet, target, frame.id.as_deref());
    writeln!(svg, "<rect x=\"{left:.2}\" y=\"{:.2}\" width=\"{width:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" data-fragment-operator=\"{}\" data-fragment-id=\"{}\" />",
        frame.top, frame.bottom - frame.top, paint.fill, paint.border, paint.border_width,
        escape(target), escape(frame.id.as_deref().unwrap_or(""))).unwrap();
    let tab = descriptor(
        svg,
        left,
        frame.top,
        &frame.operator,
        width,
        DescriptorPaint {
            color: &paint.border,
            stroke: paint.border_width,
            font: paint.font_size,
        },
    )?;
    if !frame.guard.is_empty() {
        writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">[{}]</text>",
            left + tab + 8.0, frame.top + paint.font_size + 4.0, paint.font_size,
            escape(&paint.color), escape(&frame.guard)).unwrap();
    }
    for (guard, split) in frame.separators {
        writeln!(svg, "<path d=\"M{left:.2},{split:.2} h{width:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"6 4\" />",
            paint.border, paint.border_width).unwrap();
        if !guard.is_empty() {
            writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">[{}]</text>",
                left + 10.0, split + paint.font_size + 5.0, paint.font_size, paint.color, escape(&guard)).unwrap();
        }
    }
    Ok(())
}

pub(super) fn reference_height(sheet: &StyleSheet) -> f64 {
    let font = styles::fragment(sheet, "ref", None).font_size;
    (descriptor_height(font) + font + 10.0).max(50.0)
}

pub(super) fn reference(
    svg: &mut String,
    label: &str,
    left: f64,
    y: f64,
    width: f64,
    sheet: &StyleSheet,
) -> Result<(), String> {
    let top = y - 22.0;
    let paint = styles::fragment(sheet, "ref", None);
    let height = reference_height(sheet);
    writeln!(svg, "<rect x=\"{left:.2}\" y=\"{top:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
        paint.fill, paint.border, paint.border_width).unwrap();
    descriptor(
        svg,
        left,
        top,
        "ref",
        width,
        DescriptorPaint {
            color: &paint.border,
            stroke: paint.border_width,
            font: paint.font_size,
        },
    )?;
    writeln!(svg, "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
        left + 10.0, top + height - 10.0, paint.font_size, paint.color, escape(label)).unwrap();
    Ok(())
}
