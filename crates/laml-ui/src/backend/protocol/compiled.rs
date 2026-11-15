//! Last successful preview per document; never accept geometry from the client.
use super::*;
use crate::backend::{DependencyWatch, Handle};
use std::{collections::BTreeMap, path::Path};

struct Context {
    source: String,
    address: String,
    presentation: Presentation,
    revision: u64,
    preview: Preview,
    dependencies: Box<dyn DependencyWatch>,
}
#[derive(Default)]
pub(super) struct Cache {
    contexts: BTreeMap<(u64, u64), Context>,
}
impl Cache {
    pub fn close(&mut self, stamp: Stamp) {
        self.contexts.remove(&(stamp.session, stamp.document));
    }
    pub fn compile(
        &mut self,
        backend: &Handle,
        stamp: Stamp,
        source: String,
        address: String,
        presentation: Presentation,
    ) -> Reply {
        let owner = (stamp.session, stamp.document);
        let path = Path::new(&address);
        let prior = self.contexts.get(&owner).filter(|c| {
            c.address == address
                && c.presentation == presentation
                && c.revision <= stamp.revision
                && matches!(c.dependencies.changed(), Ok(false))
        });
        let result = match prior {
            Some(c) if c.source == source => Ok(c.preview.clone()),
            Some(c) => backend.compile_relations(&source, path, presentation, &c.preview),
            None => backend.compile(&source, path, presentation),
        };
        if let Ok(preview) = &result {
            let dependencies = backend.watch_dependencies(&source, path);
            self.contexts.insert(
                owner,
                Context {
                    source,
                    address,
                    presentation,
                    revision: stamp.revision,
                    preview: preview.clone(),
                    dependencies,
                },
            );
        }
        Reply::Compiled(result.map(Box::new))
    }
}
