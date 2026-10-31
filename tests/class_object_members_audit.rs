use laml::parser::parse;

#[test]
fn uml_members_reject_modifiers_and_kinds_that_misstate_notation() {
    for (setting, body, expected) in [
        (
            "type: object",
            "object O {\ntext \"anything\"\n}",
            "only name = value slots",
        ),
        (
            "type: object",
            "object O {\nstatic id = 1\n}",
            "only name = value slots",
        ),
        (
            "type: class",
            "enum E {\nstatic READY\n}",
            "enumeration literals cannot",
        ),
        (
            "type: class",
            "enum E {\nmethod READY\n}",
            "enumeration literals cannot",
        ),
        (
            "type: class",
            "class C {\nabstract id: UUID\n}",
            "attributes cannot be abstract",
        ),
        (
            "type: class",
            "class C {\nfield +go()\n}",
            "attribute cannot use operation parentheses",
        ),
        (
            "type: class",
            "class C {\nmethod +go: Void\n}",
            "operation requires parentheses",
        ),
    ] {
        let diagram = format!("settings {{\n{setting}\n}}\nsemantics {{\n{body}\n}}\n");
        let err = parse(&diagram).unwrap_err();
        assert!(err.message.contains(expected), "{err:?}");
    }
}
