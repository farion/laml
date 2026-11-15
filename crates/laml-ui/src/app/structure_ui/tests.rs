use super::*;
fn ready(source: &str, path: Option<PathBuf>) -> (Editor, egui::Context) {
    let mut e = Editor::new(source.into(), path);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    e.changed = Instant::now() - Duration::from_secs(1);
    super::super::tests::wait(&mut e, &ctx, |e| e.rendered == Some(0));
    (e, ctx)
}
fn changed(e: &mut Editor, ctx: &egui::Context, rev: u64) {
    super::super::tests::wait(e, ctx, |e| e.document.revision() == rev);
    assert!(e.command_error.is_none(), "{:?}", e.command_error);
}
#[test]
fn declaration_and_body_proposals_preflight_and_share_undo() {
    let source = "semantics {\n class C\n}\n";
    let (mut e, ctx) = ready(source, None);
    e.selection.click(Some("C"), false);
    let edits = structure::body(source, "C", "    field +value: String");
    e.propose_structure("Body", Some("C".into()), edits);
    assert!(e.structure.proposal.is_some());
    let p = e.structure.proposal.take().unwrap();
    e.submit_structure(&p.label, p.owner, Ok(p.edits));
    changed(&mut e, &ctx, 1);
    assert_eq!(e.preview.as_ref().unwrap().diagram.nodes[0].parts.len(), 1);
    assert!(e.document.undo());
    assert_eq!(e.document.source(), source);
    e.propose_structure(
        "Body",
        Some("C".into()),
        structure::body(source, "C", "field x"),
    );
    e.document.replace_text("newer".into()).unwrap();
    let mut output = ctx.run_ui(Default::default(), |ui| e.structure_window(ui.ctx()));
    output.textures_delta.clear();
    assert!(e.structure.proposal.is_none());
    assert_eq!(e.document.source(), "newer");
}
#[test]
fn shadowed_member_and_compartment_overrides_do_not_modify_source() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("late.lamlstyle"),"styling {\n member[C::name] {\n color: #abcdef\n }\n node[C] compartment[attributes] {\n display: none\n }\n}\n").unwrap();
    let source = "semantics {\n class C {\n field +name: String\n }\n}\nstyling {\n member[C::name] {\n color: #111111\n }\n node[C] compartment[attributes] {\n display: none\n }\n import \"late.lamlstyle\"\n}\n";
    let path = d.path().join("test.laml");
    std::fs::write(&path, source).unwrap();
    let (mut e, ctx) = ready(source, Some(path));
    let edits = structure::member_style(source, "C", "C::name", "color", Some("#123456")).unwrap();
    e.submit_command(Command {
        edits,
        label: "Color".into(),
        expectation: worker::Expectation::MemberStyle {
            owner: "C".into(),
            address: "C::name".into(),
            name: "color".into(),
            value: Some("#123456".into()),
        },
    });
    super::super::tests::wait(&mut e, &ctx, |e| e.command_error.is_some());
    assert_eq!(e.document.source(), source);
    e.command_error = None;
    let edits = structure::compartment(source, "C", "attributes", Some("block")).unwrap();
    // Force an actual local change, retaining the later imported winner.
    let edits = if edits.is_empty() {
        structure::compartment(source, "C", "attributes", Some("none")).unwrap()
    } else {
        edits
    };
    e.submit_command(Command {
        edits,
        label: "Visibility".into(),
        expectation: worker::Expectation::CompartmentStyle {
            owner: "C".into(),
            kind: "attributes".into(),
            value: Some("block".into()),
        },
    });
    super::super::tests::wait(&mut e, &ctx, |e| e.command_error.is_some());
    assert_eq!(e.document.source(), source);
}
#[test]
fn full_gne_authoring_export_and_undo_flow() {
    let source = "semantics {\n}\n";
    let (mut e, ctx) = ready(source, None);
    e.creation.palette.kind = laml_ui::element_edit::ElementKind::Class;
    e.creation.label = "Service".into();
    e.create_element();
    changed(&mut e, &ctx, 1);
    e.creation.palette.kind = laml_ui::element_edit::ElementKind::Rectangle;
    e.creation.label = "Client".into();
    e.create_element();
    changed(&mut e, &ctx, 2);
    let edits = structure::body(e.document.source(), "Node1", "field +id: Int");
    e.submit_structure("Body", Some("Node1".into()), edits);
    changed(&mut e, &ctx, 3);
    e.create_port("Node1", "In");
    changed(&mut e, &ctx, 4);
    e.create_relation("Node2", "Port1");
    changed(&mut e, &ctx, 5);
    e.creation.palette.kind = laml_ui::element_edit::ElementKind::Note;
    e.creation.label = "Documentation".into();
    e.create_element();
    changed(&mut e, &ctx, 6);
    let line = e.preview.as_ref().unwrap().diagram.relations[0].line;
    let edits = structure::attach(e.document.source(), "Note1", line);
    e.submit_structure("Attach", Some("Note1".into()), edits);
    changed(&mut e, &ctx, 7);
    let edits = structure::member_style(
        e.document.source(),
        "Node1",
        "Node1::id",
        "font-size",
        Some("22"),
    )
    .unwrap();
    e.submit_command(Command {
        edits,
        label: "Member size".into(),
        expectation: worker::Expectation::MemberStyle {
            owner: "Node1".into(),
            address: "Node1::id".into(),
            name: "font-size".into(),
            value: Some("22".into()),
        },
    });
    changed(&mut e, &ctx, 8);
    e.format_source();
    assert_eq!(e.document.revision(), 9);
    super::super::tests::wait(&mut e, &ctx, |e| e.rendered == Some(9));
    let d = tempfile::tempdir().unwrap();
    for format in [
        laml_ui::output::Format::Svg,
        laml_ui::output::Format::Png,
        laml_ui::output::Format::Pdf,
    ] {
        e.workflow_tick();
        let path = d.path().join(format!("output.{}", format.extension()));
        e.start_export(path.clone(), format);
        let end = Instant::now() + Duration::from_secs(10);
        while !path.exists() || e.workflow.busy() {
            assert!(Instant::now() < end, "{:?}", e.workflow.status);
            e.workflow_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(std::fs::metadata(path).unwrap().len() > 100);
    }
    assert_eq!(e.document.history_usage().0, 9);
    for _ in 0..9 {
        assert!(e.document.undo());
    }
    assert_eq!(e.document.source(), source);
}
