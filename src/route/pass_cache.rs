//! Reuse deterministic routing prefixes only within one fixed-geometry search.
//! Every entry includes crossing/label changes to earlier routes. A new public
//! routing call (and each spacing retry) owns a fresh cache: no layout history.
use super::{Route, RouteError, router};
use crate::{geometry::BoxNode, styles::StyleSheet, syntax::Diagram};
use std::collections::BTreeMap;

type Prefix = Vec<(usize, bool)>;
#[derive(Default)]
pub(super) struct PassCache {
    pub corridor: bool,
    pub crossing_free: bool,
    entries: BTreeMap<(bool, Prefix), Vec<Route>>,
    #[cfg(test)]
    pub hits: usize,
    #[cfg(test)]
    pub disabled: bool,
}
impl PassCache {
    pub fn get(&mut self, rescue: bool, prefix: &[(usize, bool)]) -> Option<Vec<Route>> {
        #[cfg(test)]
        if self.disabled {
            return None;
        }
        let routes = self.entries.get(&(rescue, prefix.to_vec()))?.clone();
        #[cfg(test)]
        {
            self.hits += 1;
        }
        Some(routes)
    }
    pub fn insert(&mut self, rescue: bool, prefix: &[(usize, bool)], routes: &[Route]) {
        #[cfg(test)]
        if self.disabled {
            return;
        }
        self.entries
            .insert((rescue, prefix.to_vec()), routes.to_vec());
    }
    pub fn pass(
        &mut self,
        diagram: &Diagram,
        boxes: &[BoxNode],
        sheet: &StyleSheet,
        ranked_until: Option<usize>,
    ) -> Result<Vec<Route>, RouteError> {
        self.run(
            diagram,
            boxes,
            sheet,
            ranked_until,
            &(0..diagram.relations.len()).collect::<Vec<_>>(),
            false,
        )
    }
    #[allow(clippy::too_many_arguments)] // One fixed-geometry retry and its search mode.
    pub fn run(
        &mut self,
        diagram: &Diagram,
        boxes: &[BoxNode],
        sheet: &StyleSheet,
        ranked_until: Option<usize>,
        order: &[usize],
        rescue: bool,
    ) -> Result<Vec<Route>, RouteError> {
        router::route_in_order_mode(
            diagram,
            boxes,
            sheet,
            ranked_until,
            order,
            rescue,
            None,
            self,
        )
    }
}

#[cfg(test)]
#[path = "pass_cache_tests.rs"]
mod tests;
