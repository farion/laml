//! Small, per-owner cache of read-only transient render contexts.
use super::*;
use std::{collections::BTreeMap, path::Path};
struct Context {
    presentation: Presentation,
    source: String,
    address: String,
    revision: u64,
    preview: Preview,
}
#[derive(Default)]
pub(super) struct Cache {
    contexts: BTreeMap<(u64, u64), Context>,
    limits: bool,
}
impl Cache {
    pub fn set_limits(&mut self, limits: bool) {
        self.limits = limits;
    }
    pub fn close(&mut self, stamp: Stamp) {
        self.contexts.remove(&(stamp.session, stamp.document));
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        backend: &crate::backend::Handle,
        stamp: Stamp,
        source: String,
        address: String,
        route: Option<Box<laml::route::Route>>,
        bounds: (f64, f64, f64, f64),
        remove_line: Option<usize>,
        presentation: Presentation,
    ) -> Result<Reply, Failure> {
        if ![bounds.0, bounds.1, bounds.2, bounds.3]
            .iter()
            .all(|v| v.is_finite() && (!self.limits || v.abs() <= 1e9))
            || bounds.2 <= bounds.0
            || bounds.3 <= bounds.1
        {
            return Err(Failure::protocol("Invalid render bounds"));
        }
        let owner = (stamp.session, stamp.document);
        let current = self.contexts.get(&owner).is_some_and(|c| {
            c.presentation == presentation
                && c.source == source
                && c.address == address
                && c.revision == stamp.revision
        });
        if !current {
            // Bound retained geometry as well as owner count; no unbounded gesture history.
            self.contexts.clear();
            let preview = backend
                .compile(&source, Path::new(&address), presentation)
                .map_err(|e| Failure::backend(e.to_string()))?;
            if self.limits
                && serde_json::to_vec(&preview)
                    .map_err(|e| Failure::backend(e.to_string()))?
                    .len()
                    > 32 * 1024 * 1024
            {
                return Err(Failure::new(
                    FailureCode::Limit,
                    "Gesture context too large",
                ));
            }
            self.contexts.insert(
                owner,
                Context {
                    presentation,
                    source,
                    address,
                    revision: stamp.revision,
                    preview,
                },
            );
        }
        let context = &self.contexts[&owner];
        let path = Path::new(&context.address);
        let base = remove_line
            .map(|line| backend.without_route(&context.preview, &context.source, path, line))
            .transpose()
            .map_err(|e| Failure::backend(e.to_string()))?;
        let overlay = route
            .map(|route| {
                if (self.limits && (route.points.len() > 4096 || route.end_labels.len() > 64))
                    || route.from >= context.preview.nodes.len()
                    || route.to >= context.preview.nodes.len()
                    || route.points.iter().any(|p| {
                        !p.x.is_finite()
                            || !p.y.is_finite()
                            || (self.limits && (p.x.abs() > 1e9 || p.y.abs() > 1e9))
                    })
                {
                    return Err("Invalid transient route".to_owned());
                }
                backend.route_overlay(&context.preview, &context.source, path, &route, bounds)
            })
            .transpose()
            .map_err(|e| Failure::backend(e.to_string()))?;
        Ok(Reply::GestureSvg { base, overlay })
    }
}
