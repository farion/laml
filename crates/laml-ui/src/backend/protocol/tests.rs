use super::*;
use crate::backend::{local, protocol::dispatch::Dispatcher};
use std::path::Path;

fn request(id: u64, operation: Operation) -> Request {
    Request {
        stamp: Stamp {
            version: VERSION,
            session: 1,
            document: 3,
            request: id,
            revision: 7,
        },
        operation,
    }
}
fn roundtrip(dispatcher: &mut Dispatcher, request: Request) -> Response {
    let input = codec::decode_request(&codec::encode_request(&request).unwrap()).unwrap();
    let response = dispatcher.execute(input);
    codec::decode_response(&codec::encode_response(&response).unwrap()).unwrap()
}

#[test]
fn compiler_roundtrip_retains_svg_geometry_picking_styles_and_properties() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/architecture.laml");
    let source = std::fs::read_to_string(&path).unwrap();
    let preview = local()
        .compile(&source, &path, Presentation::Editor)
        .unwrap();
    let expected = serde_json::to_value(&preview).unwrap();
    let mut dispatcher = Dispatcher::new(local());
    let response = roundtrip(
        &mut dispatcher,
        request(
            1,
            Operation::Compile {
                source,
                address: address(&path).unwrap(),
                presentation: Presentation::Editor,
            },
        ),
    );
    assert_eq!(response.stamp.revision, 7);
    let Reply::Compiled(Ok(actual)) = response.result.unwrap() else {
        panic!("Expected preview");
    };
    assert_eq!(serde_json::to_value(&actual).unwrap(), expected);
    assert!(!actual.route_paths.is_empty());
    assert!(!actual.node_properties.is_empty());
    assert!(!actual.styles.rules.is_empty());
    assert_eq!(actual.routes[0].source_line, preview.routes[0].source_line);
}
#[test]
fn compile_diagnostics_keep_real_file_and_line_without_becoming_service_errors() {
    let mut dispatcher = Dispatcher::new(local());
    let response = roundtrip(
        &mut dispatcher,
        request(
            1,
            Operation::Compile {
                source: "semantics {\n A -> Missing\n}\n".into(),
                address: "bad.laml".into(),
                presentation: Presentation::Editor,
            },
        ),
    );
    let Reply::Compiled(Err(diagnostic)) = response.result.unwrap() else {
        panic!("Expected compiler diagnostic");
    };
    assert_eq!(diagnostic.line, Some(2));
    assert_eq!(diagnostic.path.as_deref(), Some(Path::new("bad.laml")));
}
#[test]
fn malformed_unknown_oversize_and_incompatible_requests_are_rejected() {
    assert!(codec::decode_request(b"not json").is_err());
    let mut value = serde_json::to_value(request(
        1,
        Operation::OpenSource {
            address: "x".into(),
        },
    ))
    .unwrap();
    value["stamp"]["version"] = 999.into();
    assert!(codec::decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
    value["stamp"]["version"] = VERSION.into();
    value["stamp"]["extra"] = true.into();
    assert!(codec::decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
    value["stamp"].as_object_mut().unwrap().remove("extra");
    value["operation"]["extra"] = true.into();
    assert!(codec::decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
    assert_eq!(
        codec::decode_request_with_limits(&vec![b' '; MAX_REQUEST_BYTES + 1], true)
            .unwrap_err()
            .code,
        FailureCode::Limit
    );
    assert!(
        codec::encode_request(&request(
            1,
            Operation::OpenSource {
                address: "\0".into()
            }
        ))
        .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("must-not-exist.laml");
    let mut invalid = request(
        0,
        Operation::SaveSource {
            address: address(&path).unwrap(),
            source: "bad".into(),
            expected: None,
        },
    );
    invalid.stamp.version = VERSION + 1;
    assert!(Dispatcher::new(local()).execute(invalid).result.is_err());
    assert!(!path.exists());
}

#[test]
fn source_and_export_writes_preserve_confirmation_and_dependency_protection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("drawing.laml");
    let source = "semantics {\n A\n}\nstyling {\n import \"palette.svg\"\n}\n";
    std::fs::write(dir.path().join("palette.svg"), "styling {\n}\n").unwrap();
    let input = address(&path).unwrap();
    let mut dispatcher = Dispatcher::new(local());
    let save = |expected| Operation::SaveSource {
        address: input.clone(),
        source: source.into(),
        expected,
    };
    assert!(
        roundtrip(&mut dispatcher, request(1, save(None)))
            .result
            .is_ok()
    );
    assert!(
        roundtrip(&mut dispatcher, request(2, save(None)))
            .result
            .is_err()
    );
    let response = roundtrip(
        &mut dispatcher,
        request(
            3,
            Operation::OpenSource {
                address: input.clone(),
            },
        ),
    );
    assert!(
        matches!(response.result, Ok(Reply::OpenedSource { source: actual, .. }) if actual == source)
    );
    std::fs::write(&path, "external").unwrap();
    assert!(
        roundtrip(&mut dispatcher, request(4, save(Some(source.into()))))
            .result
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "external");
    let forbidden = Operation::PrepareExport {
        address: address(&dir.path().join("palette.svg")).unwrap(),
        source_address: input,
        source: source.into(),
        format: Format::Svg,
    };
    assert!(
        roundtrip(&mut dispatcher, request(5, forbidden))
            .result
            .is_err()
    );
    let output = dir.path().join("export.svg");
    let operation = |expected| Operation::WriteOutput {
        address: address(&output).unwrap(),
        data: b"rendered".to_vec(),
        expected,
    };
    assert!(
        roundtrip(&mut dispatcher, request(6, operation(None)))
            .result
            .is_ok()
    );
    std::fs::write(&output, "external").unwrap();
    assert!(
        roundtrip(
            &mut dispatcher,
            request(7, operation(Some(b"rendered".to_vec())))
        )
        .result
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), "external");
}
#[test]
fn dependency_watches_are_owned_replaced_and_released_per_session_document() {
    let dir = tempfile::tempdir().unwrap();
    let mut dispatcher = Dispatcher::new(local());
    let operation = || Operation::WatchDependencies {
        source: "semantics {\n A\n}\n".into(),
        address: address(&dir.path().join("drawing.laml")).unwrap(),
    };
    let response = roundtrip(&mut dispatcher, request(1, operation()));
    let Reply::DependencyWatch { watch, .. } = response.result.unwrap() else {
        panic!();
    };
    let mut foreign = request(2, Operation::DependenciesChanged { watch });
    foreign.stamp.document += 1;
    assert!(roundtrip(&mut dispatcher, foreign).result.is_err());
    let mut foreign = request(3, Operation::DependenciesChanged { watch });
    foreign.stamp.session += 1;
    assert!(roundtrip(&mut dispatcher, foreign).result.is_err());
    assert!(matches!(
        roundtrip(
            &mut dispatcher,
            request(4, Operation::DependenciesChanged { watch })
        )
        .result,
        Ok(Reply::DependenciesChanged(false))
    ));
    let response = roundtrip(&mut dispatcher, request(5, operation()));
    let Reply::DependencyWatch { watch: newer, .. } = response.result.unwrap() else {
        panic!();
    };
    assert_ne!(watch, newer);
    assert!(
        roundtrip(
            &mut dispatcher,
            request(6, Operation::DependenciesChanged { watch })
        )
        .result
        .is_err()
    );
    assert!(
        roundtrip(&mut dispatcher, request(7, Operation::CloseDocument))
            .result
            .is_ok()
    );
    assert!(
        roundtrip(
            &mut dispatcher,
            request(8, Operation::DependenciesChanged { watch: newer })
        )
        .result
        .is_err()
    );
}

#[test]
fn roundtrip_preserves_members_ports_parallel_routes_and_self_loops() {
    for source in [
        "semantics {\n A {\n +run()\n @P port Out\n }\n B\n P -> B : request\n}\n",
        "semantics {\n package P {\n A\n }\n B\n P.A -> B\n P.A -> B\n B -> B\n}\n",
    ] {
        let path = Path::new("test.laml");
        let expected = local().compile(source, path, Presentation::Editor).unwrap();
        let mut dispatcher = Dispatcher::new(local());
        let response = roundtrip(
            &mut dispatcher,
            request(
                1,
                Operation::Compile {
                    source: source.into(),
                    address: "test.laml".into(),
                    presentation: Presentation::Editor,
                },
            ),
        );
        let Reply::Compiled(Ok(actual)) = response.result.unwrap() else {
            panic!("Expected preview");
        };
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}

#[test]
fn export_messages_use_document_theme_and_missing_snapshot_is_not_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("drawing.laml");
    let source = "settings {\n theme: plain\n}\nsemantics {\n A\n}\n";
    let mut dispatcher = Dispatcher::new(local());
    let response = roundtrip(
        &mut dispatcher,
        request(
            1,
            Operation::RenderExport {
                source: source.into(),
                address: address(&path).unwrap(),
                format: Format::Svg,
            },
        ),
    );
    let Reply::Export(bytes) = response.result.unwrap() else {
        panic!("Expected export");
    };
    assert_eq!(
        bytes,
        local().render_export(source, &path, Format::Svg).unwrap()
    );
    let response = roundtrip(
        &mut dispatcher,
        request(
            2,
            Operation::InspectSource {
                address: address(&path).unwrap(),
            },
        ),
    );
    assert!(matches!(
        response.result,
        Ok(Reply::SourceSnapshot { source: None, .. })
    ));
    let response = roundtrip(
        &mut dispatcher,
        request(
            3,
            Operation::InspectSource {
                address: address(dir.path()).unwrap(),
            },
        ),
    );
    assert!(response.result.is_err());
}

#[test]
fn default_codec_accepts_messages_above_optional_size_limits() {
    let input = request(
        1,
        Operation::Compile {
            source: "x".repeat(MAX_REQUEST_BYTES + 1),
            address: "large.laml".into(),
            presentation: Presentation::Editor,
        },
    );
    assert!(codec::encode_request_with_limits(&input, true).is_err());
    let bytes = codec::encode_request(&input).unwrap();
    assert!(codec::decode_request(&bytes).is_ok());
    assert!(codec::decode_request_with_limits(&bytes, true).is_err());
}
