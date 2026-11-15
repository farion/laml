use laml::{sequence, styles::StyleSheet, svg::TextMode};

fn source(body: &str, rules: &str) -> String {
    format!("settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n C\n{body}\n}}\nlayout {{\n{rules}\n}}\n")
}
fn ends(text: &str) -> Vec<laml::authoring::Region> {
    let doc = sequence::parse(text).unwrap();
    sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System).unwrap().1
        .into_iter().filter(|r| r.kind == "destroyed-end").collect()
}
#[test]
fn standalone_destructions_share_source_linked_geometry() {
    let text = source(" A+\n A!\n B!", " A! ? B! : align=center-y");
    let doc = sequence::parse(&text).unwrap();
    let (_, regions) = sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System).unwrap();
    let a = regions.iter().find(|r| r.kind == "destroyed-end" && r.id == "A").unwrap();
    let b = regions.iter().find(|r| r.kind == "destroyed-end" && r.id == "B").unwrap();
    assert_eq!(a.y, b.y);
    assert_eq!(text.lines().nth(a.line - 1).unwrap().trim(), "A!");
    assert_eq!(text.lines().nth(b.line - 1).unwrap().trim(), "B!");
    let activation = regions.iter().find(|r| r.kind == "execution").unwrap();
    assert!((activation.y + activation.height - a.y - a.height / 2.0).abs() < 0.01);
}
#[test]
fn alignment_is_transitive_and_rule_order_independent() {
    let a = source(" A!\n B!\n C!", " A! ? B! : align=center-y\n B! ? C! : align=center-y");
    let b = source(" A!\n B!\n C!", " C! ? B! : align=center-y\n B! ? A! : align=center-y");
    let (a, b) = (ends(&a), ends(&b));
    assert!(a.iter().all(|r| r.y == a[0].y));
    assert_eq!(a.iter().map(|r| r.y).collect::<Vec<_>>(), b.iter().map(|r| r.y).collect::<Vec<_>>());
}
#[test]
fn no_rule_preserves_separate_rows() {
    let rows = ends(&source(" A!\n B!", ""));
    assert_ne!(rows[0].y, rows[1].y);
}
#[test]
fn invalid_or_message_bound_endpoints_fail() {
    for (body, rule) in [
        (" A!\n B!", " A! ? C! : align=center-y"),
        (" A!\n B!", " A! ? A! : align=center-y"),
        (" A!\n B!", " A! ? B! : align=center-x"),
        (" A -> B! : delete\n A!", " A! ? B! : align=center-y"),
        (" alt \"branch\" {\n A!\n }\n B!", " A! ? B! : align=center-y"),
    ] {
        assert!(sequence::parse(&source(body, rule)).is_err(), "{body}\n{rule}");
    }
}
