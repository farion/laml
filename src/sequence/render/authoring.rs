//! Source-linked regions from the final, reflowed sequence geometry.
use super::*;
use crate::authoring::Region;

pub(super) fn event_line(event: &Event) -> usize {
    match event {
        Event::Message(m) => m.line,
        Event::Note(note) => note.line,
        Event::Continuation(cont) => cont.line,
        Event::Annotation(annotation) => annotation.line,
        Event::Ordering(rule) => rule.line,
        Event::Decomposition { line, .. }
        | Event::Reference { line, .. }
        | Event::FrameStart { line, .. }
        | Event::FrameBranch { line, .. }
        | Event::FrameEnd { line }
        | Event::Destroy { line, .. }
        | Event::Activate { line, .. }
        | Event::Deactivate { line, .. } => *line,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn regions(
    out: &mut Vec<Region>,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    centers: &[f64],
    widths: &[f64],
    timeline: &lifetimes::Timeline<'_>,
    positioned: &[(&Message, f64)],
    anchors: &[anchors::MessageAnchor],
    routes: &[crate::route::Route],
    frames: &[DrawnFrame],
    left: f64,
    right: f64,
) -> Result<(), String> {
    for (i, p) in diagram.participants.iter().enumerate() {
        for segment in lifetimes::segments(
            diagram,
            &p.id,
            timeline.positions,
            timeline.deaths,
            timeline.branches,
            timeline.header_height,
            timeline.bottom,
        )? {
            out.push(Region::stroke(
                &p.id,
                "lifeline",
                p.line,
                vec![(centers[i], segment.start), (centers[i], segment.end)],
            ));
            let destroyed = timeline
                .deaths
                .iter()
                .any(|(id, y)| id == &p.id && (*y - segment.end).abs() < 0.01);
            let end_line = if destroyed {
                diagram
                    .events
                    .iter()
                    .find_map(|e| match e {
                        Event::Destroy { participant, line }
                            if participant == &p.id
                                && timeline.positions.iter().any(|(at, y)| {
                                    at == line && (*y - segment.end).abs() < 0.01
                                }) =>
                        {
                            Some(*line)
                        }
                        _ => None,
                    })
                    .unwrap_or(p.line)
            } else {
                p.line
            };
            let radius = if destroyed { 7.0 } else { 5.0 };
            out.push(Region::rect(
                &p.id,
                if destroyed {
                    "destroyed-end"
                } else {
                    "lifeline-end"
                },
                end_line,
                centers[i] - radius,
                segment.end - radius,
                radius * 2.0,
                radius * 2.0,
            ));
            if let Some(y) = segment.head {
                out.push(Region::rect(
                    &p.id,
                    "participant",
                    p.line,
                    centers[i] - widths[i] / 2.0,
                    y,
                    widths[i],
                    timeline.header_height,
                ));
            }
        }
    }
    for (((m, _), a), route) in positioned.iter().zip(anchors).zip(routes) {
        let (sx, sy) = a.send;
        let (tx, ty) = a.receive;
        let path = route.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>();
        out.push(Region::rect(
            &m.from,
            "sequence-send",
            m.line,
            sx,
            sy,
            0.0,
            0.0,
        ));
        out.push(Region::rect(
            &m.to,
            "sequence-receive",
            m.line,
            tx,
            ty,
            0.0,
            0.0,
        ));
        let id = m.name.clone().unwrap_or_else(|| format!("line:{}", m.line));
        let mut region = Region::stroke(&id, "message", m.line, path.clone());
        if !route.bridges.is_empty() || !route.gaps.is_empty() {
            region.pick_paths =
                crate::svg::route_pick_paths(route, &[], crate::theme::Theme::Plain)?
                    .into_iter()
                    .map(|p| p.into_iter().map(|p| (p.x, p.y)).collect())
                    .collect();
        }
        out.push(region);
        if let Some(label) = m.label.as_ref().filter(|label| !label.trim().is_empty()) {
            let paint = styles::message(sheet, m.name.as_deref(), &m.classes);
            let w = geometry::label_width_at(label, paint.font_size)?;
            let h = paint.font_size * 1.25 * label.lines().count().max(1) as f64;
            let (x, baseline) = super::routing::label(m, a, &path, paint.font_size)?;
            out.push(Region::rect(
                id,
                "message",
                m.line,
                x - w / 2.0 - 4.0,
                baseline - paint.font_size,
                w + 8.0,
                h + 4.0,
            ));
        }
    }
    coregions::regions(out, diagram, centers, anchors)?;
    for frame in frames {
        let (x, r) = geometry::frame_bounds(diagram, sheet, centers, widths, frame, left, right)?;
        out.push(Region::rect(
            frame
                .id
                .clone()
                .unwrap_or_else(|| format!("line:{}", frame.line)),
            "frame",
            frame.line,
            x,
            frame.top,
            r - x,
            frame.bottom - frame.top,
        ));
        fragment_text(out, diagram, sheet, frame, x, r)?;
    }
    // Insertion handles consume final timeline positions, including operand boundaries.
    // Their line means "insert before", not an editable object or a pixel coordinate.
    let mut positions = timeline.positions.to_vec();
    positions.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut previous = HEADER_Y + timeline.header_height;
    for &(line, y) in &positions {
        if y > previous + 2.0 {
            out.push(Region::rect(
                format!("slot:{line}"),
                "sequence-slot",
                line,
                left,
                (previous + y) / 2.0,
                right - left,
                0.0,
            ));
        }
        previous = previous.max(y);
    }
    out.push(Region::rect(
        "slot:end",
        "sequence-slot",
        0,
        left,
        (previous + timeline.bottom - 16.0) / 2.0,
        right - left,
        0.0,
    ));
    // Timeline boundaries remain insertion docks, not selectable diagram objects.
    // Visible elements publish their own final geometry instead of full-width rows.
    Ok(())
}

pub(super) fn notes(
    out: &mut Vec<Region>,
    notes: &[(&crate::sequence::Note, f64)],
    continuations: &[(&crate::sequence::Continuation, f64)],
    shapes: &[text_collision::Rect],
) {
    for ((note, _), r) in notes.iter().zip(shapes) {
        out.push(Region::rect(
            &note.id, "note", note.line, r.x, r.y, r.width, r.height,
        ));
    }
    for ((continuation, _), r) in continuations.iter().zip(shapes.iter().skip(notes.len())) {
        out.push(Region::rect(
            &continuation.id,
            "continuation",
            continuation.line,
            r.x,
            r.y,
            r.width,
            r.height,
        ));
    }
}
pub(super) fn decompositions(
    out: &mut Vec<Region>,
    diagram: &SequenceDiagram,
    centers: &[f64],
    prepared: &[decompositions::Prepared],
    positions: &[(usize, f64)],
) {
    for &(index, top) in positions {
        let part = &diagram.decompositions[index];
        let panel = &prepared[index];
        let Some(parent) = diagram
            .participants
            .iter()
            .position(|p| p.id == part.parent)
        else {
            continue;
        };
        let left = centers[parent] - panel.width / 2.0;
        out.push(Region::rect(
            format!("decompose:{}", part.parent),
            "decomposition",
            part.line,
            left - 8.0,
            top - 8.0,
            panel.width + 16.0,
            panel.height + 16.0,
        ));
        for region in &panel.regions {
            let mut region = region.clone();
            region.line = part.line.saturating_add(region.line).saturating_sub(5);
            region.end_line = region
                .end_line
                .map(|line| part.line.saturating_add(line).saturating_sub(5));
            region.id = format!("{}.{}", part.parent, region.id);
            region.x += left;
            region.y += top;
            for p in region
                .path
                .iter_mut()
                .chain(region.pick_paths.iter_mut().flatten())
            {
                p.0 += left;
                p.1 += top;
            }
            out.push(region);
        }
    }
}

pub(super) fn annotations(
    out: &mut Vec<Region>,
    annotations: &[&crate::sequence::Annotation],
    shapes: &[text_collision::Rect],
) {
    for (annotation, r) in annotations.iter().zip(shapes) {
        out.retain(|old| old.line != annotation.line || old.kind != "event");
        out.push(Region::rect(
            format!("line:{}", annotation.line),
            "annotation",
            annotation.line,
            r.x,
            r.y,
            r.width,
            r.height,
        ));
    }
}

pub(super) fn references(
    out: &mut Vec<Region>,
    diagram: &SequenceDiagram,
    references: &[(&str, f64)],
    left: f64,
    right: f64,
    sheet: &StyleSheet,
) -> Result<(), String> {
    let lines = diagram.events.iter().filter_map(|e| match e {
        Event::Reference { line, .. } => Some(*line),
        _ => None,
    });
    let font = styles::fragment(sheet, "ref", None).font_size;
    for ((label, y), line) in references.iter().zip(lines) {
        let width = (right - left).max(geometry::label_width_at(label, font)? + 52.0);
        out.push(Region::rect(
            format!("line:{line}"),
            "reference",
            line,
            left,
            y - 22.0,
            width,
            frames::reference_height(sheet),
        ));
    }
    Ok(())
}

/// Source-linked text affordances use the same metrics as frames::fragment.
fn fragment_text(
    out: &mut Vec<Region>,
    diagram: &SequenceDiagram,
    sheet: &StyleSheet,
    frame: &DrawnFrame,
    left: f64,
    right: f64,
) -> Result<(), String> {
    let paint = styles::fragment(
        sheet,
        geometry::operator_name(&frame.operator),
        frame.id.as_deref(),
    );
    let font = paint.font_size;
    let tab = geometry::label_width_at(&frame.operator, font)? + 22.0;
    let guard_width = geometry::label_width_at(&format!("[{}]", frame.guard), font)?.max(20.0);
    out.push(Region::rect(
        format!("line:{}", frame.line),
        "sequence-guard-header",
        frame.line,
        left,
        frame.top,
        (tab + 8.0 + guard_width).min(right - left),
        frames::descriptor_height(font),
    ));
    if !frame.guard.is_empty() {
        out.push(Region::rect(
            format!("line:{}", frame.line),
            "sequence-guard",
            frame.line,
            left + tab + 8.0,
            frame.top + 4.0,
            guard_width,
            font * 1.25,
        ));
    }
    let mut stack = Vec::new();
    let mut branches = Vec::new();
    for event in &diagram.events {
        match event {
            Event::FrameStart { line, .. } => stack.push(*line),
            Event::FrameEnd { .. } => {
                stack.pop();
            }
            Event::FrameBranch { line, .. } if stack.last() == Some(&frame.line) => {
                branches.push(*line)
            }
            _ => {}
        }
    }
    for ((guard, y), line) in frame.separators.iter().zip(branches) {
        let width = geometry::label_width_at(&format!("[{guard}]"), font)?.max(20.0);
        out.push(Region::rect(
            format!("line:{line}"),
            "sequence-guard-header",
            line,
            left,
            y + 2.0,
            (width + 20.0).min(right - left),
            font * 1.25 + 6.0,
        ));
        if !guard.is_empty() {
            out.push(Region::rect(
                format!("line:{line}"),
                "sequence-guard",
                line,
                left + 10.0,
                y + 5.0,
                width,
                font * 1.25,
            ));
        }
    }
    Ok(())
}
