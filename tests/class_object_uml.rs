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
            "settings {\ntype: class\n}\nsemantics {\nclass A\nnote N \"text\"\nN .. A : unvalidated\n}\n",
            "unadorned dotted line",
        ),
        (
            "settings {\ntype: class\n}\nsemantics {\nclass A\nnote N \"text\"\nN .. A <<special>>\n}\n",
            "unadorned dotted line",
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
    let navigable = image("settings {\ntype: class\n}\nsemantics {\nA\nB\nA --> B\n}\n");
    assert!(navigable.contains("M0,0 L10,4 L0,8\" fill=\"none\""));
    assert!(navigable.contains("markerWidth=\"12\" markerHeight=\"8\" refX=\"10\""));
    // `..|>` can also denote a generic UML Realization; an interface is not required.
    let generic_realization = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA ..|> B\n}\n";
    assert!(image(generic_realization).contains("marker-end=\"url(#triangle)\""));
}

#[test]
fn classifier_visibility_and_abstract_can_be_written_in_either_order() {
    for heading in ["+ abstract class A", "abstract +class A"] {
        let source = format!("settings {{\ntype: class\n}}\nsemantics {{\n{heading}\n}}\n");
        let diagram = parse(&source).unwrap();
        assert_eq!(diagram.nodes[0].visibility, Some('+'));
        assert!(diagram.nodes[0].abstract_node);
    }
    for heading in ["abstract abstract class A", "+ #class A"] {
        let source = format!("settings {{\ntype: class\n}}\nsemantics {{\n{heading}\n}}\n");
        assert!(parse(&source).unwrap_err().message.contains("duplicate"));
    }
}

#[test]
fn abstract_modifier_applies_to_classifiers_not_instances_notes_or_packages() {
    for source in [
        "settings {\ntype: object\n}\nsemantics {\nabstract object alice(of=Person)\n}\n",
        "settings {\ntype: class\n}\nsemantics {\nabstract note N \"text\"\n}\n",
        "settings {\ntype: class\n}\nsemantics {\nabstract package P {\nclass A\n}\n}\n",
    ] {
        assert!(
            parse(source)
                .unwrap_err()
                .message
                .contains("only on a UML classifier")
        );
    }
    parse("settings {\ntype: class\n}\nsemantics {\nabstract class A\n}\n").unwrap();
}

#[test]
fn generalization_requires_acyclic_hierarchy() {
    let base = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\nclass C\nA --|> B\nB --|> C\n}\n";
    parse(base).unwrap();
    let cycle = base.replace("B --|> C", "B --|> C\nC --|> A");
    assert!(
        parse(&cycle)
            .unwrap_err()
            .message
            .contains("hierarchy contains a cycle")
    );
    let inverse = base.replace("B --|> C", "B --|> C\nA <|-- C");
    assert!(
        parse(&inverse)
            .unwrap_err()
            .message
            .contains("hierarchy contains a cycle")
    );
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
