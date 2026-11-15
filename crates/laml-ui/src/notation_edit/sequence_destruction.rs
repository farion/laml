//! Source-only alignment of standalone destruction occurrences.
use super::*;
use laml::sequence::destruction_layout::{groups, standalone_line};

pub fn align(source: &str, owner: &str, target: &str) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    standalone_line(&doc, owner)?;
    standalone_line(&doc, target)?;
    if owner == target { return Err("Choose another destruction occurrence".into()); }
    let group = groups(&doc).into_iter().find(|g| g.iter().any(|id| id == owner));
    if group.as_ref().is_some_and(|g| g.iter().any(|id| id == target)) { return Ok(vec![]); }
    let mut edits = Vec::new();
    let mut declarations = Vec::new();
    if let Some(group) = group {
        // Detach just the dragged member, retaining the old peers' alignment.
        for rule in &doc.destruction_layout {
            if !group.contains(&rule.first) { continue; }
            let record = record(source, rule.line)?;
            let raw = &source[record.range.clone()];
            let body = code(raw);
            edits.push(Edit { range: record.range.start + body.len() - body.trim_start().len()
                ..record.range.start + body.trim_end().len(), replacement: String::new() });
        }
        let peers: Vec<_> = group.iter().filter(|id| id.as_str() != owner).collect();
        for pair in peers.windows(2) {
            declarations.push(format!("{}! ? {}! : align=center-y", pair[0], pair[1]));
        }
    }
    declarations.push(format!("{owner}! ? {target}! : align=center-y"));
    edits.extend(insert(source, "layout", None, None, &declarations.join("\n"))?);
    let candidate = apply(source, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds_layout_without_changing_semantics_and_keeps_crlf() {
        let source = "settings {\r\n type: sequence\r\n}\r\nsemantics {\r\n A\r\n B\r\n A!\r\n B!\r\n}\r\n";
        let edits = align(source, "A", "B").unwrap();
        let result = apply(source, &edits).unwrap();
        assert!(result.starts_with(source));
        assert!(result.contains("A! ? B! : align=center-y\r\n"));
        assert!(align(&result, "A", "B").unwrap().is_empty());
    }
    #[test]
    fn moving_one_member_keeps_its_peers_aligned_and_preserves_comments() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n D\n A!\n B!\n C!\n D!\n}\nlayout {\n A! ? B! : align=center-y // retain this\n A! ? C! : align=center-y\n}\n";
        let result = apply(source, &align(source, "A", "D").unwrap()).unwrap();
        assert!(result.contains("// retain this"));
        let doc = laml::sequence::parse(&result).unwrap();
        let groups = groups(&doc);
        assert!(groups.iter().any(|g| g.len() == 2 && g.contains(&"B".into()) && g.contains(&"C".into())));
        assert!(groups.iter().any(|g| g.len() == 2 && g.contains(&"A".into()) && g.contains(&"D".into())));
    }
}
