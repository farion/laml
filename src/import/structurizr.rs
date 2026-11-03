use super::{Imported, identifier, quoted, unique_names};
mod dsl;
use serde_json::Value;

pub(super) fn dsl(text: &str) -> Result<Vec<Imported>, String> {
    dsl::parse(text)
}
use std::collections::HashSet;

#[derive(Clone)]
struct Element {
    id: String,
    alias: String,
    name: String,
    description: String,
    technology: String,
    kind: Kind,
    parent: Option<String>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Person,
    System,
    Container,
    Component,
}
impl Kind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "person" => Some(Self::Person),
            "softwareSystem" => Some(Self::System),
            "container" => Some(Self::Container),
            "component" => Some(Self::Component),
            _ => None,
        }
    }
    fn laml(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::System => "system",
            Self::Container => "container",
            Self::Component => "component",
        }
    }
}
#[derive(Clone)]
struct Relation {
    id: String,
    from: String,
    to: String,
    description: String,
    technology: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Context,
    Container,
    Component,
}
struct View {
    key: String,
    scope: String,
    level: Level,
    shown: HashSet<String>,
    links: Option<HashSet<String>>,
    direction: Option<String>,
}
#[derive(Default)]
struct Model {
    elements: Vec<Element>,
    relations: Vec<Relation>,
    views: Vec<View>,
}
impl Model {
    fn element(&self, id: &str) -> Result<&Element, String> {
        self.elements
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| format!("unknown model element '{id}'"))
    }
    fn add(&mut self, e: Element) -> Result<(), String> {
        identifier(&e.alias)?;
        if self
            .elements
            .iter()
            .any(|old| old.id == e.id || old.alias == e.alias)
        {
            return Err(format!("duplicate model identifier '{}'", e.alias));
        }
        self.elements.push(e);
        Ok(())
    }
    fn render(&self) -> Result<Vec<Imported>, String> {
        if self.views.is_empty() {
            return Err("workspace has no systemContext, container or component view".into());
        }
        let mut result = Vec::new();
        for view in &self.views {
            result.push(self.render_view(view)?);
        }
        unique_names(&result)?;
        Ok(result)
    }
    fn reference(&self, e: &Element, view: &View) -> Result<String, String> {
        if view.level == Level::Container && e.kind == Kind::Container {
            return Ok(format!("{}.{}", self.element(&view.scope)?.alias, e.alias));
        }
        if view.level == Level::Component && e.kind == Kind::Component {
            return Ok(format!("{}.{}", self.element(&view.scope)?.alias, e.alias));
        }
        Ok(e.alias.clone())
    }
    fn declaration(&self, e: &Element, view: &View) -> String {
        let external = e.kind == Kind::System && e.id != view.scope;
        let prefix = if external { "external " } else { "" };
        let mut attrs = format!(
            "label={}; description={}",
            quoted(&e.name),
            quoted(&e.description)
        );
        if matches!(e.kind, Kind::Container | Kind::Component) {
            attrs.push_str(&format!("; technology={}", quoted(&e.technology)));
        }
        format!("{prefix}{} {}({attrs})", e.kind.laml(), e.alias)
    }
    fn render_view(&self, view: &View) -> Result<Imported, String> {
        let scope = self.element(&view.scope)?;
        let expected = if view.level == Level::Component {
            Kind::Container
        } else {
            Kind::System
        };
        if scope.kind != expected {
            return Err(format!(
                "view '{}' scope has the wrong element kind",
                view.key
            ));
        }
        let diagram_type = match view.level {
            Level::Context => "c4-context",
            Level::Container => "c4-container",
            Level::Component => "c4-component",
        };
        let mut shown = view.shown.clone();
        for id in &shown {
            self.element(id)
                .map_err(|e| format!("view '{}': {e}", view.key))?;
        }
        shown.insert(view.scope.clone());
        let parent = if view.level == Level::Component {
            scope.parent.as_deref()
        } else {
            None
        };
        if parent.is_some_and(|id| shown.contains(id)) {
            return Err(format!(
                "view '{}' includes its parent system; an explicit boundary mapping is required",
                view.key
            ));
        }
        if let Some(links) = &view.links {
            for id in links {
                let relation = self.relations.iter().find(|r| &r.id == id).ok_or_else(|| {
                    format!("view '{}' refers to unknown relationship '{id}'", view.key)
                })?;
                if !shown.contains(&relation.from) || !shown.contains(&relation.to) {
                    return Err(format!(
                        "view '{}' includes relationship '{id}' without both endpoints",
                        view.key
                    ));
                }
            }
        }
        let mut out = format!(
            "settings {{\n    type: {diagram_type}\n    scope: {}\n",
            scope.alias
        );
        if let Some(direction) = &view.direction {
            out.push_str(&format!("    direction: {direction}\n"));
        }
        out.push_str("}\nsemantics {\n");
        if view.level == Level::Component {
            // A component view may show siblings and external systems around its scoped container.
            for e in &self.elements {
                if !shown.contains(&e.id)
                    || e.id == view.scope
                    || e.parent.as_deref() == Some(view.scope.as_str())
                    || Some(e.id.as_str()) == parent
                {
                    continue;
                }
                if e.kind == Kind::Component {
                    return Err(format!(
                        "view '{}' includes a component outside its scope",
                        view.key
                    ));
                }
                out.push_str(&format!("    {}\n", self.declaration(e, view)));
            }
            out.push_str(&format!("    {} {{\n", self.declaration(scope, view)));
            for e in &self.elements {
                if shown.contains(&e.id) && e.parent.as_deref() == Some(view.scope.as_str()) {
                    out.push_str(&format!("        {}\n", self.declaration(e, view)));
                }
            }
            out.push_str("    }\n");
        } else if view.level == Level::Container {
            for e in &self.elements {
                if !shown.contains(&e.id)
                    || e.id == view.scope
                    || e.parent.as_deref() == Some(view.scope.as_str())
                {
                    continue;
                }
                if !matches!(e.kind, Kind::Person | Kind::System) {
                    return Err(format!(
                        "view '{}' includes an element outside the container level",
                        view.key
                    ));
                }
                out.push_str(&format!("    {}\n", self.declaration(e, view)));
            }
            out.push_str(&format!("    {} {{\n", self.declaration(scope, view)));
            for e in &self.elements {
                if shown.contains(&e.id) && e.parent.as_deref() == Some(view.scope.as_str()) {
                    if e.kind != Kind::Container {
                        return Err(format!(
                            "view '{}' includes a non-container child",
                            view.key
                        ));
                    }
                    out.push_str(&format!("        {}\n", self.declaration(e, view)));
                }
            }
            out.push_str("    }\n");
        } else {
            for e in &self.elements {
                if !shown.contains(&e.id) || e.id == view.scope {
                    continue;
                }
                if !matches!(e.kind, Kind::Person | Kind::System) {
                    return Err(format!(
                        "view '{}' includes a non-context element",
                        view.key
                    ));
                }
                out.push_str(&format!("    {}\n", self.declaration(e, view)));
            }
            out.push_str(&format!("    {}\n", self.declaration(scope, view)));
        }
        for r in &self.relations {
            if let Some(links) = &view.links
                && !links.contains(&r.id)
            {
                continue;
            }
            if !shown.contains(&r.from) || !shown.contains(&r.to) {
                continue;
            }
            let a = self.element(&r.from)?;
            let b = self.element(&r.to)?;
            if view.level == Level::Component
                && (Some(a.id.as_str()) == parent || Some(b.id.as_str()) == parent)
            {
                return Err(format!(
                    "view '{}' has a relationship to its parent system boundary",
                    view.key
                ));
            }
            let (a, b) = (self.reference(a, view)?, self.reference(b, view)?);
            let label = if r.description.is_empty() {
                String::new()
            } else {
                format!(" : {}", r.description)
            };
            let tech = if r.technology.is_empty() {
                String::new()
            } else {
                format!("(technology={})", quoted(&r.technology))
            };
            out.push_str(&format!(
                "    @{}{} {a} -> {b}{label}\n",
                safe_id(&r.id),
                tech
            ));
        }
        out.push_str("}\n");
        Ok(Imported {
            name: view.key.clone(),
            source: out,
        })
    }
}

