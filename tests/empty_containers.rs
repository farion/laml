use laml::{geometry, parser, styles};
use std::path::Path;

fn boxes(kind: &str, body: &str, styling: &str) -> Vec<geometry::BoxNode> {
    let setting = if kind == "generic" {
        String::new()
    } else {
        format!("type: {kind}")
    };
    let source = format!(
        "settings {{\n {setting}\n}}\nsemantics {{\n{body}\n}}\nstyling {{\n{styling}\n}}\n"
    );
    let diagram = parser::parse(&source).unwrap();
    let sheet = styles::load(&diagram, Path::new("empty.laml")).unwrap();
    geometry::place(&diagram, &sheet).unwrap()
}
#[test]
fn empty_containers_have_space_below_the_heading() {
    for (kind, body) in [
        ("generic", "package P {\n}"),
        ("package", "package P"),
        ("component", "component P {\n}"),
        ("state", "state P {\n}"),
        ("deployment", "node P {\n}"),
        ("composite", "structure P {\n}"),
        ("usecase", "system P {\n}"),
        ("profile", "profile P {\n}"),
    ] {
        let nodes = boxes(kind, body, "");
        let n = &nodes[0];
        assert!(n.width >= 200.0, "{kind}: width {}", n.width);
        let heading = n.padding * 2.0 + n.font_size * 1.25 * n.lines.len() as f64;
        assert!(n.height >= heading + 100.0, "{kind}: height {}", n.height);
    }
}
#[test]
fn explicit_container_sizes_override_defaults() {
    let nodes = boxes(
        "generic",
        "package P {\n}",
        "node[P] {\n width: 150\n height: 90\n}",
    );
    assert_eq!((nodes[0].width, nodes[0].height), (150.0, 90.0));
}
