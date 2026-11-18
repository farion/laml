//! Joint sequence routing: contained self-calls get inner corridors, not crossings.
use super::{
    Message, SequenceDiagram, anchors::MessageAnchor, geometry::label_width_at, message_geometry,
    styles,
};
use crate::{
    route::{Point, Route},
    styles::StyleSheet,
};

pub(super) fn paths(
    messages: &[(&Message, f64)],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
    diagonal: bool,
) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let mut paths = messages
        .iter()
        .zip(anchors)
        .map(|((m, _), a)| message_geometry::path(a.send, a.receive, m.from == m.to, diagonal))
        .collect::<Vec<_>>();
    let mut loops = messages
        .iter()
        .enumerate()
        .filter(|(_, (m, _))| m.from == m.to)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    loops.sort_by(|&a, &b| {
        (anchors[a].receive.1 - anchors[a].send.1)
            .total_cmp(&(anchors[b].receive.1 - anchors[b].send.1))
            .then(a.cmp(&b))
    });
    let mut placed = Vec::<usize>::new();
    for i in loops {
        let a = &anchors[i];
        let mut right = a.send.0.max(a.receive.0) + 44.0;
        for &j in &placed {
            let b = &anchors[j];
            if messages[i].0.from != messages[j].0.from
                || a.send.1 >= b.receive.1 - 0.01
                || a.receive.1 <= b.send.1 + 0.01
            {
                continue;
            }
            let previous = paths[j][1].0;
            let contained = a.send.1 <= b.send.1 + 0.01 && a.receive.1 >= b.receive.1 - 0.01;
            if contained || (right - previous).abs() < 16.0 {
                let paint =
                    styles::message(sheet, messages[j].0.name.as_deref(), &messages[j].0.classes);
                let width = messages[j]
                    .0
                    .label
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| label_width_at(s, paint.font_size))
                    .transpose()?
                    .unwrap_or(0.0);
                right = right.max(previous + 16.0 + width);
            }
        }
        paths[i] = message_geometry::self_path(a.send, a.receive, right);
        placed.push(i);
    }
    Ok(paths)
}
pub(super) fn label(
    m: &Message,
    a: &MessageAnchor,
    path: &[(f64, f64)],
    font: f64,
) -> Result<(f64, f64), String> {
    let text = m.label.as_deref().unwrap_or("");
    let width = label_width_at(text, font)?;
    let bent = m.from != m.to && path.len() > 2;
    let x = if m.from == m.to {
        path[1].0 + 8.0 + width / 2.0
    } else if bent {
        a.send.0
            + (a.receive.0 - a.send.0).signum()
                * (width / 2.0 + 20.0).min((a.receive.0 - a.send.0).abs() / 2.0)
    } else {
        (a.send.0 + a.receive.0) / 2.0
    };
    let row = if bent {
        a.send.1
    } else {
        (a.send.1 + a.receive.1) / 2.0
    };
    Ok((
        x,
        row - 9.0 - font * 1.25 * (text.split('\n').count() - 1) as f64,
    ))
}
pub(super) fn reserve(
    diagram: &SequenceDiagram,
    messages: &[(&Message, f64)],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
    centers: &mut [f64],
    widths: &[f64],
) -> Result<(), String> {
    let paths = paths(messages, anchors, sheet, diagram.allow_diagonal)?;
    let mut needed = vec![0.0_f64; centers.len()];
    for ((m, _), path) in messages.iter().zip(paths) {
        if m.from != m.to {
            continue;
        }
        let Some(i) = diagram.participants.iter().position(|p| p.id == m.from) else {
            continue;
        };
        let font = styles::message(sheet, m.name.as_deref(), &m.classes).font_size;
        let caption = m
            .label
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| label_width_at(s, font))
            .transpose()?
            .unwrap_or(0.0);
        needed[i] = needed[i].max(path[1].0 - centers[i] + caption + 16.0);
    }
    let mut shift = 0.0;
    for i in 1..centers.len() {
        centers[i] += shift;
        let extra = (centers[i - 1] + needed[i - 1] + widths[i] / 2.0 + 16.0 - centers[i]).max(0.0);
        centers[i] += extra;
        shift += extra;
    }
    Ok(())
}
pub(super) fn build(
    diagram: &SequenceDiagram,
    messages: &[(&Message, f64)],
    anchors: &[MessageAnchor],
    sheet: &StyleSheet,
) -> Result<Vec<Route>, String> {
    let paths = paths(messages, anchors, sheet, diagram.allow_diagonal)?;
    let mut routes = Vec::<Route>::new();
    for (((m, _), a), path) in messages.iter().zip(anchors).zip(paths) {
        let paint = styles::message(sheet, m.name.as_deref(), &m.classes);
        let label = if let Some(text) = m.label.as_ref().filter(|s| !s.trim().is_empty()) {
            let (x, baseline) = label(m, a, &path, paint.font_size)?;
            let width = label_width_at(text, paint.font_size)?;
            Some(crate::route::Label {
                text: text.clone(),
                x: x - width / 2.0,
                y: baseline - paint.font_size,
                width,
                height: paint.font_size * 1.25 * text.lines().count().max(1) as f64,
                angle: 0.0,
                placement: crate::syntax::LabelPlacement::Near,
            })
        } else {
            None
        };
        let mut appearance = crate::syntax::EdgeAppearance::default();
        if m.to != "?" && m.kind != super::MessageKind::Delete {
            appearance.end = crate::syntax::EndSymbol::Arrow;
        }
        let mut route = Route {
            source_line: m.line,
            points: path.into_iter().map(|(x, y)| Point { x, y }).collect(),
            qualifiers: vec![],
            bridges: vec![],
            gaps: vec![],
            label,
            end_labels: vec![],
            from: diagram
                .participants
                .iter()
                .position(|p| p.id == m.from)
                .unwrap_or(usize::MAX),
            to: diagram
                .participants
                .iter()
                .position(|p| p.id == m.to)
                .unwrap_or(usize::MAX),
            color: paint.color,
            width: paint.width,
            font_size: paint.font_size,
            bidirectional: false,
            marker_end: false,
            appearance,
        };
        if !crate::route::decorate_crossings(&mut route, &mut routes) {
            super::crossings::cut(&mut route, &mut routes)?;
        }
        routes.push(route);
    }
    Ok(routes)
}

