use laml::{geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

#[test]
fn repeated_member_names_resolve_in_nearest_package_scope() {
    let source = "settings {\ntype: class\n}\nsemantics {\npackage P {\nclass A {\nid: UUID\n}\nnote Why {\nP reason\n}\nWhy .. A::id\n}\npackage Q {\nclass A {\nid: UUID\n}\nnote Why \"Q reason\"\nWhy .. A::id\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.relations[0].from, "P.Why");
    assert_eq!(diagram.relations[0].to, "P.A::id");
    assert_eq!(diagram.relations[1].from, "Q.Why");
    assert_eq!(diagram.relations[1].to, "Q.A::id");
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routed = route(&diagram, &boxes, &styles).unwrap();
    assert!(render(&boxes, &routed).unwrap().contains("P reason"));
}

#[test]
fn explicit_member_ids_are_stable_and_unique_across_scopes_and_nodes() {
    let source = "settings {\ntype: class\n}\nsemantics {\npackage P {\nclass A {\n@globalMember id: UUID\n}\n}\nnote N \"why\"\nN .. globalMember\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.relations[0].to, "globalMember");
    let moved = source.replace("package P {", "package Q {");
    let diagram = parse(&moved).unwrap();
    assert_eq!(diagram.relations[0].to, "globalMember");
    let collision = source.replace("note N \"why\"", "@globalMember note N \"why\"");
    assert!(parse(&collision).unwrap_err().message.contains("duplicate"));
}
