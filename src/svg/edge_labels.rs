use crate::route::Route;
use crate::syntax::NodeKind;
use crate::text::measure;
use crate::theme::Theme;
use std::fmt::Write;

use super::escape::escape;

pub(super) fn draw(svg: &mut String, route: &Route, theme: Theme) -> Result<(), String> {
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
            // Rotate the text-sized pill, not the collision envelope of rotated text.
            let (pill_width, pill_height) = if label.angle.abs() > 1e-9 {
                (
                    measure(&label.text, route.font_size)?,
                    route.font_size * 1.25,
                )
            } else {
                (label.width, label.height)
            };
            writeln!(svg, "<rect data-edge-label-pill=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" fill=\"{}\"{rotation}/>",
                cx-pill_width/2.0-4.0,cy-pill_height/2.0-2.0,pill_width+8.0,pill_height+4.0,neutral.fill).unwrap();
        }
        writeln!(svg,"<text x=\"{cx:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\"{}>{}</text>",
            cy+route.font_size*1.25*0.28,route.font_size,route.color,rotation,escape(&label.text)).unwrap();
    }
    Ok(())
}
