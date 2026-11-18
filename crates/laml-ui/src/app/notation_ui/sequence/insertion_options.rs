//! Revision-bound nonblocking feasibility cache shared by menus and reference picking.
use super::*;
pub(super) use edit::sequence_options::{Choice, Query};
use laml_ui::backend::{
    Presentation, async_client,
    protocol::{self, Operation, Reply, Stamp},
};

pub(super) struct Cache {
    queries: Vec<Query>,
    references: bool,
    presentation: Presentation,
    service: async_client::Client,
    stamp: Stamp,
    result: Option<Result<Vec<Choice>, String>>,
}
impl Drop for Cache {
    fn drop(&mut self) {
        self.service.forget(self.stamp);
    }
}
impl Editor {
    pub(super) fn sequence_valid_options(
        &mut self,
        ctx: &egui::Context,
        queries: Vec<Query>,
        references: bool,
    ) -> Result<Option<Vec<Choice>>, String> {
        let unchanged = self.notation_ui.sequence.options.as_ref().is_some_and(|c| {
            c.queries == queries
                && c.references == references
                && c.presentation == self.compiler.presentation
                && c.stamp.revision == self.document.revision()
        });
        if !unchanged {
            let mut service = self.backend.client();
            let stamp = service
                .submit(
                    self.id,
                    self.document.revision(),
                    Operation::SequenceOptions {
                        source: self.document.source().into(),
                        address: protocol::address(&self.path).map_err(|e| e.to_string())?,
                        presentation: self.compiler.presentation,
                        queries: queries.clone(),
                        references,
                    },
                )
                .map_err(|e| e.to_string())?;
            self.notation_ui.sequence.options = Some(Cache {
                queries,
                references,
                presentation: self.compiler.presentation,
                service,
                stamp,
                result: None,
            });
        }
        let cache = self.notation_ui.sequence.options.as_mut().unwrap();
        if let Some(response) = cache.service.poll() {
            cache.result = Some(match response.result {
                Ok(Reply::SequenceOptions(choices)) => Ok(choices),
                Err(e) => Err(e.to_string()),
                _ => Err("Invalid insertion option response".into()),
            });
        }
        match &cache.result {
            Some(result) => result.clone().map(Some),
            None => {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
                Ok(None)
            }
        }
    }
}
pub(super) fn query(
    editor: &Editor,
    recipe: edit::catalog::Recipe,
    point: Option<&Point>,
) -> Query {
    let mut values = super::insertion::defaults(editor, recipe);
    values.insert("gap".into(), "24".into());
    values.insert(
        "id".into(),
        edit::sequence_create::fresh(editor.document.source(), "Element"),
    );
    if let Some(point) = point {
        values.insert("participant".into(), point.dock.owner.clone());
        values.insert("from".into(), point.dock.owner.clone());
    }
    let keys = edit::catalog::parameters(recipe)
        .into_iter()
        .filter(|key| {
            edit::sequence_create::reference(key)
                && !(point.is_some() && matches!(key.as_str(), "participant" | "from"))
        })
        .map(|key| {
            let choices = reference_values(editor, point, &key);
            (key, choices)
        })
        .collect();
    Query {
        title: recipe.title.into(),
        values,
        keys,
        before: point.map_or(0, |p| p.dock.before),
        owner: point.map(|p| p.dock.owner.clone()),
    }
}
fn reference_values(editor: &Editor, point: Option<&Point>, key: &str) -> Vec<String> {
    let scope = point.and_then(|p| p.dock.owner.rsplit_once('.').map(|(s, _)| s));
    let mut out = Vec::new();
    if let Some(preview) = &editor.preview {
        for r in &preview.regions {
            if r.id.rsplit_once('.').map(|(s, _)| s) != scope {
                continue;
            }
            if matches!(key, "occurrence" | "first" | "second") {
                if r.kind == "message" && !r.path.is_empty() {
                    out.extend([format!("@{}.send", r.line), format!("@{}.receive", r.line)]);
                }
            } else {
                let kind = match key {
                    "frame" => "frame",
                    "note" => "note",
                    "continuation" => "continuation",
                    _ => "participant",
                };
                if r.kind == kind {
                    out.push(
                        if r.id
                            .rsplit('.')
                            .next()
                            .is_some_and(|id| id.starts_with("line:"))
                        {
                            format!("@{}.frame", r.line)
                        } else {
                            r.id.clone()
                        },
                    );
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
