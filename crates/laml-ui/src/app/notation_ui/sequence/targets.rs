//! Drop docks come from compiler lifelines, execution contours and event boundaries.
use laml::authoring::Region;
#[derive(Clone, Debug)]
pub(super) struct Target {
    pub owner: String,
    pub before: Option<usize>,
    pub x: f64,
    pub y: f64,
    pub row_y: f64,
    pub bar: Option<Region>,
    pub activation_line: Option<usize>,
}
pub(super) struct Baseline<'a> {
    pub before: Option<usize>,
    pub send_y: f64,
    pub moving_y: f64,
    pub moving_owner: Option<&'a str>,
}
/// Select exactly the same innermost active bar as the compiler's attachments.
pub(super) fn dock(regions: &[Region], owner: &str, y: f64, toward_x: f64, self_message: bool) -> Option<(f64, Option<Region>)> {
    let line = regions.iter().find(|r| r.kind == "lifeline" && r.id == owner && y >= r.y - 0.01 && y <= r.y+r.height+0.01)?;
    let bar = regions.iter().filter(|r| r.kind == "sequence-execution" && r.id == owner && y >= r.y - 0.01 && y <= r.y+r.height+0.01)
        .max_by(|a,b| (a.x+a.width/2.0).total_cmp(&(b.x+b.width/2.0)));
    if let Some(bar) = bar {
        let middle = bar.x + bar.width / 2.0;
        let side = if self_message || toward_x >= middle { 1.0 } else { -1.0 };
        Some((middle + side * bar.width / 2.0, Some(bar.clone())))
    } else { Some((line.x, None)) }
}
pub(super) fn all(regions: &[Region], fixed_owner: &str, fixed_x: f64, moving_source: bool, baseline: Baseline<'_>) -> Vec<Target> {
    let mut owners: Vec<_> = regions.iter().filter(|r| r.kind == "lifeline" && !r.id.contains('.')).map(|r| r.id.as_str()).collect();
    owners.sort(); owners.dedup();
    let slots: Vec<_> = regions.iter().filter(|r| r.kind == "sequence-slot").collect();
    let mut out = Vec::new();
    for owner in owners {
        let self_message = fixed_owner == owner;
        // A current-row dock avoids moving a message just to reconnect it horizontally.
        let current_y = if baseline.before.is_none() && baseline.moving_owner == Some(owner) { baseline.moving_y }
            else if self_message && !moving_source { baseline.send_y + 24.0 }
            else if baseline.before.is_none() && baseline.moving_owner != Some(fixed_owner) && !self_message { baseline.moving_y }
            else { baseline.send_y };
        let rows = std::iter::once((baseline.before, baseline.send_y, current_y))
            .chain(slots.iter().map(|slot| (Some(slot.line), slot.y, slot.y + if self_message && !moving_source { 24.0 } else { 0.0 })));
        for (before, row_y, y) in rows {
            // New insertion expands the interval. Dock using the source boundary,
            // rather than rejecting a self-call because its return leg needs space.
            let lookup_y = if before.is_some() { row_y } else { y };
            if let Some((x, bar)) = dock(regions, owner, lookup_y, fixed_x, self_message)
                && !out.iter().any(|t: &Target| t.owner == owner && (t.y-y).abs() < 0.1 && t.before == before) {
                out.push(Target { owner: owner.into(), before, x, y, row_y, bar, activation_line: None });
            }
        }
    }
    if !moving_source {
        for bar in regions.iter().filter(|r| r.kind == "sequence-execution" && !r.id.contains('.')) {
            let self_message = bar.id == fixed_owner;
            let center = bar.x + bar.width / 2.0;
            let x = if self_message || fixed_x >= center { bar.x + bar.width } else { bar.x };
            out.push(Target { owner: bar.id.clone(), before: Some(bar.line), x, y: bar.y,
                row_y: bar.y - if self_message { 24.0 } else { 0.0 }, bar: Some(bar.clone()), activation_line: Some(bar.line) });
        }
    }
    out
}
pub(super) fn hit<'a>(targets: &'a [Target], regions: &[Region], x: f64, y: f64, tolerance: f64) -> Option<&'a Target> {
    // The top edge is a semantic target, not the adjacent before/after slot.
    if let Some(top) = targets.iter().filter(|t| t.activation_line.is_some() && (t.y-y).abs() <= tolerance
        && t.bar.as_ref().is_some_and(|bar| x >= bar.x-tolerance && x <= bar.x+bar.width+tolerance))
        .min_by(|a,b| (a.x-x).hypot(a.y-y).total_cmp(&(b.x-x).hypot(b.y-y))) { return Some(top); }
    if let Some(target) = targets.iter().filter(|t| (t.x-x).hypot(t.y-y) <= tolerance)
        .min_by(|a,b| (a.x-x).hypot(a.y-y).total_cmp(&(b.x-x).hypot(b.y-y)).then(a.before.is_some().cmp(&b.before.is_some()))) { return Some(target); }
    let region = regions.iter().filter(|r| !r.id.contains('.') && matches!(r.kind.as_str(), "participant" | "lifeline" | "sequence-execution")
        && x >= r.x-tolerance && x <= r.x+r.width+tolerance && y >= r.y && y <= r.y+r.height)
        .min_by(|a,b| {
            let priority = |r: &Region| if r.kind == "sequence-execution" { 0 } else { 1 };
            priority(a).cmp(&priority(b)).then((x-a.x-a.width/2.0).abs().total_cmp(&(x-b.x-b.width/2.0).abs()))
        })?;
    let mut choices = targets.iter().filter(|t| t.owner == region.id).collect::<Vec<_>>();
    if region.kind == "participant" {
        choices.sort_by_key(|t| t.before.is_some());
        return choices.first().copied();
    }
    choices.into_iter().filter(|t| region.kind != "sequence-execution" || (t.row_y >= region.y && t.row_y <= region.y+region.height))
        .min_by(|a,b| (a.y-y).abs().total_cmp(&(b.y-y).abs()).then(a.before.is_some().cmp(&b.before.is_some())))
}
