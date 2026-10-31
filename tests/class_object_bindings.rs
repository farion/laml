use laml::{export, geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

const SOURCE: &str = "settings {\ntype: class\n}\nsemantics {\nclass Repository<T>\nclass StringRepository\n@bind StringRepository ..> Repository <<bind(T->String)>>\n}\n";

#[test]
fn template_binding_validates_declared_formals_and_exports_visible_substitution() {
    let diagram = parse(SOURCE).unwrap();
    assert_eq!(
        diagram.relations[0].binding.as_ref().unwrap(),
        &[("T".into(), "String".into())]
    );
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("«bind» T-&gt;String"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn binding_rejects_unknown_or_duplicate_formals_and_non_template_targets() {
    for (bad, expected) in [
        (
            SOURCE.replace("T->String", "Unknown->String"),
            "unknown template parameter",
        ),
        (
            SOURCE.replace("T->String", "T->String, T->Other"),
            "duplicate template binding",
        ),
        (
            SOURCE.replace("..> Repository", "--> Repository"),
            "dashed open-arrow",
        ),
        (
            SOURCE.replace("Repository<T>", "Repository"),
            "local template class",
        ),
        (
            SOURCE.replace("<<bind(T->String)>>", "<<bind>>"),
            "binding validation",
        ),
        (
            SOURCE.replace("<<bind(T->String)>>", "<<bind()>>"),
            "at least one",
        ),
    ] {
        let error = parse(&bad).unwrap_err();
        assert!(error.message.contains(expected), "{error:?}");
    }
    let generic = SOURCE.replace("settings {\ntype: class\n}\n", "");
    assert!(
        parse(&generic)
            .unwrap_err()
            .message
            .contains("requires type: class")
    );
}
