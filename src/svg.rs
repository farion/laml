//! SVG output with selectable or outlined text measured against bundled Inter.

mod picking;
use crate::geometry::{BoxNode, NodeShape};
use crate::route::Route;
use crate::syntax::{NodeKind, NodePart};
use crate::text::{FONT, ITALIC_FONT, measure, measure_italic};
use base64::Engine;
pub use picking::route_pick_paths;
use std::fmt::Write;
mod activity;
mod bounds;
pub mod c4;
mod compact;
mod component_container;
mod composite;
mod deployment;
mod edge_labels;
mod escape;
mod interaction_use;
mod markers;
mod note;
mod package;
mod route_ends;
mod routes;
mod state;
mod usecase;
use escape::escape;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    Embedded,
    System,
    Paths,
}

/// Canvas bounds shared by rendering and editor hit testing (GNE, without C4 margins).
pub fn canvas_bounds(boxes: &[BoxNode], routes: &[Route]) -> (f64, f64, f64, f64) {
    bounds::measure(boxes, routes)
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
    let svg = render_svg(boxes, routes, mode, theme, None)?;
    if mode == TextMode::Paths {
        crate::export::outlines(&svg)
    } else {
        Ok(svg)
    }
}

/// Paint routes using exactly the document painter, without node backgrounds.
/// Bounds can be the viewport, so a freely dragged endpoint is never clipped to
/// the original document image.
pub fn render_route_overlay(
    boxes: &[BoxNode],
    routes: &[Route],
    theme: crate::theme::Theme,
    bounds: (f64, f64, f64, f64),
) -> Result<String, String> {
    let (l, t, r, b) = bounds;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{l} {t} {} {}\"><defs>",
        r - l,
        b - t,
        r - l,
        b - t
    );
    markers::definitions(&mut svg, theme);
    routes::draw(&mut svg, boxes, routes, theme)?;
    routes::qualifiers(&mut svg, routes);
    svg.push_str("</svg>");
    Ok(svg)
}

fn render_svg(
    boxes: &[BoxNode],
    routes: &[Route],
    mode: TextMode,
    theme: crate::theme::Theme,
    c4_diagram: Option<&crate::syntax::Diagram>,
) -> Result<String, String> {
    composite::validate(boxes, routes)?;
    let (left, top, right, bottom) = bounds::measure(boxes, routes);
    let mut svg = String::new();
    let top_margin = if c4_diagram.is_some() { 54.0 } else { 0.0 };
    let bottom_margin = if c4_diagram.is_some() { 68.0 } else { 0.0 };
    let side_margin = if c4_diagram.is_some() { 20.0 } else { 0.0 };
    writeln!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.2}\" height=\"{:.2}\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\">", right-left+side_margin*2.0,bottom-top+top_margin+bottom_margin,left-side_margin,top-top_margin,right-left+side_margin*2.0,bottom-top+top_margin+bottom_margin).unwrap();
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
    if let Some(diagram) = c4_diagram {
        c4::heading(&mut svg, diagram, left, top);
    }
    for node in boxes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Package | NodeKind::Profile))
    {
        if c4_diagram
            .and_then(|d| d.nodes.iter().find(|n| n.id == node.id))
            .is_some_and(|n| n.classes.iter().any(|c| c == "c4Boundary"))
        {
            c4::boundary(&mut svg, node);
        } else {
            package::background(&mut svg, node)?;
        }
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
    routes::draw(&mut svg, boxes, routes, theme)?;
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
        } else if c4_diagram
            .and_then(|d| d.nodes.iter().find(|n| n.id == node.id))
            .is_some_and(|n| c4::shape(&mut svg, node, &n.classes))
        {
            // C4 element outlines and icons are drawn in a dedicated notation layer.
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
            let c4_element = c4_diagram
                .and_then(|d| d.nodes.iter().find(|n| n.id == node.id))
                .is_some_and(|n| n.classes.iter().any(|c| c.starts_with("c4")));
            let c4_font = if c4_element {
                c4::font_size(node, i)
            } else {
                node.font_size
            };
            writeln!(svg,"<text x=\"{text_x:.2}\" y=\"{baseline:.2}\" text-anchor=\"{anchor}\" font-family=\"Inter\" font-size=\"{c4_font:.2}\" fill=\"{}\"{}{}>{}</text>",
                node.color,
                if c4_element && i == 0 { " font-weight=\"bold\"" } else { "" },
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
    routes::qualifiers(&mut svg, routes);
    if c4_diagram.is_some() {
        c4::legend(&mut svg, left, bottom, theme);
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}
