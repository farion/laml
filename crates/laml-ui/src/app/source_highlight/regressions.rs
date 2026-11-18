use super::{
    Role,
    tests::{inside, member, role},
};

#[test]
fn qualified_selector_ids_are_not_style_classes() {
    let s = inside("styling", "node[Shop.API].critical {\nfill: \"#fff\"\n}");
    assert_eq!(role(&s, "Shop"), Role::Id);
    assert_eq!(role(&s, "API"), Role::Id);
    assert_eq!(role(&s, "critical"), Role::StyleClass);
}

#[test]
fn label_semicolons_and_arrow_symbols_remain_text() {
    for s in [
        inside("semantics", "A -> B : first; second -> third"),
        inside("semantics", "work: B+ : first; second -> third"),
    ] {
        for word in ["first", "second", "third"] {
            assert_eq!(role(&s, word), Role::Text);
        }
    }
}

#[test]
fn member_names_matching_declaration_words_are_not_keywords() {
    for text in [
        "+port: String",
        "input: String",
        "note: String",
        "output: String",
    ] {
        let s = member(text);
        assert_eq!(role(&s, "String"), Role::Type);
    }
    let s = member("+send(input: Integer, output: String): Result");
    assert_eq!(role(&s, "input"), Role::Member);
    assert_eq!(role(&s, "output"), Role::Member);
}

#[test]
fn nested_generic_parameter_types_do_not_swallow_the_next_parameter() {
    let s = member("+send(items: Map<Key, List<Value>>, timeout: Integer): Result");
    assert_eq!(role(&s, "items"), Role::Member);
    assert_eq!(role(&s, "timeout"), Role::Member);
    for name in ["Map", "Key", "List", "Value", "Integer", "Result"] {
        assert_eq!(role(&s, name), Role::Type);
    }
}

#[test]
fn named_sequence_fragments_keep_guards_text_and_nested_members_semantic() {
    let s = inside(
        "semantics",
        "choice: alt approved {\nA -> B : proceed\n}\nelse rejected {\nA -> C : reject\n}",
    );
    assert_eq!(role(&s, "choice"), Role::Id);
    assert_eq!(role(&s, "alt"), Role::Keyword);
    assert_eq!(role(&s, "approved"), Role::Text);
    assert_eq!(role(&s, "rejected"), Role::Text);
    for id in ["A", "B", "C"] {
        assert_eq!(role(&s, id), Role::Id);
    }
}

#[test]
fn nested_fragment_operators_and_loop_attributes_are_separate() {
    let s = inside(
        "semantics",
        "strict loop(min=1; max=3) accepted {\nA -> B : next\n}",
    );
    for keyword in ["strict", "loop"] {
        assert_eq!(role(&s, keyword), Role::Keyword);
    }
    for key in ["min", "max"] {
        assert_eq!(role(&s, key), Role::Property);
    }
    assert_eq!(role(&s, "accepted"), Role::Text);
    assert_eq!(role(&s, "1"), Role::Number);
}

#[test]
fn bpmn_untyped_subprocess_contents_are_not_uml_fields_even_with_settings_last() {
    let s = "semantics {\nOpen {\nStart\nstartEvent Begin\nStart --> Begin\n}\n}\nsettings {\ntype: bpmn\n}\n";
    assert_eq!(role(s, "Open"), Role::Id);
    assert_eq!(role(s, "Start"), Role::Id);
    assert_eq!(role(s, "startEvent"), Role::Keyword);
    assert_eq!(role(s, "Begin"), Role::Id);
}

#[test]
fn timing_and_annotation_properties_keep_numbers_and_references() {
    let s = "settings {\ntype: timing\n}\nsemantics {\nstate User at 2 : WaitCard\n}\nlayout {\nscale: 105\ntick 2\n}\n";
    assert_eq!(role(s, "state"), Role::Keyword);
    assert_eq!(role(s, "at"), Role::Keyword);
    assert_eq!(role(s, "WaitCard"), Role::Text);
    assert_eq!(role(s, "scale"), Role::Property);
    assert_eq!(role(s, "105"), Role::Number);
    assert_eq!(role(s, "tick"), Role::Keyword);
    let s = inside(
        "semantics",
        "between[first.send,second.receive] : duration-constraint=\"< 2s\"",
    );
    assert_eq!(role(&s, "between"), Role::Keyword);
    assert_eq!(role(&s, "duration-constraint"), Role::Property);
    assert_eq!(role(&s, "first"), Role::Id);
}
