use super::{Element, Kind, Level, Model, Relation, View};
use crate::import::{Imported, identifier, line_error, words};
use std::collections::HashSet;

fn resolve(model: &Model, alias: &str) -> Result<String, String> {
    model
        .elements
        .iter()
        .find(|e| e.alias == alias)
        .map(|e| e.id.clone())
        .ok_or_else(|| format!("unknown Structurizr identifier '{alias}'"))
}
fn all_visible(model: &Model, view: &View) -> HashSet<String> {
    model
        .elements
        .iter()
        .filter(|e| match view.level {
            Level::Context => matches!(e.kind, Kind::Person | Kind::System),
            Level::Container => {
                matches!(e.kind, Kind::Person | Kind::System)
                    || e.kind == Kind::Container && e.parent.as_deref() == Some(view.scope.as_str())
            }
            Level::Component => {
                (e.kind == Kind::Person
                    || e.kind == Kind::System
                        && e.id
                            != model
                                .element(&view.scope)
                                .ok()
                                .and_then(|scope| scope.parent.clone())
                                .unwrap_or_default())
                    || e.kind == Kind::Container
                        && e.parent
                            == model
                                .element(&view.scope)
                                .ok()
                                .and_then(|scope| scope.parent.clone())
                    || e.kind == Kind::Component && e.parent.as_deref() == Some(view.scope.as_str())
            }
        })
        .map(|e| e.id.clone())
        .collect()
}
pub(super) fn parse(text: &str) -> Result<Vec<Imported>, String> {
    let mut model = Model::default();
    let mut stack: Vec<String> = Vec::new();
    let mut all_views = HashSet::new();
    for (line, raw) in text.lines().enumerate() {
        let n = line + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with("//") || text.starts_with('#') {
            continue;
        }
        if text == "}" {
            stack.pop().ok_or_else(|| line_error(n, "unmatched }"))?;
            continue;
        }
        let tokens = words(text).map_err(|e| line_error(n, e))?;
        let open = tokens.last().is_some_and(|s| s == "{");
        let args = &tokens[..tokens.len() - usize::from(open)];
        if args.is_empty() || args.iter().any(|t| t == "}" || t == "{") {
            return Err(line_error(
                n,
                "put each declaration and closing brace on its own line",
            ));
        }
        let context = stack.last().map(String::as_str).unwrap_or("");
        match context {
            "" if args.first().is_some_and(|s| s == "workspace") && open => {
                stack.push("workspace".into())
            }
            "workspace" if args == ["model"] && open => stack.push("model".into()),
            "workspace" if args == ["views"] && open => stack.push("views".into()),
            ctx if (ctx == "model" || ctx.starts_with("element:"))
                && args.len() >= 3
                && args[1] == "->" =>
            {
                if open {
                    return Err(line_error(n, "relationship blocks need a separate mapping"));
                }
                if args.len() > 5 {
                    return Err(line_error(n, "relationship tags need a mapping"));
                }
                let endpoint = |alias: &str| {
                    if alias == "this" {
                        ctx.strip_prefix("element:")
                            .map(str::to_owned)
                            .ok_or("'this' requires an element block".to_owned())
                    } else {
                        resolve(&model, alias)
                    }
                };
                let from = endpoint(&args[0]).map_err(|e| line_error(n, e))?;
                let to = endpoint(&args[2]).map_err(|e| line_error(n, e))?;
                let id = format!("R{}", model.relations.len() + 1);
                model.relations.push(Relation {
                    id,
                    from,
                    to,
                    description: args.get(3).cloned().unwrap_or_default(),
                    technology: args.get(4).cloned().unwrap_or_default(),
                });
            }
            ctx if (ctx == "model" || ctx.starts_with("element:"))
                && args.len() >= 3
                && args[1] == "="
                && Kind::parse(&args[2]).is_some() =>
            {
                let kind = Kind::parse(&args[2]).unwrap();
                let alias = identifier(&args[0]).map_err(|e| line_error(n, e))?;
                let fields = &args[3..];
                if fields.is_empty()
                    || fields.len()
                        > if matches!(kind, Kind::Person | Kind::System) {
                            2
                        } else {
                            3
                        }
                {
                    return Err(line_error(
                        n,
                        "element requires name and optional description/technology; tags need a mapping",
                    ));
                }
                let parent = stack
                    .iter()
                    .rev()
                    .find_map(|s| s.strip_prefix("element:"))
                    .map(str::to_owned);
                let valid_parent = match kind {
                    Kind::Person | Kind::System => parent.is_none(),
                    Kind::Container => parent
                        .as_deref()
                        .and_then(|p| model.element(p).ok())
                        .is_some_and(|e| e.kind == Kind::System),
                    Kind::Component => parent
                        .as_deref()
                        .and_then(|p| model.element(p).ok())
                        .is_some_and(|e| e.kind == Kind::Container),
                };
                if !valid_parent {
                    return Err(line_error(n, "element is outside its C4 parent"));
                }
                model
                    .add(Element {
                        id: alias.clone(),
                        alias: alias.clone(),
                        name: fields[0].clone(),
                        description: fields.get(1).cloned().unwrap_or_default(),
                        technology: fields.get(2).cloned().unwrap_or_default(),
                        kind,
                        parent,
                    })
                    .map_err(|e| line_error(n, e))?;
                if open {
                    stack.push(format!("element:{alias}"));
                }
            }
            "views"
                if open
                    && matches!(
                        args.first().map(String::as_str),
                        Some("systemContext" | "container" | "component")
                    ) =>
            {
                if args.len() < 2 || args.len() > 3 {
                    return Err(line_error(n, "view expects scope and optional key"));
                }
                let level = match args[0].as_str() {
                    "systemContext" => Level::Context,
                    "container" => Level::Container,
                    _ => Level::Component,
                };
                let scope = resolve(&model, &args[1]).map_err(|e| line_error(n, e))?;
                let key = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| format!("View{}", model.views.len() + 1));
                identifier(&key).map_err(|e| line_error(n, e))?;
                model.views.push(View {
                    key,
                    scope,
                    level,
                    shown: HashSet::new(),
                    links: None,
                    direction: None,
                });
                stack.push(format!("view:{}", model.views.len() - 1));
            }
            ctx if ctx.starts_with("element:")
                && !open
                && args.len() == 2
                && matches!(args[0].as_str(), "description" | "technology") =>
            {
                let id = ctx.trim_start_matches("element:");
                let e = model
                    .elements
                    .iter_mut()
                    .find(|e| e.id == id)
                    .expect("element context");
                if args[0] == "description" {
                    e.description = args[1].clone();
                } else {
                    e.technology = args[1].clone();
                }
            }
            ctx if ctx.starts_with("view:") && args.len() == 2 && !open => {
                let i = ctx
                    .trim_start_matches("view:")
                    .parse::<usize>()
                    .expect("view context");
                if args[0] == "include" {
                    if args[1] == "*" {
                        all_views.insert(i);
                    } else {
                        let id = resolve(&model, &args[1]).map_err(|e| line_error(n, e))?;
                        model.views[i].shown.insert(id);
                    }
                } else if args[0] == "autoLayout" {
                    let direction = match args[1].to_ascii_lowercase().as_str() {
                        "lr" => "left-to-right",
                        "rl" => "right-to-left",
                        "tb" => "top-to-bottom",
                        "bt" => "bottom-to-top",
                        _ => {
                            return Err(line_error(
                                n,
                                "autoLayout direction needs LR, RL, TB or BT",
                            ));
                        }
                    };
                    model.views[i].direction = Some(direction.into());
                } else {
                    return Err(line_error(n, format!("unmapped view construct: {text}")));
                }
            }
            _ => {
                return Err(line_error(
                    n,
                    format!("unmapped Structurizr DSL construct: {text}"),
                ));
            }
        }
    }
    if !stack.is_empty() {
        return Err("unclosed Structurizr DSL block".into());
    }
    for i in all_views {
        let visible = all_visible(&model, &model.views[i]);
        model.views[i].shown.extend(visible);
    }
    model.render()
}
