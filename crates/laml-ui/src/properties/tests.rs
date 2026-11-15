use super::*;
#[test]
fn snapshots_use_imports_contextual_specificity_and_real_rendered_geometry() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("theme.lamlstyle"), "styling {\n node.service {\n fill: #123456\n }\n node[P] node[A] {\n padding: 31\n font-size: 21\n }\n port[Input] {\n fill: #654321\n }\n}\n").unwrap();
    let source = "semantics {\n package P {\n A(style-classes=service) {\n @Input port In\n }\n }\n}\nstyling {\n import \"theme.lamlstyle\"\n}\n";
    let preview = crate::preview::build(source, &dir.path().join("diagram.laml")).unwrap();
    let node = preview.nodes.iter().find(|n| n.id == "P.A").unwrap();
    assert_eq!(
        preview.node_properties["P.A"]["padding"].value.as_deref(),
        Some("31")
    );
    assert_eq!(
        preview.node_properties["P.A"]["font-size"].value.as_deref(),
        Some("21")
    );
    assert_eq!(
        preview.node_properties["P.A"]["fill"].value.as_deref(),
        Some("#123456")
    );
    assert_eq!(
        preview.node_properties["P.A"]["width"].value.as_deref(),
        Some(node.width.to_string().as_str())
    );
    assert_eq!(
        preview.port_properties["Input"]["fill"].value.as_deref(),
        Some("#654321")
    );
    assert!(!preview.node_properties["P"]["shape"].editable);
    assert!(!preview.node_properties["P"]["icon"].editable);
}
#[test]
fn hidden_objects_have_inspectable_visibility_without_fake_dimensions() {
    let preview = crate::preview::build(
        "semantics {\n A\n}\nstyling {\n node[A] {\n display: none\n }\n}\n",
        std::path::Path::new("test.laml"),
    )
    .unwrap();
    assert!(preview.nodes.is_empty());
    assert_eq!(
        preview.node_properties["A"]["display"].value.as_deref(),
        Some("none")
    );
    assert_eq!(preview.node_properties["A"]["width"].value, None);
    let mut selection = crate::selection::Selection::default();
    selection.click(Some("A"), false);
    selection.retain_diagram(&preview.diagram);
    assert_eq!(selection.ids, ["A"]);
}
#[test]
fn canonical_values_come_from_compiler_property_validation() {
    assert_eq!(
        string(&laml::styles::property_value("fill", "rgb(18, 52, 86)").unwrap()),
        "#123456"
    );
    assert!(laml::styles::property_value("font-size", "0").is_err());
    assert!(laml::styles::property_value("width", "NaN").is_err());
    assert!(laml::styles::property_value("line-style", "dashed").is_err());
    assert_eq!(source_value("icon", "folder"), "\"folder\"");
}