fn safe_id(raw: &str) -> String {
    let mut id = String::from("E");
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() {
            id.push(byte as char);
        } else {
            id.push_str(&format!("_{byte:02x}"));
        }
    }
    id
}
fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    match value.get(key) {
        None => Ok(&[]),
        Some(Value::Array(a)) => Ok(a),
        _ => Err(format!("'{key}' must be an array")),
    }
}
fn json_elements(
    model: &mut Model,
    parent: Option<&str>,
    list: &[Value],
    kind: Kind,
) -> Result<(), String> {
    for element in list {
        let id = string(element, "id");
        if id.is_empty() {
            return Err("model element has no id".into());
        }
        let alias = safe_id(&id);
        model.add(Element {
            id: id.clone(),
            alias,
            name: string(element, "name"),
            description: string(element, "description"),
            technology: string(element, "technology"),
            kind,
            parent: parent.map(str::to_owned),
        })?;
        for rel in array(element, "relationships")? {
            let rid = string(rel, "id");
            if rid.is_empty() {
                return Err(format!("relationship from '{id}' has no id"));
            }
            model.relations.push(Relation {
                id: rid,
                from: string(rel, "sourceId"),
                to: string(rel, "destinationId"),
                description: string(rel, "description"),
                technology: string(rel, "technology"),
            });
        }
        if kind == Kind::System {
            json_elements(
                model,
                Some(&id),
                array(element, "containers")?,
                Kind::Container,
            )?;
        }
        if kind == Kind::Container {
            json_elements(
                model,
                Some(&id),
                array(element, "components")?,
                Kind::Component,
            )?;
        }
    }
    Ok(())
}
pub(super) fn json(text: &str) -> Result<Vec<Imported>, String> {
    let document: Value =
        serde_json::from_str(text).map_err(|e| format!("invalid Structurizr JSON: {e}"))?;
    let model_value = document
        .get("model")
        .ok_or("workspace JSON requires 'model'")?;
    let mut model = Model::default();
    json_elements(
        &mut model,
        None,
        array(model_value, "people")?,
        Kind::Person,
    )?;
    json_elements(
        &mut model,
        None,
        array(model_value, "softwareSystems")?,
        Kind::System,
    )?;
    let views = document
        .get("views")
        .ok_or("workspace JSON requires 'views'")?;
    if let Some(styles) = views.get("configuration").and_then(|c| c.get("styles"))
        && (!array(styles, "elements")?.is_empty() || !array(styles, "relationships")?.is_empty())
    {
        return Err("workspace has custom visual styles; a style mapping is required".into());
    }
    for (key, level) in [
        ("systemContextViews", Level::Context),
        ("containerViews", Level::Container),
        ("componentViews", Level::Component),
    ] {
        for view in array(views, key)? {
            let scope = if level == Level::Component {
                string(view, "containerId")
            } else {
                string(view, "softwareSystemId")
            };
            let view_key = string(view, "key");
            identifier(&view_key)?;
            let mut shown = HashSet::new();
            for e in array(view, "elements")? {
                shown.insert(string(e, "id"));
            }
            let mut links = HashSet::new();
            for r in array(view, "relationships")? {
                links.insert(string(r, "id"));
            }
            model.views.push(View {
                key: view_key,
                scope,
                level,
                shown,
                links: Some(links),
                direction: None,
            });
        }
    }
    for unsupported in [
        "systemLandscapeViews",
        "dynamicViews",
        "deploymentViews",
        "filteredViews",
        "customViews",
        "imageViews",
    ] {
        if !array(views, unsupported)?.is_empty() {
            return Err(format!("{unsupported} needs its own diagram mapping"));
        }
    }
    let mut seen = HashSet::new();
    for relation in &model.relations {
        if !seen.insert(&relation.id) {
            return Err(format!("duplicate relationship id '{}'", relation.id));
        }
        model.element(&relation.from)?;
        model.element(&relation.to)?;
    }
    model.render()
}
