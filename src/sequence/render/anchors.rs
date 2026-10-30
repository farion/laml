//! Rendered occurrence positions, including different send/receive heights on self messages.

use crate::sequence::AnnotationEnd;

pub(super) struct MessageAnchor {
    pub id: String,
    pub send: (f64, f64),
    pub receive: (f64, f64),
}

pub(super) fn position(
    anchors: &[MessageAnchor],
    name: &str,
    end: AnnotationEnd,
) -> Result<(f64, f64), String> {
    let anchor = anchors
        .iter()
        .find(|a| a.id == name)
        .ok_or_else(|| format!("missing named event '{name}'"))?;
    Ok(if end == AnnotationEnd::Send {
        anchor.send
    } else {
        anchor.receive
    })
}
