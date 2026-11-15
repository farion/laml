use super::dispatch::Dispatcher;
use super::*;
use crate::backend::local;

fn request(id: u64, operation: Operation) -> Request {
    Request {
        stamp: Stamp {
            version: VERSION,
            session: 1,
            document: 2,
            request: id,
            revision: 7,
        },
        operation,
    }
}
fn call(dispatcher: &mut Dispatcher, request: Request) -> Response {
    let input = codec::decode_request(&codec::encode_request(&request).unwrap()).unwrap();
    let response = dispatcher.execute(input);
    codec::decode_response(&codec::encode_response(&response).unwrap()).unwrap()
}
fn begin(dir: &std::path::Path, source: &str) -> Operation {
    Operation::BeginExport {
        address: address(&dir.join("out.svg")).unwrap(),
        source_address: address(&dir.join("source.laml")).unwrap(),
        source: source.into(),
        format: Format::Svg,
    }
}
fn ticket(response: Response) -> u64 {
    let Reply::ExportPrepared { ticket, .. } = response.result.unwrap() else {
        panic!("Expected prepared export");
    };
    ticket
}
const SOURCE: &str = "semantics {\n A\n}\n";

#[test]
fn export_ticket_is_bound_to_session_document_and_revision_and_commits_only_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut dispatcher = Dispatcher::new(local());
    let id = ticket(call(&mut dispatcher, request(1, begin(dir.path(), SOURCE))));
    assert!(!dir.path().join("out.svg").exists());
    for mutation in 0..3 {
        let mut input = request(2 + mutation, Operation::CommitExport { ticket: id });
        match mutation {
            0 => input.stamp.session += 1,
            1 => input.stamp.document += 1,
            _ => input.stamp.revision += 1,
        }
        assert!(call(&mut dispatcher, input).result.is_err());
        assert!(!dir.path().join("out.svg").exists());
    }
    assert!(
        call(
            &mut dispatcher,
            request(5, Operation::CommitExport { ticket: id })
        )
        .result
        .is_ok()
    );
    assert!(
        std::fs::read_to_string(dir.path().join("out.svg"))
            .unwrap()
            .contains("<svg")
    );
    assert!(
        call(
            &mut dispatcher,
            request(6, Operation::CommitExport { ticket: id })
        )
        .result
        .is_err()
    );
}
#[test]
fn replacing_cancelling_and_closing_release_tickets_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let mut dispatcher = Dispatcher::new(local());
    let first = ticket(call(&mut dispatcher, request(1, begin(dir.path(), SOURCE))));
    let second = ticket(call(&mut dispatcher, request(2, begin(dir.path(), SOURCE))));
    assert_ne!(first, second);
    assert!(
        call(
            &mut dispatcher,
            request(3, Operation::CommitExport { ticket: first })
        )
        .result
        .is_err()
    );
    assert!(
        call(
            &mut dispatcher,
            request(4, Operation::CancelExport { ticket: second })
        )
        .result
        .is_ok()
    );
    assert!(
        call(
            &mut dispatcher,
            request(5, Operation::CommitExport { ticket: second })
        )
        .result
        .is_err()
    );
    let third = ticket(call(&mut dispatcher, request(6, begin(dir.path(), SOURCE))));
    assert!(
        call(&mut dispatcher, request(7, Operation::CloseDocument))
            .result
            .is_ok()
    );
    assert!(
        call(
            &mut dispatcher,
            request(8, Operation::CommitExport { ticket: third })
        )
        .result
        .is_err()
    );
    assert!(!dir.path().join("out.svg").exists());
}
#[test]
fn output_and_dependency_changes_after_prepare_prevent_the_final_write() {
    for change_dependency in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = format!("{SOURCE}styling {{\n import \"theme.lamlstyle\"\n}}\n");
        let theme = dir.path().join("theme.lamlstyle");
        let output = dir.path().join("out.svg");
        std::fs::write(&theme, "styling {\n}\n").unwrap();
        std::fs::write(&output, "original").unwrap();
        let mut dispatcher = Dispatcher::new(local());
        let id = ticket(call(
            &mut dispatcher,
            request(1, begin(dir.path(), &source)),
        ));
        if change_dependency {
            std::fs::write(theme, "// dependency changed\nstyling {\n}\n").unwrap();
        } else {
            std::fs::write(&output, "external").unwrap();
        }
        assert!(
            call(
                &mut dispatcher,
                request(2, Operation::CommitExport { ticket: id })
            )
            .result
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            if change_dependency {
                "original"
            } else {
                "external"
            }
        );
    }
}
