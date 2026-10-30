use laml::sequence::{Event, parse};

#[test]
fn continuation_text_and_mandatory_span_are_in_separate_sections() {
    let head = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nnext: continuation \"Continue here\"\n}\n";
    let source = format!("{head}layout {{\nnext spans A,B : gap=12\n}}\n");
    let diagram = parse(&source).unwrap();
    assert!(
        matches!(&diagram.events[0], Event::Continuation(c) if c.id == "next" && c.text == "Continue here")
    );
    assert_eq!(diagram.continuation_layout[0].participants, ["A", "B"]);
    assert!(
        parse(head)
            .unwrap_err()
            .message
            .contains("exactly one layout")
    );
    for layout in [
        "next spans A,A",
        "next spans A,C",
        "missing spans A,B",
        "next spans A,B : gap=-1",
        "next spans A,B\nnext spans A",
    ] {
        let bad = format!("{head}layout {{\n{layout}\n}}\n");
        assert!(parse(&bad).is_err(), "accepted {layout}");
    }
}

#[test]
fn execution_ids_are_local_to_participants_and_close_in_stack_order() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nwork: API+ : \"Process\"\ncheck: API+ : \"Audit\"\ncheck: API-\nwork: API-\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        matches!(&diagram.events[0], Event::Activate { participant, id: Some(id), label: Some(label), .. }
        if participant == "API" && id == "work" && label == "Process")
    );
    assert!(matches!(&diagram.events[1], Event::Activate { id: Some(id), .. } if id == "check"));
    let bad = source.replace("check: API-", "work: API-");
    assert!(
        parse(&bad)
            .unwrap_err()
            .message
            .contains("not the open top")
    );
    let duplicate = source.replace("check: API+", "work: API+");
    assert!(
        parse(&duplicate)
            .unwrap_err()
            .message
            .contains("duplicate execution")
    );
    let unnamed = source
        .replace("check: API-", "API-")
        .replace("work: API-", "API-");
    assert!(parse(&unnamed).is_ok());
    let other = source.replace(
        "API\nwork:",
        "API\nOther\nwork: Other+\nwork: Other-\nwork:",
    );
    assert!(parse(&other).is_ok());
}

#[test]
fn coregion_groups_only_occurrences_on_the_named_lifeline() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nAPI\nfirst: A -> API : request\nsecond: B -> API : event\ncoregion API {\nfirst.receive\nsecond.receive\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.coregions[0].participant, "API");
    assert_eq!(diagram.coregions[0].ends.len(), 2);
    let reordered = format!("{source}layout {{\nsecond.receive above first.receive\n}}\n");
    let parsed = parse(&reordered).unwrap();
    assert_eq!(parsed.vertical_layout.len(), 1);
    let cyclic = reordered.replace(
        "second.receive above first.receive",
        "second.receive above first.receive\nfirst.receive above second.receive",
    );
    assert!(
        parse(&cyclic)
            .unwrap_err()
            .message
            .contains("contradictory vertical")
    );
    let cross_lifeline = reordered.replace(
        "second.receive above first.receive",
        "first.send above second.receive",
    );
    assert!(
        parse(&cross_lifeline)
            .unwrap_err()
            .message
            .contains("same coregion")
    );
    for (old, replacement) in [
        ("first.receive", "first.send"),
        ("second.receive", "missing.receive"),
        ("second.receive", "first.receive"),
        ("coregion API {", "coregion Unknown {"),
        ("second.receive\n}", "}"),
    ] {
        let bad = source.replace(old, replacement);
        assert!(
            parse(&bad).is_err(),
            "accepted invalid coregion: {old} -> {replacement}"
        );
    }
    let unclosed = source.replace("second.receive\n}\n}", "second.receive\n}");
    assert!(parse(&unclosed).is_err());
}

