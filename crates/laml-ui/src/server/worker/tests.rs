use super::*;
fn worker() -> (tempfile::TempDir, Worker) {
    let root = tempfile::tempdir().unwrap();
    let workspace_dir = root.path().join("workspace");
    std::fs::create_dir(&workspace_dir).unwrap();
    let recovery = root.path().join("recovery");
    std::fs::create_dir(&recovery).unwrap();
    let workspace = Arc::new(Workspace::open(&workspace_dir).unwrap());
    let dispatcher = Dispatcher::new(Arc::new(Scoped::new(workspace.clone())));
    (
        root,
        Worker {
            workspace,
            dispatcher,
            recovery,
            documents: BTreeMap::new(),
            stores: BTreeMap::new(),
        },
    )
}
fn request(id: u64, revision: u64, operation: Operation) -> Request {
    Request {
        stamp: protocol::Stamp {
            version: protocol::VERSION,
            session: 1,
            document: 1,
            request: id,
            revision,
        },
        operation,
    }
}
#[test]
fn replayed_and_stale_source_mutations_are_rejected_without_side_effects() {
    let (_root, mut worker) = worker();
    let save = request(
        1,
        2,
        Operation::SaveSource {
            address: "file.laml".into(),
            source: "first".into(),
            expected: None,
        },
    );
    assert!(worker.run(save.clone()).result.is_ok());
    assert!(worker.run(save).result.is_err());
    assert!(
        worker
            .run(request(
                2,
                1,
                Operation::SaveSource {
                    address: "file.laml".into(),
                    source: "stale".into(),
                    expected: Some("first".into())
                }
            ))
            .result
            .is_err()
    );
    assert_eq!(
        worker
            .workspace
            .read(Path::new("file.laml"), 100)
            .unwrap()
            .unwrap(),
        b"first"
    );
    assert!(
        worker
            .run(request(
                3,
                3,
                Operation::SaveSource {
                    address: "file.laml".into(),
                    source: "new".into(),
                    expected: Some("first".into())
                }
            ))
            .result
            .is_ok()
    );
}
#[test]
fn network_surface_excludes_raw_output_writes_and_render_without_a_ticket() {
    let (_root, mut worker) = worker();
    for operation in [
        Operation::WriteOutput {
            address: "out.svg".into(),
            data: b"not applied".to_vec(),
            expected: None,
        },
        Operation::InspectOutput {
            address: "out.svg".into(),
        },
        Operation::RenderExport {
            source: "semantics {\n A\n}\n".into(),
            address: "file.laml".into(),
            format: crate::output::Format::Svg,
        },
    ] {
        assert!(worker.run(request(1, 0, operation)).result.is_err());
    }
    assert!(
        worker
            .workspace
            .read(Path::new("out.svg"), 100)
            .unwrap()
            .is_none()
    );
}
#[test]
fn inactive_browser_keeps_its_recovery_lease_even_when_another_client_polls() {
    for limits in [false, true] {
        let (root, mut worker) = worker();
        worker.workspace = Arc::new(
            Workspace::open(&root.path().join("workspace"))
                .unwrap()
                .with_limits(limits),
        );
        assert!(
            worker
                .run(request(
                    1,
                    0,
                    Operation::RecoverySync {
                        address: Some("untitled.laml".into()),
                        source: "still editing".into(),
                        dirty: true,
                    }
                ))
                .result
                .is_ok()
        );
        let snapshot = worker.stores[&(1, 1)].pending().unwrap();
        assert!(snapshot.is_empty());
        // Simulate a background tab inactive beyond both former idle cutoffs.
        worker.documents.get_mut(&(1, 1)).unwrap().seen =
            Instant::now() - Duration::from_secs(3600);
        let mut poll = request(1, 0, Operation::RecoveryList);
        poll.stamp.session = 2;
        let Reply::RecoveryRecords(records) = worker.run(poll).result.unwrap() else {
            panic!("Expected recovery records")
        };
        assert!(records.is_empty(), "A background tab is not abandoned");
        assert!(worker.stores.contains_key(&(1, 1)));
        let Reply::RecoveryRecords(records) = worker
            .run(request(2, 0, Operation::RecoveryList))
            .result
            .unwrap()
        else {
            panic!("Expected recovery records")
        };
        assert!(
            records.is_empty(),
            "Returning must not offer its own snapshot"
        );
        // Actual worker termination still leaves the unsaved buffer recoverable.
        let recovery = worker.recovery.clone();
        drop(worker);
        let store = Store::open(recovery).unwrap();
        let records = store.pending().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "still editing");
    }
}
#[test]
fn live_recovery_is_not_listed_and_orphaned_records_have_only_opaque_ids() {
    let (_root, mut worker) = worker();
    assert!(
        worker
            .run(request(
                1,
                0,
                Operation::RecoverySync {
                    address: Some("file.laml".into()),
                    source: "unsaved buffer".into(),
                    dirty: true
                }
            ))
            .result
            .is_ok()
    );
    let response = worker.run(request(2, 0, Operation::RecoveryList));
    let Reply::RecoveryRecords(records) = response.result.unwrap() else {
        panic!("Expected recovery records")
    };
    assert!(records.is_empty());
    worker.stores.clear();
    let response = worker.run(request(3, 0, Operation::RecoveryList));
    let Reply::RecoveryRecords(records) = response.result.unwrap() else {
        panic!("Expected recovery records")
    };
    assert_eq!(records.len(), 1);
    assert!(records[0].file.is_relative());
    assert_eq!(records[0].file.components().count(), 1);
    assert_eq!(records[0].source, "unsaved buffer");
    assert!(
        worker
            .run(request(
                4,
                0,
                Operation::RecoveryDiscard {
                    id: "../../outside.json".into()
                }
            ))
            .result
            .is_err()
    );
    assert!(
        worker
            .run(request(
                5,
                0,
                Operation::RecoveryDiscard {
                    id: records[0].file.to_str().unwrap().into()
                }
            ))
            .result
            .is_ok()
    );
}
