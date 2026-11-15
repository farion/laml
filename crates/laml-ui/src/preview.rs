//! GNE preview uses the compiler's real layout and routing, not a second renderer.

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PickLabel {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PickRoute {
    pub line: usize,
    pub paths: Vec<Vec<laml::route::Point>>,
    pub labels: Vec<PickLabel>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Preview {
    pub svg: String,
    pub diagram: laml::syntax::Diagram,
    pub styles: laml::styles::StyleSheet,
    pub nodes: Vec<laml::geometry::BoxNode>,
    pub routes: Vec<laml::route::Route>,
    pub route_paths: Vec<PickRoute>,
    pub relation_properties: crate::properties::RelationValues,
    pub node_properties: crate::properties::Values,
    pub port_properties: crate::properties::Values,
    pub bounds: (f64, f64, f64, f64),
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod incremental;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
