use laml::{geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

fn image(source: &str) -> String {
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    render(&boxes, &routes).unwrap()
}

#[test]
fn uml_rejects_relationship_glyphs_that_misstate_the_element_kinds() {
    for (source, reason) in [
        (
            "settings {\ntype: class\n}\nsemantics {\nA\nA --|> A\n}\n",
            "itself",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nclass A\ninterface I\nA --|> I\n}\n",
            "different UML classifier kinds",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nA\nB\nA \"1\" --|> B\n}\n",
            "association-end",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nA\nB\nA /r ..> B\n}\n",
            "association-end",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nA\nnote N \"text\"\nN -- A\n}\n",
            "note attaches",
        ),
        (
            "settings {\ntype: object\n}\nsemantics {\na\nb\na .. b\n}\n",
            "dotted UML annotation",
        ),
    ] {
        let err = parse(source).unwrap_err();
        assert!(err.message.contains(reason), "{err:?}");
    }
    let realization =
        "settings {\ntype: class\n}\nsemantics {\nclass C\ninterface API\nC ..|> API\n}\n";
    assert!(image(realization).contains("marker-end=\"url(#triangle)\""));
    assert!(
        image("settings {\ntype: class\n}\nsemantics {\nA\nB\nA --> B\n}\n")
            .contains("M0,0 L10,4 L0,8\" fill=\"none\"")
    );
    // `..|>` can also denote a generic UML Realization; an interface is not required.
    let generic_realization = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA ..|> B\n}\n";
    assert!(image(generic_realization).contains("marker-end=\"url(#triangle)\""));
}

#[test]
fn uml_rejects_unvalidated_package_relations_and_nonclass_wholes() {
    for (source, reason) in [
        (
            "settings {\ntype: class\n}\nsemantics {\npackage P {\nclass A\n}\nP --> P.A\n}\n",
            "package relationships",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\ninterface API\nclass A\nAPI *-- A\n}\n",
            "class at the whole end",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nclass A\ninterface API\nA --* API\n}\n",
            "class at the whole end",
        ),
    ] {
        let err = parse(source).unwrap_err();
        assert!(err.message.contains(reason), "{err:?}");
    }
}
