//! C4-specific SVG notation layered over the measured generic graph geometry.
use super::escape::escape;
use crate::geometry::BoxNode;
use crate::syntax::Diagram;
use std::fmt::Write;

pub fn render(
    boxes: &[BoxNode],
    routes: &[crate::route::Route],
    mode: super::TextMode,
    theme: crate::theme::Theme,
    diagram: &Diagram,
) -> Result<String, String> {
    super::render_svg(boxes, routes, mode, theme, Some(diagram))
}

pub(super) fn heading(svg: &mut String, diagram: &Diagram, left: f64, top: f64) {
    let owner = diagram
        .nodes
        .iter()
        .rfind(|n| n.classes.iter().any(|c| c == "c4Boundary"))
        .or_else(|| {
            diagram.nodes.iter().find(|n| {
                n.classes
                    .iter()
                    .any(|c| c == "c4System" && !n.classes.iter().any(|c| c == "c4External"))
            })
        });
    let Some(owner) = owner else {
        return;
    };
    let name = owner
        .label
        .as_deref()
        .unwrap_or(&owner.id)
        .lines()
        .next()
        .unwrap_or(&owner.id);
    let parent = owner
        .parent
        .as_deref()
        .and_then(|id| diagram.nodes.iter().find(|n| n.id == id));
    let parent_name = parent
        .and_then(|p| p.label.as_deref())
        .and_then(|s| s.lines().next());
    let title = parent_name.map_or_else(|| name.to_string(), |p| format!("{p} - {name}"));
    let ty = if diagram.settings.diagram_type == crate::syntax::DiagramType::Class {
        "Code"
    } else if owner.classes.iter().any(|c| c == "c4Container") {
        "Component"
    } else if owner.classes.iter().any(|c| c == "c4Boundary") {
        "Container"
    } else {
        "System Context"
    };
    let ink = if diagram.settings.theme.dark() {
        "#edf4fc"
    } else {
        "#1b3345"
    };
    writeln!(svg, "<text x=\"{left:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"20\" font-weight=\"bold\" fill=\"{ink}\">{} View: {}</text>", top-23.0, ty, escape(&title)).unwrap();
}

pub(super) fn font_size(node: &BoxNode, index: usize) -> f64 {
    node.font_size
        * match index {
            0 => 1.12,
            1 => 0.75,
            _ => 0.94,
        }
}

pub(super) fn legend(svg: &mut String, left: f64, bottom: f64, theme: crate::theme::Theme) {
    let ink = if theme.dark() { "#cbd9ea" } else { "#536477" };
    writeln!(svg, "<text x=\"{left:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"12\" fill=\"{ink}\">Key: type/technology inside each box · arrows show labelled relationships</text>",bottom+38.0).unwrap();
}

pub(super) fn boundary(svg: &mut String, node: &BoxNode) {
    writeln!(svg, "<g data-node-id=\"{}\"><title>{}</title><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>",
        escape(&node.id),escape(&node.label),node.x,node.y,node.width,node.height,node.fill,node.border_color,node.border_width).unwrap();
    let name = node.lines.first().map_or("", String::as_str);
    let kind = node.lines.get(1).map_or("", String::as_str);
    writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"18\" font-weight=\"bold\" fill=\"{}\">{}</text>",node.x+12.0,node.y+node.height-30.0,node.color,escape(name)).unwrap();
    writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter\" font-size=\"12\" fill=\"{}\">{}</text></g>",node.x+12.0,node.y+node.height-13.0,node.color,escape(kind)).unwrap();
}

pub(super) fn shape(svg: &mut String, node: &BoxNode, classes: &[String]) -> bool {
    let has = |name: &str| classes.iter().any(|c| c == name);
    if !(has("c4Person") || has("c4System") || has("c4Container") || has("c4Component")) {
        return false;
    }
    let (x, y, w, h) = (node.x, node.y, node.width, node.height);
    let (stroke, fill, thick) = (&node.border_color, &node.fill, node.border_width);
    if has("c4Bucket") {
        writeln!(svg,"<path d=\"M{x:.2},{:.2} L{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2} L{:.2},{:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/><ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"10\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",
            y+10.0,x+12.0,y+h-13.0,x+w/2.0,y+h-1.0,x+w-12.0,y+h-13.0,x+w,y+10.0,x+w/2.0,y+10.0,w/2.0).unwrap();
    } else if has("c4Database") {
        writeln!(svg,"<path d=\"M{x:.2},{:.2} A{:.2},10 0 0 1 {:.2},{:.2} V{:.2} A{:.2},10 0 0 1 {x:.2},{:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/><ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"10\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",
            y+10.0,w/2.0,x+w,y+10.0,y+h-10.0,w/2.0,y+h-10.0,x+w/2.0,y+10.0,w/2.0).unwrap();
    } else {
        writeln!(svg,"<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",node.radius).unwrap();
    }
    if has("c4Browser") {
        writeln!(svg,"<path d=\"M{x:.2},{:.2} H{:.2}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/><circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"2\" fill=\"{stroke}\"/><circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"2\" fill=\"{stroke}\"/>",y+15.0,x+w,x+9.0,y+8.0,x+17.0,y+8.0).unwrap();
    } else if has("c4Folder") {
        writeln!(svg,"<path d=\"M{:.2},{:.2} V{:.2} H{:.2} L{:.2},{:.2} H{:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",x+8.0,y+28.0,y+17.0,x+23.0,x+28.0,y+22.0,x+39.0).unwrap();
    } else if has("c4Terminal") {
        writeln!(svg,"<path d=\"M{:.2},{:.2} l7,5 -7,5 M{:.2},{:.2} h9\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",x+10.0,y+15.0,x+20.0,y+26.0).unwrap();
    } else if has("c4Component") {
        for offset in [0.0, 13.0] {
            writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"15\" height=\"9\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",x+7.0,y+17.0+offset).unwrap();
        }
    } else if has("c4Person") {
        writeln!(svg,"<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"7\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/><path d=\"M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{thick:.2}\"/>",x+20.0,y+19.0,x+7.0,y+42.0,x+20.0,y+23.0,x+33.0,y+42.0).unwrap();
    }
    true
}
