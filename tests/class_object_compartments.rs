use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};

const SOURCE: &str = "settings {\ntype: class\n}\nsemantics {\nclass A {\n@id id: UUID\n--\n+run(): void\n}\nclass B {\nname: String\n--\n+save(): void\n}\nenum State {\nREADY\nDONE\n}\n}\nstyling {\nnode[A] compartment[attributes] {\ndisplay: none\n}\nnode[State] compartment[literals] {\ndisplay: none\n}\n}\n";

#[test]
fn hidden_compartments_remove_members_and_separators_without_erasing_other_nodes() {
    let diagram = parse(SOURCE).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert!(
        boxes[0]
            .body
            .iter()
            .all(|part| !matches!(part, laml::syntax::NodePart::Separator))
    );
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(!svg.contains("id: UUID"));
    assert!(svg.contains("+run(): void"));
    assert!(svg.contains("name: String"));
    assert!(!svg.contains("READY"));
    assert!(!svg.contains("DONE"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let methods = SOURCE.replace("compartment[attributes]", "compartment[operations]");
    let diagram = parse(&methods).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let svg = render(&place(&diagram, &sheet).unwrap(), &[]).unwrap();
    assert!(svg.contains("id: UUID"));
    assert!(!svg.contains("+run(): void"));
}

#[test]
fn visible_notes_cannot_target_a_hidden_compartment_member() {
    let source = SOURCE.replace("enum State", "note N \"reason\"\nN .. id\nenum State");
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("hidden")
    );
    let unknown = SOURCE.replace("compartment[attributes]", "compartment[unknown]");
    assert!(parse(&unknown).unwrap_err().message.contains("invalid"));
}
