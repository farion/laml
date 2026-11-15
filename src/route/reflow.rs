//! Shared compiler orchestration: expand automatic layout only after routing fails.
use super::{Route, RouteError, route};
use crate::{
    geometry::{self, BoxNode},
    styles::StyleSheet,
    syntax::{Diagram, DiagramType},
};

/// Compute geometry and routes together. Editor and CLI use this same entry point.
/// All retries derive from the current source, preserve mandatory constraints,
/// and never mutate source or styles. `route` remains available for fixed boxes.
pub fn place_and_route(
    diagram: &Diagram,
    sheet: &StyleSheet,
) -> Result<(Vec<BoxNode>, Vec<Route>), RouteError> {
    let boxes = geometry::place(diagram, sheet).map_err(|e| RouteError {
        line: e.line,
        message: e.message,
    })?;
    let mut failure = match route(diagram, &boxes, sheet) {
        Ok(routes) => return Ok((boxes, routes)),
        Err(error) => error,
    };
    if !matches!(
        diagram.settings.diagram_type,
        DiagramType::Generic | DiagramType::Package
    ) {
        return Err(failure);
    }
    // Exponential expansion is deterministic. Resource limits prevent runaway
    // diagrams, not a small arbitrary number of allowed bends or crossings.
    for spacing in [32.0, 64.0, 128.0, 256.0, 512.0, 1024.0] {
        let Ok(boxes) = geometry::place_with_spacing(diagram, sheet, spacing) else {
            continue;
        };
        match route(diagram, &boxes, sheet) {
            Ok(routes) => return Ok((boxes, routes)),
            Err(error) => failure = error,
        }
    }
    failure.message.push_str(". Expanded path search and automatic spacing retries were exhausted; no proof of impossibility was established. Check mandatory endpoint/layout constraints.");
    Err(failure)
}
