//! SVG output with selectable or outlined text measured against bundled Inter.

use crate::geometry::{BoxNode, NodeShape};
use crate::route::Route;
use crate::syntax::{NodeKind, NodePart};
use crate::text::{FONT, ITALIC_FONT, measure, measure_italic};
use base64::Engine;
use std::fmt::Write;
mod activity;
mod bounds;
mod compact;
mod component_container;
mod composite;
mod deployment;
mod escape;
mod interaction_use;
mod markers;
mod note;
mod package;
mod route_ends;
mod state;
mod usecase;
use escape::escape;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    Embedded,
    System,
    Paths,
}

pub fn render(boxes: &[BoxNode], routes: &[Route]) -> Result<String, String> {
    render_with_mode(boxes, routes, TextMode::Embedded)
}

pub fn render_with_mode(
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
) -> Result<String, String> {
    render_with_theme(boxes, routes, mode, crate::theme::Theme::Modern)
}

pub fn render_with_theme(
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
    theme: crate::theme::Theme,
) -> Result<String, String> {
    let svg = render_svg(boxes, routes, mode, theme)?;
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}

fn render_svg(
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
    theme: crate::theme::Theme,
) -> Result<String, String> {
    composite::validate(boxes, routes)?;
    let (left, top, right, bottom) = bounds::measure(boxes, routes);
    let mut svg = String::new();
    writeln!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"{left:.2} {top:.2} {:.2} {:.2}\">", right-left,bottom-top,right-left,bottom-top).unwrap();
    svg.push_str("<defs>");
    if mode == TextMode::Embedded {
        let font = base64::engine::general_purpose::STANDARD.encode(FONT);
        writeln!(svg, "<style>@font-face {{ font-family: 'Inter'; font-style: normal; src: url('data:font/otf;base64,{font}') format('opentype'); }}").unwrap();
        if boxes.iter().any(|node| {
            node.abstract_node
                || node.body.iter().any(|part| {
                    matches!(
                        part,
                        NodePart::Member {
                            abstract_member: true,
                            ..
                        }
                    )
                })
        }) {
            let italic = base64::engine::general_purpose::STANDARD.encode(ITALIC_FONT);
            writeln!(svg, "@font-face {{ font-family: 'Inter'; font-style: italic; src: url('data:font/otf;base64,{italic}') format('opentype'); }}").unwrap();
        }
        svg.push_str("</style>\n");
    }
    markers::definitions(&mut svg, theme);
    if boxes
        .iter()
        .any(|node| !node.icons.is_empty() || node.body_styles.iter().any(|row| row.icon.is_some()))
    {
        svg.push_str(crate::export_metadata::MARKER);
        svg.push_str(&escape(crate::export_metadata::PHOSPHOR_LICENSE));
        svg.push_str("</metadata>\n");
    }
    for node in boxes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Package | NodeKind::Profile))
    {
        package::background(&mut svg, node)?;
    }
    for node in boxes
        .iter()
        .filter(|node| node.kind == NodeKind::Component && node.is_container)
    {
        component_container::draw(&mut svg, node);
    }
    for node in boxes
        .iter()
        .filter(|node| node.kind == NodeKind::CompositeStructure && node.is_container)
    {
        composite::background(&mut svg, node);
    }
    deployment::backgrounds(&mut svg, boxes);
    state::backgrounds(&mut svg, boxes);
    usecase::background(&mut svg, boxes);
    for route in routes {
        let (visual_start, visual_end) = route_ends::marker_tips(route, boxes, theme);
        let start = markers::marker(route.appearance.start, true);
        let end = markers::marker(route.appearance.end, false);
        let start_marker = if start.is_empty() {
            String::new()
        } else {
            format!(" marker-start=\"{start}\"")
        };
        let end_marker = if end.is_empty() {
            String::new()
        } else {
            format!(" marker-end=\"{end}\"")
        };
        let dash = if route.appearance.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        if route.bridges.is_empty() && route.gaps.is_empty() {
            let last = route.points.len().saturating_sub(1);
            let points: Vec<String> = route
                .points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let point = if i == 0 {
                        visual_start.unwrap_or(*p)
                    } else if i == last {
                        visual_end.unwrap_or(*p)
                    } else {
                        *p
                    };
                    format!("{:.2},{:.2}", point.x, point.y)
                })
                .collect();
            writeln!(svg,"<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{end_marker}{start_marker} />",
                points.join(" "), route.color, route.width).unwrap();
        } else {
            let fragments = if visual_start.is_none() && visual_end.is_none() {
                crate::svg_paths::fragments(route)?
            } else {
                crate::svg_paths::fragments_with_ends(route, visual_start, visual_end)?
            };
            let last = fragments.len() - 1;
            for (i, path) in fragments.iter().enumerate() {
                writeln!(svg,"<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{}{} />",
                    path,route.color,route.width,
                    if i==0 { &start_marker } else { "" },
                    if i==last { &end_marker } else { "" }).unwrap();
            }
        }
        for label in route.label.iter().chain(route.end_labels.iter()) {
            let cx = label.x + label.width / 2.0;
            let cy = label.y + label.height / 2.0;
            let rotation = if label.angle.abs() > 1e-9 {
                format!(" transform=\"rotate({:.2} {cx:.2} {cy:.2})\"", label.angle)
            } else {
                String::new()
            };
            if theme.modern() {
                let neutral = theme.colors(theme.default_base(NodeKind::Package));
                let pill = if label.angle.abs() > 1e-9 {
                    rotation.as_str()
                } else {
                    ""
                };
                writeln!(svg, "<rect data-edge-label-pill=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" fill=\"{}\"{pill}/>",
                    label.x-4.0,label.y-2.0,label.width+8.0,label.height+4.0,neutral.fill).unwrap();
            }
            writeln!(svg,"<text x=\"{cx:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}>{}</text>",
                cy+route.font_size*1.25*0.28,route.font_size,route.color,rotation,escape(&label.text)).unwrap();
        }
    }
    for (index, node) in boxes
        .iter()
        .enumerate()
        .filter(|(_, node)| !matches!(node.kind, NodeKind::Package | NodeKind::Profile))
    {
        writeln!(
            svg,
            "<g data-node-id=\"{}\"><title>{}</title>",
            escape(&node.id),
            escape(&node.label)
        )
        .unwrap();
        if matches!(
            node.kind,
            NodeKind::ActivityInitial
                | NodeKind::ActivityFinal
                | NodeKind::ActivityFlowFinal
                | NodeKind::ActivityDecision
                | NodeKind::ActivityMerge
                | NodeKind::ActivityFork
                | NodeKind::ActivityJoin
        ) {
            activity::symbol(&mut svg, node);
        } else if matches!(
            node.kind,
            NodeKind::StateInitial | NodeKind::StateFinal | NodeKind::StateChoice
        ) {
            state::symbol(&mut svg, node);
        } else if node.kind == NodeKind::UseCaseActor {
            usecase::actor(&mut svg, node);
        } else if node.kind == NodeKind::UseCaseSystem {
            // Frame painted behind its contained use cases and routed edges.
        } else if node.kind == NodeKind::State && node.is_container {
            // The state frame is behind its internal transitions.
        } else if node.kind.is_deployment_target() {
            if !node.is_container {
                deployment::cube(&mut svg, node);
            }
        } else if node.kind == NodeKind::CompositeStructure && node.is_container {
            // Its internal-structure compartment was painted behind routed connectors.
        } else if node.kind == NodeKind::Component && node.is_container {
            // Its background was painted before internal routes; draw the heading and ports now.
        } else if matches!(node.kind, NodeKind::Socket | NodeKind::Lollipop) {
            compact::draw(&mut svg, node);
        } else if matches!(node.kind, NodeKind::Association | NodeKind::Junction) {
            compact::junction(&mut svg, node);
        } else if node.kind == NodeKind::CompositeRole {
            composite::role(&mut svg, node);
        } else if node.shape == NodeShape::Note {
            note::draw(&mut svg, node, index);
        } else if node.shape == NodeShape::Ellipse {
            writeln!(svg,"<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x+node.width/2.0,node.y+node.height/2.0,node.width/2.0,node.height/2.0,node.fill,node.border_color,node.border_width).unwrap();
        } else if node.shape == NodeShape::Diamond {
            writeln!(svg,"<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x+node.width/2.0,node.y,node.x+node.width,node.y+node.height/2.0,
                node.x+node.width/2.0,node.y+node.height,node.x,node.y+node.height/2.0,
                node.fill,node.border_color,node.border_width).unwrap();
        } else {
            writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x,node.y,node.width,node.height,node.radius,node.fill,node.border_color,node.border_width).unwrap();
        }
        if node.kind == NodeKind::InteractionUse {
            interaction_use::tab(&mut svg, node);
        }
        if node.kind == NodeKind::Component && node.top_band == 0.0 {
            component_container::icon(&mut svg, node);
        }
        let line_height = node.font_size * 1.25;
        if let Some(parameters) = &node.template {
            let width = measure(parameters, node.font_size)? + 12.0;
            let x = node.x + node.width - width - 4.0;
            writeln!(svg,"<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"{width:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-dasharray=\"4 3\" />",node.y+4.0,line_height+4.0,node.fill,node.border_color).unwrap();
            writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",x+6.0,node.y+6.0+node.font_size,node.font_size,node.color,escape(parameters)).unwrap();
        }
        let first_baseline = if node.kind == NodeKind::UseCaseActor {
            node.y + node.height - 4.0 - line_height * node.lines.len().saturating_sub(1) as f64
        } else if node.body.is_empty()
            && node.ports.is_empty()
            && node.template.is_none()
            && !matches!(node.kind, NodeKind::Package | NodeKind::Profile)
            && !node.is_container
            && !node.kind.is_deployment_target()
        {
            node.y
                + (node.height - line_height * node.lines.len() as f64) / 2.0
                + line_height * 0.78
        } else {
            node.y + node.padding / 2.0 + node.top_band + line_height * 0.78
        };
        for (i, line) in node.lines.iter().enumerate() {
            let baseline = first_baseline + line_height * i as f64;
            let last_with_icons = i + 1 == node.lines.len() && !node.icons.is_empty();
            let (text_x, anchor) = if last_with_icons {
                let width = measure(line, node.font_size)?;
                let span = (node.font_size + 4.0) * node.icons.len() as f64;
                let x = node.x + (node.width - width - span) / 2.0;
                for (index, icon) in node.icons.iter().enumerate() {
                    crate::svg_icons::draw(
                        &mut svg,
                        icon,
                        x + (node.font_size + 4.0) * index as f64,
                        baseline - node.font_size * 0.82,
                        node.font_size,
                        &node.color,
                    )?;
                }
                (x + span, "start")
            } else if node.kind == NodeKind::UseCaseSystem {
                (node.x + node.padding, "start")
            } else {
                (deployment::center(node), "middle")
            };
            writeln!(svg,"<text x=\"{text_x:.2}\" y=\"{baseline:.2}\" text-anchor=\"{anchor}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}>{}</text>",
                node.font_size,node.color,
                if node.abstract_node && i + 1 == node.lines.len() { " font-style=\"italic\"" } else { "" },escape(line)).unwrap();
            if node.kind == NodeKind::Object && i + 1 == node.lines.len() {
                let width = measure(line, node.font_size)?;
                let center = node.x + node.width / 2.0;
                let baseline = first_baseline + line_height * i as f64;
                writeln!(svg,"<line x1=\"{:.2}\" x2=\"{:.2}\" y1=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" />",
                    center-width/2.0,center+width/2.0,baseline+2.0,baseline+2.0,node.color).unwrap();
            }
        }
        if !node.body.is_empty() {
            let mut y =
                node.y + node.padding + node.top_band + line_height * node.lines.len() as f64;
            writeln!(svg,"<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                node.x, node.x+node.width, node.divider_color, node.border_width).unwrap();
            y += 4.0;
            for (part, row) in node.body.iter().zip(&node.body_styles) {
                match part {
                    NodePart::Separator => {
                        y += 4.0;
                        writeln!(svg,"<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                            node.x,node.x+node.width,node.divider_color,node.border_width).unwrap();
                        y += 4.0;
                    }
                    NodePart::Member {
                        id,
                        default_id,
                        text,
                        abstract_member,
                        static_member,
                        ..
                    } => {
                        let baseline = y + row.font_size * 1.25 * 0.78;
                        let (text_x, shown) = if let Some(icon) = &row.icon {
                            crate::svg_icons::draw(
                                &mut svg,
                                icon,
                                node.x + node.padding,
                                baseline - row.font_size * 0.82,
                                row.font_size,
                                &row.color,
                            )?;
                            (
                                node.x + node.padding + row.font_size + 4.0,
                                crate::geometry::member_visible_text(text, row),
                            )
                        } else {
                            (node.x + node.padding, text.as_str())
                        };
                        let rendered_id = id.clone().or_else(|| {
                            default_id
                                .as_ref()
                                .map(|name| format!("{}::{name}", node.id))
                        });
                        writeln!(svg,"<text x=\"{:.2}\" y=\"{baseline:.2}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}{}>{}</text>",
                            text_x,row.font_size,row.color,
                            if *abstract_member { " font-style=\"italic\"" } else { "" },
                            rendered_id.as_ref().map(|id| format!(" data-member-id=\"{}\"",escape(id))).unwrap_or_default(),
                            escape(shown)).unwrap();
                        if *static_member {
                            let width = if *abstract_member {
                                measure_italic(shown, row.font_size)?
                            } else {
                                measure(shown, row.font_size)?
                            };
                            writeln!(svg,"<line x1=\"{:.2}\" x2=\"{:.2}\" y1=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" />",
                                text_x,text_x+width,baseline+2.0,baseline+2.0,row.color).unwrap();
                        }
                        y += row.font_size * 1.25;
                    }
                }
            }
        }
        if !node.ports.is_empty() {
            let mut separators = Vec::new();
            if node.top_band > 0.0 {
                separators.push(node.y + node.padding + node.top_band - 4.0);
            }
            if node.ports.iter().any(|port| {
                matches!(
                    port.side,
                    crate::syntax::PortSide::Left | crate::syntax::PortSide::Right
                )
            }) {
                separators.push(node.y + node.port_start - 4.0);
            }
            if node.bottom_band > 0.0 {
                separators.push(node.y + node.height - node.padding - node.bottom_band);
            }
            if node.kind != NodeKind::ActivityAction
                && !matches!(node.shape, NodeShape::Ellipse | NodeShape::Diamond)
            {
                for separator in separators {
                    writeln!(svg, "<line x1=\"{:.2}\" y1=\"{separator:.2}\" x2=\"{:.2}\" y2=\"{separator:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                        node.x, node.x + node.width, node.divider_color, node.border_width).unwrap();
                }
            }
            for port in &node.ports {
                if theme.modern() && node.kind == NodeKind::Generic {
                    let fill = if port.explicit_fill {
                        &port.fill
                    } else {
                        &port.border_color
                    };
                    writeln!(svg, "<circle data-port-id=\"{}\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" />",
                        escape(&port.id),port.x,port.y,fill,node.fill).unwrap();
                } else {
                    writeln!(svg, "<rect data-port-id=\"{}\" x=\"{:.2}\" y=\"{:.2}\" width=\"10\" height=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                        escape(&port.id), port.x-5.0, port.y-5.0, port.fill, port.border_color, node.border_width).unwrap();
                }
                let (x, y, anchor) = match port.side {
                    crate::syntax::PortSide::Left => (
                        if matches!(node.shape, NodeShape::Ellipse | NodeShape::Diamond) {
                            port.x + node.padding + 4.0
                        } else {
                            node.x + node.padding
                        },
                        port.y + node.font_size * 0.28,
                        "start",
                    ),
                    crate::syntax::PortSide::Right => (
                        if matches!(node.shape, NodeShape::Ellipse | NodeShape::Diamond) {
                            port.x - node.padding - 4.0
                        } else {
                            node.x + node.width - node.padding
                        },
                        port.y + node.font_size * 0.28,
                        "end",
                    ),
                    crate::syntax::PortSide::Top => (
                        port.x,
                        node.y + node.padding + node.font_size * 0.78,
                        "middle",
                    ),
                    crate::syntax::PortSide::Bottom => (
                        port.x,
                        node.y + node.height - node.padding - node.bottom_band
                            + 4.0
                            + node.font_size * 0.78,
                        "middle",
                    ),
                };
                writeln!(svg,"<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                    node.font_size,port.color,escape(&port.label)).unwrap();
            }
        }
        svg.push_str("</g>\n");
    }
    for route in routes {
        for qualifier in &route.qualifiers {
            writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"white\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                qualifier.x, qualifier.y, qualifier.width, qualifier.height, route.color, route.width/2.0).unwrap();
            for (index, line) in qualifier.lines.iter().enumerate() {
                writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                    qualifier.x+qualifier.width/2.0,qualifier.y+5.0+qualifier.font_size*1.25*(index as f64+0.78),
                    qualifier.font_size,route.color,escape(line)).unwrap();
            }
        }
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}
