use laml::parser::{parse, parse_stylesheet};
use laml::styles::{Element, Value, load};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let name = format!(
            "laml-style-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir().join(name);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn diagram(&self) -> PathBuf {
        self.path("architecture.laml")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn parse_diagram(styling: &str) -> laml::syntax::Diagram {
    parse(&format!(
        "semantics {{\nA(style-classes=backend,critical)\n}}\nstyling {{\n{styling}\n}}\n"
    ))
    .unwrap()
}

#[test]
fn universal_defaults_lose_to_type_selectors_even_when_declared_later() {
    let f = Fixture::new();
    let diagram = parse_diagram("node {\ncolor: rgb(1, 2, 3)\n}\n* {\ncolor: rgb(4, 5, 6)\n}");
    let sheet = load(&diagram, &f.diagram()).unwrap();
    let node = Element::Node {
        id: "A",
        classes: &diagram.nodes[0].classes,
    };
    assert_eq!(
        sheet.get(&node, "color").unwrap().value,
        Value::Color("#010203".into())
    );
    let relation = Element::Relation {
        name: None,
        classes: &[],
    };
    assert_eq!(
        sheet.get(&relation, "color").unwrap().value,
        Value::Color("#040506".into())
    );
}

#[test]
fn imported_styles_are_applied_in_source_order_with_id_priority() {
    let f = Fixture::new();
    f.write(
        "shared.lamlstyle",
        "styling {\n.backend {\nfill: rgb(1, 2, 3)\n}\n.critical {\nfill: rgb(4, 5, 6)\n}\n}\n",
    );
    let diagram = parse_diagram(
        "import \"shared.lamlstyle\"\nnode.backend {\nfill: rgb(7, 8, 9)\n}\nnode[A] {\nfill: rgb(10, 11, 12)\n}",
    );
    let sheet = load(&diagram, &f.diagram()).unwrap();
    let node = Element::Node {
        id: "A",
        classes: &diagram.nodes[0].classes,
    };
    assert_eq!(
        sheet.get(&node, "fill").unwrap().value,
        Value::Color("#0a0b0c".into())
    );
    assert_eq!(sheet.rules.len(), 4);
}

#[test]
fn class_order_on_object_does_not_determine_cascade() {
    let f = Fixture::new();
    let diagram =
        parse_diagram(".critical {\nfill: rgb(1, 2, 3)\n}\n.backend {\nfill: rgb(4, 5, 6)\n}");
    let sheet = load(&diagram, &f.diagram()).unwrap();
    let node = Element::Node {
        id: "A",
        classes: &diagram.nodes[0].classes,
    };
    assert_eq!(
        sheet.get(&node, "fill").unwrap().value,
        Value::Color("#040506".into())
    );
}

#[test]
fn known_but_inapplicable_property_is_ignored_for_relation() {
    let f = Fixture::new();
    let diagram = parse_diagram(".backend {\nfill: rgb(1, 2, 3)\ncolor: rgb(4, 5, 6)\n}");
    let sheet = load(&diagram, &f.diagram()).unwrap();
    let names = vec!["backend".to_owned()];
    let relation = Element::Relation {
        name: None,
        classes: &names,
    };
    assert!(sheet.get(&relation, "fill").is_none());
    assert_eq!(
        sheet.get(&relation, "color").unwrap().value,
        Value::Color("#040506".into())
    );
}

#[test]
fn nested_imports_are_relative_to_each_importing_file() {
    let f = Fixture::new();
    f.write(
        "themes/base.lamlstyle",
        "styling {\nimport \"inner/colors.lamlstyle\"\n}\n",
    );
    f.write(
        "themes/inner/colors.lamlstyle",
        "styling {\nnode {\npadding: 18\n}\n}\n",
    );
    let diagram = parse_diagram("import \"themes/base.lamlstyle\"");
    let sheet = load(&diagram, &f.diagram()).unwrap();
    assert_eq!(
        sheet
            .get(
                &Element::Node {
                    id: "A",
                    classes: &[]
                },
                "padding"
            )
            .unwrap()
            .value,
        Value::Number(18.0)
    );
}

#[test]
fn import_cycles_and_unknown_properties_fail_with_location() {
    let f = Fixture::new();
    f.write("a.lamlstyle", "styling {\nimport \"b.lamlstyle\"\n}\n");
    f.write("b.lamlstyle", "styling {\nimport \"a.lamlstyle\"\n}\n");
    let error = load(&parse_diagram("import \"a.lamlstyle\""), &f.diagram()).unwrap_err();
    assert!(error.message.contains("cycle"));
    assert_eq!(error.line, 2);
    assert!(error.path.ends_with(Path::new("b.lamlstyle")));

    let error = load(&parse_diagram("node {\nfont-szie: 14\n}"), &f.diagram()).unwrap_err();
    assert!(error.message.contains("unknown style property"));
    assert_eq!(error.line, 6);
}

#[test]
fn stylesheet_cannot_define_semantics() {
    assert!(parse_stylesheet("semantics {\nA\n}\n").is_err());
}