pub(super) fn canvas_width(base: f64, centers: &[f64], widths: &[f64], routes: &[Route]) -> f64 {
    let mut width = base.max(
        centers
            .last()
            .zip(widths.last())
            .map_or(0.0, |(x, w)| x + w / 2.0 + 64.0),
    );
    for route in routes {
        width = width.max(route.points.iter().map(|p| p.x + 32.0).fold(0.0, f64::max));
        if let Some(label) = &route.label {
            width = width.max(label.x + label.width + 32.0);
        }
    }
    width
}

pub(super) fn frame_extents(
    diagram: &SequenceDiagram,
    messages: &[(&Message, f64)],
    routes: &[Route],
    frames: &mut [super::DrawnFrame],
) {
    for frame in frames {
        let coverage = frame
            .id
            .as_ref()
            .and_then(|id| diagram.frame_layout.iter().find(|layout| &layout.id == id));
        for &i in &frame.messages {
            let m = messages[i].0;
            if m.from != m.to || coverage.is_some_and(|c| !c.participants.contains(&m.from)) {
                continue;
            }
            let r = &routes[i];
            let right = r
                .points
                .iter()
                .map(|p| p.x)
                .fold(0.0, f64::max)
                .max(r.label.as_ref().map_or(0.0, |l| l.x + l.width));
            frame.self_right = frame.self_right.max(right + 16.0);
        }
    }
}
