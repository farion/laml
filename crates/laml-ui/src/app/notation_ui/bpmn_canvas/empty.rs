//! Hover-local interior insertion points for empty BPMN containers.
use super::*;

pub(super) fn position(r: &Region, doc: &laml::bpmn::Document) -> Option<(f64, f64)> {
    let empty = match r.kind.as_str() {
        "pool" => doc
            .pools
            .iter()
            .find(|p| p.line == r.line)
            .is_some_and(|p| {
                p.lanes.is_empty() && !doc.nodes.iter().any(|n| n.pool.as_deref() == Some(&p.id))
            }),
        "lane" => doc
            .pools
            .iter()
            .find_map(|p| p.lanes.iter().find(|l| l.line == r.line).map(|l| (p, l)))
            .is_some_and(|(p, l)| {
                !doc.nodes
                    .iter()
                    .any(|n| n.pool.as_deref() == Some(&p.id) && n.lane.as_deref() == Some(&l.id))
            }),
        "node" => doc.nodes.iter().find(|n| n.id == r.id).is_some_and(|n| {
            n.kind == laml::bpmn::Kind::Subprocess
                && !doc
                    .nodes
                    .iter()
                    .any(|c| c.pool == n.pool && c.parent.as_deref() == n.id.rsplit('.').next())
        }),
        _ => false,
    };
    empty.then_some((r.x + r.width / 2.0, r.y + r.height / 2.0 + 12.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_bpmn_container_centers_exclude_ordinary_tasks() {
        for (body, expected) in [
            ("pool P {\n}", "pool"),
            ("pool P {\n lane L {\n }\n}", "lane"),
            ("S {\n}", "node"),
        ] {
            let source = format!("settings {{\n type: bpmn\n}}\nsemantics {{\n{body}\n}}\n");
            let doc = laml::bpmn::parse(&source).unwrap();
            let preview =
                laml_ui::preview::build(&source, std::path::Path::new("empty.laml")).unwrap();
            assert!(
                preview
                    .regions
                    .iter()
                    .any(|r| r.kind == expected && position(r, &doc).is_some()),
                "{body}"
            );
        }
    }
}
