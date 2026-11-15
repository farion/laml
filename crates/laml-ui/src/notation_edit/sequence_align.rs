//! Create stable activation-edge references and explicit equal-height rules.
use super::*;
use laml::sequence::activation_layout::{executions, groups, resolve};

/// Detach only the edited edge. Other members and the opposite edge retain
/// their alignments; comments and unrelated layout remain untouched.
fn detach(source: &str, owner: &str, start: usize, lower: bool) -> Result<Vec<Edit>, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let is_owner = |name: &str| resolve(&doc, name).is_ok_and(|e| e.owner == owner && e.start == start);
    let Some(group) = groups(&doc, lower).into_iter().find(|g| g.iter().any(|name| is_owner(name))) else { return Ok(vec![]); };
    let mut edits = Vec::new();
    for rule in &doc.activation_layout {
        if rule.lower != lower || !group.contains(&rule.first) { continue; }
        let r = record(source, rule.line)?;
        let body = code(&source[r.range.clone()]);
        edits.push(Edit { range: r.range.start + body.len() - body.trim_start().len()..r.range.start + body.trim_end().len(), replacement: String::new() });
    }
    let peers: Vec<_> = group.iter().filter(|name| !is_owner(name)).collect();
    let suffix = if lower { '-' } else { '+' };
    let rules = peers.windows(2).map(|pair| format!("{}{suffix} ? {}{suffix} : align=center-y", pair[0], pair[1])).collect::<Vec<_>>();
    if !rules.is_empty() { edits.extend(insert(source, "layout", None, None, &rules.join("\n"))?); }
    Ok(edits)
}

#[allow(clippy::too_many_arguments)]
pub fn resize(source: &str, owner: &str, start: usize, edge: usize, lower: bool,
    open: bool, before: usize) -> Result<(Vec<Edit>, usize), String> {
    let detached = detach(source, owner, start, lower)?;
    let map = |line| if line == 0 { 0 } else { crate::relation_edit::remap_line(source, line, &detached) };
    let (start, edge, before) = (map(start), map(edge), map(before));
    let intermediate = apply(source, &detached)?;
    let (movement, moved_line) = if open {
        super::sequence_drag::close_open(&intermediate, start, before)?
    } else { super::sequence_drag::move_event(&intermediate, edge, before)? };
    let selected = if lower { crate::relation_edit::remap_line(&intermediate, start, &movement) } else { moved_line };
    let candidate = apply(&intermediate, &movement)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok((crate::graph_edit::difference(source, &candidate), selected))
}

fn reference(source: &mut String, owner: &str, start: usize, lower: bool) -> Result<String, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    let e = executions(&doc).into_iter().find(|e| e.start == start && e.owner == owner)
        .ok_or("The activation no longer exists")?;
    if lower && e.end.is_none() {
        let (edits, _) = super::sequence_drag::close_open(source, start, 0)?;
        *source = apply(source, &edits)?;
    }
    let name = if let Some(name) = e.name { name } else {
        let name = (1..).map(|n| format!("activation{n}")).find(|name| !source.contains(name)).unwrap();
        let edits = super::sequence::assign_name(source, start, &name)?;
        *source = apply(source, &edits)?;
        name
    };
    Ok(format!("{owner}.{name}"))
}
pub fn align(source: &str, owner: &str, start: usize, target_owner: &str, target_start: usize,
    lower: bool) -> Result<(Vec<Edit>, usize), String> {
    if start == target_start && owner == target_owner { return Ok((vec![], start)); }
    let mut candidate = source.to_owned();
    // Naming adds no lines; closing an open root activation appends at the end
    // of semantics, leaving existing occurrence line addresses intact.
    let from = reference(&mut candidate, owner, start, lower)?;
    let to = reference(&mut candidate, target_owner, target_start, lower)?;
    let doc = laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    let suffix = if lower { '-' } else { '+' };
    let old_group = groups(&doc, lower).into_iter().find(|g| g.contains(&from));
    if old_group.as_ref().is_some_and(|g| g.contains(&to)) { return Ok((crate::graph_edit::difference(source, &candidate), start)); }
    let mut edits = Vec::new();
    let mut rules = Vec::new();
    if let Some(group) = old_group {
        for rule in &doc.activation_layout {
            if rule.lower != lower || !group.contains(&rule.first) { continue; }
            let r = record(&candidate, rule.line)?;
            let body = code(&candidate[r.range.clone()]);
            edits.push(Edit { range: r.range.start + body.len() - body.trim_start().len()..r.range.start + body.trim_end().len(), replacement: String::new() });
        }
        let peers: Vec<_> = group.iter().filter(|s| *s != &from).collect();
        for pair in peers.windows(2) { rules.push(format!("{}{suffix} ? {}{suffix} : align=center-y", pair[0], pair[1])); }
    }
    rules.push(format!("{from}{suffix} ? {to}{suffix} : align=center-y; reference=second"));
    edits.extend(insert(&candidate, "layout", None, None, &rules.join("\n"))?);
    candidate = apply(&candidate, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok((crate::graph_edit::difference(source, &candidate), start))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(source: &str, text: &str) -> usize {
        source.lines().position(|s| s.trim() == text).unwrap() + 1
    }
    #[test]
    fn shortening_an_aligned_end_detaches_it_even_at_its_existing_source_slot() {
        let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A+\n second: B+\n A -> B : Foo\n B-\n A-\n}\nlayout {\n A.first+ ? B.second+ : align=center-y\n A.first- ? B.second- : align=center-y // keep this comment\n}\n";
        let start = line(source, "second: B+");
        let end = line(source, "B-");
        let (edits, selected) = resize(source, "B", start, end, true, false, end).unwrap();
        assert!(!edits.is_empty());
        assert_eq!(selected, start);
        let candidate = apply(source, &edits).unwrap();
        assert!(candidate.contains("// keep this comment"));
        assert!(candidate.contains("A.first+ ? B.second+"));
        assert!(!candidate.contains("A.first- ? B.second-"));
        let doc = laml::sequence::parse(&candidate).unwrap();
        let (_, regions) = laml::sequence::render::render_for_editor(&doc, &laml::styles::StyleSheet::default(), laml::svg::TextMode::System).unwrap();
        let bars: Vec<_> = regions.iter().filter(|r| r.kind == "execution").collect();
        assert_ne!(bars[0].y + bars[0].height, bars[1].y + bars[1].height);
    }
}
