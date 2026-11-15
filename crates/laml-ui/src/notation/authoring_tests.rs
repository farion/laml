//! Focused acceptance of the newly shared notation-editing paths.
use crate::{notation::Notation, notation_edit as edit};
use std::collections::BTreeMap;
fn patch(source: &str, edits: Vec<crate::document::Edit>) -> String {
    edit::apply(source, &edits).unwrap()
}
#[test]
fn regular_pool_action_wraps_existing_process_and_can_add_a_second_pool() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n // existing flow\n startEvent Start\n Start --> Task\n}\n";
    let recipe = edit::catalog::recipes(Notation::Bpmn)
        .into_iter()
        .find(|r| r.title == "Pool")
        .unwrap();
    let mut values = BTreeMap::from([
        ("id".into(), "Process".into()),
        ("label".into(), "Process pool".into()),
    ]);
    let first = patch(
        source,
        edit::catalog::create(source, recipe, &values, None, None).unwrap(),
    );
    let doc = laml::bpmn::parse(&first).unwrap();
    assert_eq!(doc.pools.len(), 1);
    assert_eq!(doc.nodes.len(), 2);
    assert!(
        doc.nodes
            .iter()
            .all(|n| n.pool.as_deref() == Some("Process"))
    );
    assert_eq!(doc.edges.len(), 1);
    assert!(first.contains("// existing flow"));
    values.insert("id".into(), "Other".into());
    let second = patch(
        &first,
        edit::catalog::create(&first, recipe, &values, None, None).unwrap(),
    );
    assert_eq!(laml::bpmn::parse(&second).unwrap().pools.len(), 2);
}
#[test]
fn graph_palettes_follow_compiler_types() {
    use crate::element_edit::ElementKind as K;
    let class = Notation::Class.starter(false);
    let kinds = K::available(&class);
    assert!(kinds.contains(&K::Class));
    assert!(!kinds.contains(&K::Action));
    assert!(!kinds.contains(&K::Rectangle));
    let state = Notation::State.starter(false);
    assert!(K::available(&state).contains(&K::State));
    assert!(!K::available(&state).contains(&K::Class));
}
#[test]
fn class_and_object_edits_keep_original_language_and_comments() {
    for (ty, kind) in [
        (Notation::Class, crate::element_edit::ElementKind::Class),
        (Notation::Object, crate::element_edit::ElementKind::Object),
    ] {
        let source = format!("// keep\n{}", ty.starter(false));
        let candidate = patch(
            &source,
            crate::graph_edit::add_node(&source, kind, "Added", "Label").unwrap(),
        );
        let candidate = patch(
            &candidate,
            crate::node_edit::label(&candidate, "Added", "Changed").unwrap(),
        );
        assert!(candidate.starts_with("// keep\n"));
        assert_eq!(
            crate::notation::parse_graph(&candidate)
                .unwrap()
                .nodes
                .iter()
                .find(|n| n.id == "Added")
                .unwrap()
                .label
                .as_deref(),
            Some("Changed")
        );
    }
}
#[test]
fn semantic_relationships_are_not_interchangeable_arrowheads() {
    let source = "settings {\n type: usecase\n}\nsemantics {\n actor User\n system App {\n  usecase First\n  usecase Second\n }\n User -- App.First\n}\n";
    let d = crate::notation::parse_graph(source).unwrap();
    let cases: Vec<_> = d
        .nodes
        .iter()
        .filter(|n| n.kind == laml::syntax::NodeKind::UseCase)
        .collect();
    assert!(cases.len() >= 2);
    let (_, edits) =
        crate::graph_edit::add_relation(source, &cases[0].id, &cases[1].id, "include", None, None)
            .unwrap();
    let changed = patch(source, edits);
    assert!(changed.contains("<<include>>"));
    assert!(crate::notation::parse_graph(&changed).is_ok());
}
#[test]
fn nested_frames_move_as_complete_alternative_groups() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n alt \"yes\" {\n  A ->> B : one\n }\n else \"no\" {\n  B ->> A : two\n }\n A ->> B : after\n}\n";
    let grouped = edit::record(source, 7).unwrap();
    assert!(source[grouped.block].contains("else"));
    let changed = patch(source, edit::reorder(source, 7, true).unwrap());
    assert!(changed.find("after").unwrap() < changed.find("alt ").unwrap());
    laml::sequence::parse(&changed).unwrap();
}
#[test]
fn recipes_keep_braces_and_quote_unicode_values() {
    let recipe = edit::catalog::recipes(Notation::Sequence)
        .into_iter()
        .find(|r| r.title == "Alternative")
        .unwrap();
    let mut values = BTreeMap::new();
    for p in edit::catalog::parameters(recipe) {
        values.insert(p, "Text".into());
    }
    values.insert("from".into(), "A".into());
    values.insert("to".into(), "B".into());
    values.insert("label".into(), "Grün \"quoted\"".into());
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n}\n";
    let changed = patch(
        source,
        edit::catalog::create(source, recipe, &values, None, None).unwrap(),
    );
    assert!(changed.contains("Grün \\\"quoted\\\""));
    laml::sequence::parse(&changed).unwrap();
}
#[test]
fn adding_members_creates_a_body_without_losing_comments() {
    let source = "settings {\n type: class\n}\nsemantics {\n class A // keep\n}\n";
    let changed = patch(
        source,
        edit::insert(source, "semantics", Some(5), None, "count: Integer").unwrap(),
    );
    assert!(changed.contains("class A { // keep"));
    crate::notation::parse_graph(&changed).unwrap();
}
#[test]
fn changing_sequence_identity_updates_references_not_labels() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A(label=\"A unchanged\")\n B\n A ->> B : A stays text\n}\nlayout {\n A < B\n}\n";
    let changed = patch(source, edit::rename(source, "A", 5, "Client").unwrap());
    assert!(changed.contains("Client ->> B : A stays text"));
    assert!(changed.contains("label=\"A unchanged\""));
}
#[test]
fn bpmn_membership_moves_one_inline_node_and_retains_connected_flows() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n pool P {\n  lane One {\n   A --> B --> C\n  }\n  lane Two {\n   D\n  }\n }\n}\n";
    let changed = patch(source, edit::bpmn::relocate(source, "P.B", 9).unwrap());
    let doc = laml::bpmn::parse(&changed).unwrap();
    assert_eq!(
        doc.nodes
            .iter()
            .find(|n| n.id == "P.B")
            .unwrap()
            .lane
            .as_deref(),
        Some("Two")
    );
    assert_eq!(
        doc.nodes
            .iter()
            .find(|n| n.id == "P.C")
            .unwrap()
            .lane
            .as_deref(),
        Some("One")
    );
    assert_eq!(doc.edges.len(), 2);
}
#[test]
fn specialized_formatting_keeps_compiler_interpretation() {
    for ty in [Notation::Sequence, Notation::Bpmn, Notation::Timing] {
        let source = ty.starter(false).replace("    ", " ");
        let changed = patch(&source, crate::syntax_tools::format(&source).unwrap());
        assert_eq!(
            crate::notation::inspect(&source).unwrap(),
            crate::notation::inspect(&changed).unwrap()
        );
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn specialized_regions_come_from_final_compiler_geometry() {
    for (source, expected) in [
        (
            include_str!("../../../../examples/sequence-notes.laml"),
            "note",
        ),
        (
            include_str!("../../../../examples/sequence-decomposition.laml"),
            "decomposition",
        ),
        (include_str!("../../../../examples/bpmn-basic.laml"), "flow"),
        (
            include_str!("../../../../examples/timing-basic.laml"),
            "state-change",
        ),
    ] {
        let p = crate::preview::build(source, std::path::Path::new("regions.laml")).unwrap();
        assert!(p.regions.iter().any(|r| r.kind == expected));
        for region in &p.regions {
            // The append insertion slot deliberately has no existing source
            // event; all other regions must retain their compiler source link.
            let append_slot = region.kind == "sequence-slot" && region.id == "slot:end";
            assert!(
                (region.line > 0 || append_slot) && region.x.is_finite() && region.y.is_finite(),
                "{expected}: invalid source-linked region {region:?}"
            );
        }
    }
}

#[test]
fn strict_overview_topology_is_extended_atomically() {
    let source = include_str!("../../../../examples/interaction-overview-basic.laml");
    for title in [
        "Insert interaction on flow",
        "Insert alternative on flow",
        "Insert parallel branches on flow",
    ] {
        let recipe = edit::catalog::recipes(Notation::InteractionOverview)
            .into_iter()
            .find(|r| r.title == title)
            .unwrap();
        let values = [
            ("id", "Extra"),
            ("from", "Start"),
            ("to", "SignIn"),
            ("classifier", "Step"),
            ("other_interaction", "OtherStep"),
            ("condition", "ready"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let edits = edit::catalog::create(source, recipe, &values, None, None).unwrap();
        let changed = patch(source, edits);
        laml::interaction_overview::parse(&changed)
            .unwrap_or_else(|e| panic!("{title}: {}", e.message));
    }
}
#[test]
fn timing_lane_reordering_preserves_explicit_times_and_events() {
    let source = include_str!("../../../../examples/timing-basic.laml");
    let changed = patch(
        source,
        edit::timing::move_lane(source, "Client", "Server", false).unwrap(),
    );
    let doc = laml::timing::parse(&changed).unwrap();
    assert_eq!(doc.lifelines[0].id, "Server");
    let client = doc.lifelines.iter().find(|p| p.id == "Client").unwrap();
    assert_eq!(client.changes[0].time, 2.0);
    assert_eq!(client.events[0].time, 2.0);
}
#[test]
fn named_note_properties_expose_text_without_declaration_syntax() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n hint: note \"Hello\"\n}\nlayout {\n hint over A\n}\n";
    let fields = edit::fields::of(source, 6);
    let text = fields.iter().find(|f| f.name == "Text").unwrap();
    assert_eq!(text.value, "Hello");
    let changed = patch(source, edit::fields::edit(source, text, "Changed").unwrap());
    assert_eq!(
        laml::sequence::parse(&changed)
            .unwrap()
            .events
            .iter()
            .find_map(|e| if let laml::sequence::Event::Note(n) = e {
                Some(n.text.as_str())
            } else {
                None
            }),
        Some("Changed")
    );
}
#[test]
fn profile_stereotypes_are_created_with_their_mandatory_extension() {
    let source = include_str!("../../../../examples/profile-quickstart.laml");
    for (title, parent) in [
        ("Stereotype with extension", Some(5)),
        ("Profile with stereotype", None),
    ] {
        let recipe = edit::catalog::recipes(Notation::Profile)
            .into_iter()
            .find(|r| r.title == title)
            .unwrap();
        let values = [("id", "Extra"), ("name", "Marked"), ("classifier", "Class")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let changed = patch(
            source,
            edit::catalog::create(source, recipe, &values, parent, None).unwrap(),
        );
        crate::notation::parse_graph(&changed).unwrap();
    }
}
#[test]
fn bpmn_task_and_attached_event_move_as_one_membership_transaction() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n pool P {\n  lane One {\n   Review\n   Escalate\n   Review -(timer@Deadline:\"2 days\")-> Escalate\n  }\n  lane Two {\n   Other\n  }\n }\n}\n";
    let changed = patch(
        source,
        edit::bpmn::relocate(source, "P.Review", 11).unwrap(),
    );
    let doc = laml::bpmn::parse(&changed).unwrap();
    for id in ["P.Review", "P.Deadline"] {
        assert_eq!(
            doc.nodes
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .lane
                .as_deref(),
            Some("Two")
        );
    }
    assert_eq!(doc.edges.len(), 1);
}
#[test]
fn bpmn_root_process_can_be_wrapped_in_a_pool_without_dangling_ids() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n startEvent Start\n Start --> Review --> Done\n}\nlayout {\n Start < Review\n}\nstyling {\n node[Review] {\n  fill: #ffccaa\n }\n}\n";
    let changed = patch(
        source,
        edit::bpmn::wrap_root_process(source, "P", "Process").unwrap(),
    );
    let doc = laml::bpmn::parse(&changed).unwrap();
    assert!(doc.nodes.iter().all(|n| n.pool.as_deref() == Some("P")));
    assert_eq!(doc.edges.len(), 2);
    assert!(changed.contains("node[P.Review]"));
}
#[test]
fn deletion_follows_both_alternative_replies_without_touching_visible_labels() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n request: A -> B : Call\n alt \"one\" {\n  <-- : first\n  A ->> C : keep first\n }\n else \"two\" {\n  <-- : second\n  A ->> C : keep second\n }\n A ->> C : request stays visible\n}\n";
    let plan = edit::delete(source, 8, Some("request"), false).unwrap();
    let changed = patch(source, plan.edits);
    assert!(!changed.contains("<--"));
    assert!(changed.contains("request stays visible"));
    laml::sequence::parse(&changed).unwrap();
}
#[test]
fn rename_does_not_rewrite_css_properties_that_share_an_entity_name() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n fill\n Other\n fill ->> Other : fill is visible\n}\nstyling {\n node[fill] {\n  fill: #aabbcc\n }\n}\n";
    let changed = patch(source, edit::rename(source, "fill", 5, "Renamed").unwrap());
    assert!(changed.contains("node[Renamed]"));
    assert!(changed.contains("fill: #aabbcc"));
    assert!(changed.contains("fill is visible"));
}
#[test]
fn recipe_substitution_never_interprets_the_users_label_as_placeholders() {
    let recipe = edit::catalog::recipes(Notation::Sequence)
        .into_iter()
        .find(|r| r.title == "Named message")
        .unwrap();
    let values = [
        ("id", "call"),
        ("from", "A"),
        ("to", "B"),
        ("label", "Keep {from} and {to}"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    assert!(
        edit::catalog::instantiate(recipe, &values)
            .unwrap()
            .contains("\"Keep {from} and {to}\"")
    );
}
#[test]
fn unpack_alternatives_removes_each_wrapper_and_keeps_comments() {
    let source = "settings {\r\n type: sequence\r\n}\r\nsemantics {\r\n A\r\n B\r\n alt \"one\" { // first\r\n  A ->> B : one\r\n }\r\n else \"two\" { // second\r\n  A ->> B : two\r\n }\r\n}\r\n";
    let changed = patch(source, edit::delete(source, 7, None, true).unwrap().edits);
    assert!(!changed.contains("else"));
    assert!(changed.contains("// first\r\n"));
    assert!(changed.contains("// second\r\n"));
    laml::sequence::parse(&changed).unwrap();
}
#[test]
fn execution_local_rename_updates_only_the_selected_owners_bar() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n work: A+ : \"A work\"\n work: B+ : \"B work\"\n work: B-\n work: A-\n at[execution(A,work).start] : state=\"running\"\n}\n";
    let field = edit::fields::of(source, 7)
        .into_iter()
        .find(|f| f.name == "Name / reference" && f.value == "work")
        .unwrap();
    let changed = patch(source, edit::fields::edit(source, &field, "job").unwrap());
    assert!(changed.contains("job: A+"));
    assert!(changed.contains("job: A-"));
    assert!(changed.contains("work: B+"));
    assert!(changed.contains("execution(A,job).start"));
    let deleted = patch(
        source,
        edit::delete(source, 7, Some("line:7"), false)
            .unwrap()
            .edits,
    );
    assert!(!deleted.contains("execution(A,work)"));
    assert!(!deleted.contains("work: A-"));
    assert!(deleted.contains("work: B+"));
    laml::sequence::parse(&deleted).unwrap();
}
#[test]
fn expanded_subprocess_scope_moves_preserve_internal_edges_and_style_targets() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n pool P {\n  Sub {\n   A --> B\n  }\n }\n pool Q {\n  Other\n }\n}\nstyling {\n node[P.Sub.A] {\n  fill: #ddeeff\n }\n}\n";
    let changed = patch(source, edit::bpmn::relocate(source, "P.Sub", 10).unwrap());
    let doc = laml::bpmn::parse(&changed).unwrap();
    assert!(doc.nodes.iter().any(|n| n.id == "Q.Sub.A"));
    assert_eq!(doc.edges[0].from, "Q.Sub.A");
    assert!(changed.contains("node[Q.Sub.A]"));
}

#[test]
fn filled_replies_can_be_created_without_a_colliding_fixed_name() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B : call\n}\n";
    let recipe = edit::catalog::recipes(Notation::Sequence)
        .into_iter()
        .find(|r| r.title == "Filled reply")
        .unwrap();
    let values = [("label".to_owned(), "Reply".to_owned())]
        .into_iter()
        .collect();
    let changed = patch(
        source,
        edit::catalog::create(source, recipe, &values, None, None).unwrap(),
    );
    let doc = laml::sequence::parse(&changed).unwrap();
    assert!(doc.events.iter().any(
        |e| matches!(e, laml::sequence::Event::Message(m) if m.filled_reply && m.name.is_none())
    ));
}
#[test]
fn deleting_a_root_lifeline_does_not_cross_decomposition_namespaces() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n Root\n A ->> Root : outside\n decompose Root \"Inside\" {\n  A\n  B\n  A ->> B : inside\n }\n}\n";
    let changed = patch(
        source,
        edit::delete(source, 5, Some("A"), false).unwrap().edits,
    );
    assert!(changed.contains("A ->> B : inside"));
    assert!(!changed.contains("A ->> Root : outside"));
    laml::sequence::parse(&changed).unwrap();
}