#[test]
fn notes_have_separate_semantics_and_mandatory_referenced_layout() {
    let prefix = "settings {\ntype: sequence\n}\nsemantics {\nA\nrequest: A -> A : call\nretryHint: note \"Retry on timeout\"\n}\n";
    let source = format!("{prefix}layout {{\nretryHint over A : gap=16\n}}\n");
    let diagram = parse(&source).unwrap();
    assert!(
        matches!(&diagram.events[1], Event::Note(note) if note.id == "retryHint" && note.text == "Retry on timeout")
    );
    assert_eq!(diagram.note_layout[0].target, "A");
    let message_anchor = source.replace("retryHint over A", "retryHint over request.receive");
    assert_eq!(
        parse(&message_anchor).unwrap().note_layout[0].target,
        "request.receive"
    );
    let send_anchor = source.replace("retryHint over A", "retryHint over request.send");
    assert!(parse(&send_anchor).is_ok());
    for target in ["missing.receive", "request.sent", "request.receive.extra"] {
        let bad = source.replace("retryHint over A", &format!("retryHint over {target}"));
        assert!(parse(&bad).is_err());
    }
    assert!(
        parse(prefix)
            .unwrap_err()
            .message
            .contains("needs layout placement")
    );
    for layout in [
        "retryHint over Unknown",
        "unknown over A",
        "retryHint over A\nretryHint over A",
        "retryHint over A : gap=-1",
    ] {
        let bad = format!("{prefix}layout {{\n{layout}\n}}\n");
        assert!(parse(&bad).is_err(), "accepted invalid layout: {layout}");
    }
    let duplicated = source.replace(
        "retryHint: note \"Retry on timeout\"",
        "retryHint: note \"One\"\nretryHint: note \"Two\"",
    );
    assert!(
        parse(&duplicated)
            .unwrap_err()
            .message
            .contains("duplicate diagram identifier")
    );
    let early = "settings {\ntype: sequence\n}\nsemantics {\nA\ncomment: note \"Earlier\"\nrequest: A -> A : later\n}\nlayout {\ncomment over request.receive\n}\n";
    assert!(
        parse(early)
            .unwrap_err()
            .message
            .contains("earlier named message")
    );
    let ambiguous = source
        .replace(
            "retryHint: note",
            "request: A -> A : again\nretryHint: note",
        )
        .replace("retryHint over A", "retryHint over request.receive");
    let message = parse(&ambiguous).unwrap_err().message;
    assert!(message.contains("duplicate message"), "{message}");
    for semantics in [
        "A\nA!\ncomment: note \"Too late\"",
        "comment: note \"Before creation\"\nA+",
    ] {
        let source = format!(
            "settings {{\ntype: sequence\n}}\nsemantics {{\n{semantics}\n}}\nlayout {{\ncomment over A\n}}\n"
        );
        assert!(
            parse(&source)
                .unwrap_err()
                .message
                .contains("inactive participant")
        );
    }
}

#[test]
fn continuation_spans_two_lifelines_and_takes_its_own_style() {
    use laml::styles::load;
    use laml::svg::TextMode;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nnext: continuation \"Ready & waiting\"\n}\nlayout {\nnext spans A,B : gap=12\n}\nstyling {\ncontinuation[next] {\nfill: \"#aabbcc\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.lines().any(
        |line| line.contains("data-continuation=\"next\"") && line.contains("fill=\"#aabbcc\"")
    ));
    assert!(svg.contains("Ready &amp; waiting"));
}

#[test]
fn inline_decomposition_is_scoped_and_uses_root_layout() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\ndecompose API \"Internals\" {\nCache\nDB\nCache -> DB : query\n}\nAPI -> Client : done\n}\nlayout {\nClient < API\nAPI.Cache < API.DB : gap=44\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.decompositions.len(), 1);
    assert_eq!(diagram.decompositions[0].diagram.participants.len(), 2);
    let strict = source.replace(
        "decompose API \"Internals\" {",
        "decompose API \"Internals\" strict {",
    );
    assert!(parse(&strict).unwrap().decompositions[0].strict);
    assert_eq!(diagram.decompositions[0].diagram.layout[0].gap, 44.0);
    assert!(matches!(
        &diagram.events[0],
        Event::Decomposition { index: 0, .. }
    ));
    let bad = source.replace("API.Cache < API.DB", "API.Cache < Client");
    assert!(
        parse(&bad)
            .unwrap_err()
            .message
            .contains("must start with 'API.'")
    );
    let bad = source.replace("API.Cache < API.DB", "API.Cache < API.Unknown");
    assert!(
        parse(&bad)
            .unwrap_err()
            .message
            .contains("unknown participant")
    );
    let bad = source
        .replace("decompose API", "decompose Missing")
        .replace("API.Cache < API.DB : gap=44\n", "");
    assert!(
        parse(&bad)
            .unwrap_err()
            .message
            .contains("unknown decomposed participant")
    );
    let bad = source.replace("Cache\nDB", "Cache\nCache");
    assert!(parse(&bad).is_err());
    let reused = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nWorker\ndecompose API \"One\" {\nCache\n}\ndecompose Worker \"Two\" {\nCache\n}\n}\n";
    assert_eq!(parse(reused).unwrap().decompositions.len(), 2);
    let bad = source.replace(
        "Cache\nDB",
        "Cache\nstyling {\nnode {\nfill: \"#eeeeee\"\n}\n}\nDB",
    );
    assert!(parse(&bad).is_err());
}

#[test]
fn inline_decomposition_renders_child_lifelines_and_main_messages() {
    use laml::svg::TextMode;
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/sequence-decomposition.laml"
    ))
    .unwrap();
    let diagram = parse(&source).unwrap();
    let svg = laml::sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-decomposition=\"API\""));
    assert!(svg.contains("ref Gateway internals"));
    assert!(svg.contains(">Cache</text>") && svg.contains(">DB</text>"));
    assert!(svg.contains("query") && svg.contains("response"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF"));
}

#[test]
fn css_descendant_selectors_scope_reused_child_ids() {
    use laml::{styles::load, svg::TextMode, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI(style-classes=critical)\nWorker\ndecompose API \"One\" {\nCache\n}\ndecompose Worker \"Two\" {\nCache\n}\n}\nstyling {\nnode[Cache] {\nfill: \"#dddddd\"\n}\nnode[API] node[Cache] {\nfill: \"#aa0000\"\n}\nnode[Worker] node[Cache] {\nfill: \"#0000aa\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let children: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-participant=\"Cache\""))
        .collect();
    assert_eq!(children.len(), 2);
    assert!(children[0].contains("fill=\"#aa0000\""));
    assert!(children[1].contains("fill=\"#0000aa\""));
    let class_scoped = source.replace("node[API] node[Cache]", "node.critical node[Cache]");
    let diagram = parse(&class_scoped).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.lines().any(
        |line| line.contains("data-participant=\"Cache\"") && line.contains("fill=\"#aa0000\"")
    ));
    assert!(parse(&source.replace("node[API] node[Cache]", "node[API] node[bad-name]")).is_err());
}

#[test]
fn ignore_consider_frames_display_message_names_in_braces() {
    use laml::svg::TextMode;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nignore \"cancel, update\" {\nA ->> B : one\n}\nconsider \"create\" {\nB ->> A : two\n}\n}\n";
    let svg = laml::sequence::render::render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"ignore {cancel, update}\""));
    assert!(svg.contains("data-frame-descriptor=\"consider {create}\""));
    assert!(parse(&source.replace("ignore \"cancel, update\"", "ignore \"\"")).is_err());
}

#[test]
fn named_fragment_can_override_operator_style_without_changing_other_frames() {
    use laml::{styles::load, svg::TextMode, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\ncheckout: alt \"yes\" {\nA ->> B : first\n}\nalt \"maybe\" {\nB ->> A : second\n}\n}\nstyling {\nfragment[alt] {\nborder-color: \"#444444\"\n}\nfragment[checkout] {\nborder-color: \"#aa0000\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        diagram
            .events
            .iter()
            .any(|e| matches!(e, Event::FrameStart { id: Some(id), .. } if id == "checkout"))
    );
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let named = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"checkout\""))
        .unwrap();
    let unnamed = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"\""))
        .unwrap();
    assert!(named.contains("stroke=\"#aa0000\""));
    assert!(unnamed.contains("stroke=\"#444444\""));
    assert!(parse(&source.replace("checkout: alt", "alt: alt")).is_err());
    assert!(parse(&source.replace("alt \"maybe\"", "checkout: alt \"maybe\"")).is_err());
}

#[test]
fn global_fragment_stays_visible_across_inline_decomposition() {
    use laml::svg::TextMode;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nalt \"ready\" {\ndecompose API \"Internals\" {\nCache\nDB\nCache -> DB : query\n}\n}\n}\n";
    let svg = laml::sequence::render::render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(
        svg.find("data-decomposition=\"API\"").unwrap()
            < svg.find("data-fragment-operator=\"alt\"").unwrap()
    );
    assert!(svg.contains("query") && svg.contains("ref Internals"));
}

#[test]
fn css_descendant_selectors_reach_nested_messages_not_other_subdiagrams() {
    use laml::{styles::load, svg::TextMode, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI(style-classes=critical)\nWorker\ndecompose API \"One\" {\nCache\nDB\nquery: Cache -> DB : read\n}\ndecompose Worker \"Two\" {\nCache\nDB\nquery: Cache -> DB : read\n}\n}\nstyling {\nrelation[query] {\ncolor: \"#333333\"\n}\nnode[API] relation[query] {\ncolor: \"#aa0000\"\n}\nnode[Worker] relation[query] {\ncolor: \"#0000aa\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let messages: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-message=\"query\""))
        .collect();
    assert_eq!(messages.len(), 2);
    assert!(messages[0].contains("stroke=\"#aa0000\""));
    assert!(messages[1].contains("stroke=\"#0000aa\""));
    let classed = source.replace("node[API] relation[query]", "node.critical relation[query]");
    let diagram = parse(&classed).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(
        svg.lines()
            .any(|line| line.contains("data-message=\"query\"")
                && line.contains("stroke=\"#aa0000\""))
    );
}

#[test]
fn nested_annotations_take_layout_only_from_qualified_root_rules() {
    use laml::svg::TextMode;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\ndecompose API \"Internals\" {\nCache\nDB\nquery: Cache -> DB : read\nhint: note \"Cache hit\"\ncondition: continuation \"Ready\"\nat[query.receive] : invariant=\"valid\"\n}\n}\nlayout {\nAPI.Cache < API.DB\nAPI.hint over API.query.receive\nAPI.condition spans API.Cache,API.DB\n}\n";
    let diagram = parse(source).unwrap();
    let child = &diagram.decompositions[0].diagram;
    assert_eq!(child.note_layout[0].target, "query.receive");
    assert_eq!(child.continuation_layout[0].participants, ["Cache", "DB"]);
    let svg = laml::sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-note=\"hint\""));
    assert!(svg.contains("data-continuation=\"condition\""));
    assert!(svg.contains("data-annotation=\"invariant\""));
    assert!(parse(&source.replace("API.hint over API.query.receive", "API.hint over DB")).is_err());
}
