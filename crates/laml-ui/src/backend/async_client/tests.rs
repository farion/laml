use super::*;
use crate::backend::{Presentation, local, native_transport::NativeTransport, protocol::Reply};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    sent: Vec<Request>,
    replies: VecDeque<Result<Vec<u8>, Failure>>,
    reject: Option<Failure>,
}
struct Script(Rc<RefCell<State>>);
impl Transport for Script {
    fn send(&mut self, bytes: Vec<u8>) -> Result<(), Failure> {
        let mut state = self.0.borrow_mut();
        if let Some(error) = state.reject.take() {
            return Err(error);
        }
        state.sent.push(codec::decode_request(&bytes)?);
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        self.0.borrow_mut().replies.pop_front().transpose()
    }
}
fn client() -> (Client, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State::default()));
    (
        Client::new(11, Box::new(Script(state.clone()))).unwrap(),
        state,
    )
}
fn open() -> Operation {
    Operation::OpenSource {
        address: "drawing.laml".into(),
    }
}
fn reply(state: &Rc<RefCell<State>>, stamp: Stamp, result: Result<Reply, Failure>) {
    state.borrow_mut().replies.push_back(Ok(
        codec::encode_response(&Response { stamp, result }).unwrap()
    ));
}
fn opened() -> Reply {
    Reply::OpenedSource {
        address: "drawing.laml".into(),
        source: "source".into(),
    }
}

#[test]
fn out_of_order_results_remain_bound_to_session_document_request_and_revision() {
    let (mut client, state) = client();
    let a = client.submit(4, 0, open()).unwrap();
    let b = client.submit(5, 7, open()).unwrap();
    reply(&state, b, Ok(opened()));
    reply(&state, a, Ok(opened()));
    assert_eq!(client.poll().unwrap().stamp, b);
    assert_eq!(client.poll().unwrap().stamp, a);
    assert_eq!(client.pending(), 0);
    assert!(client.poll().is_none());
}
#[test]
fn forgotten_and_duplicate_responses_never_replace_a_current_result() {
    let (mut client, state) = client();
    let a = client.submit(4, 0, open()).unwrap();
    client.forget(a);
    let b = client.submit(4, 1, open()).unwrap();
    reply(&state, a, Ok(opened()));
    reply(&state, b, Ok(opened()));
    reply(&state, b, Ok(opened()));
    assert_eq!(client.poll().unwrap().stamp, b);
    assert!(client.poll().is_none());
    assert!(!client.failed());
}
#[test]
fn wrong_context_or_payload_fails_all_pending_requests_without_accepting_data() {
    for mutation in 0..5 {
        let (mut client, state) = client();
        let a = client.submit(4, 0, open()).unwrap();
        client.submit(5, 1, open()).unwrap();
        let mut wrong = a;
        match mutation {
            0 => wrong.session += 1,
            1 => wrong.document += 1,
            2 => wrong.revision += 1,
            _ => {}
        }
        if mutation == 4 {
            state
                .borrow_mut()
                .replies
                .push_back(Ok(b"malformed".to_vec()));
        } else {
            reply(
                &state,
                wrong,
                Ok(if mutation == 3 { Reply::Unit } else { opened() }),
            );
        }
        assert_eq!(
            client.poll().unwrap().result.unwrap_err().code,
            FailureCode::Protocol
        );
        assert!(client.poll().unwrap().result.is_err());
        assert_eq!(client.pending(), 0);
        assert!(client.failed());
        assert!(client.submit(4, 2, open()).is_err());
    }
}
#[test]
fn queue_backpressure_rejection_does_not_create_a_phantom_pending_request() {
    let (mut client, state) = client();
    state.borrow_mut().reject = Some(Failure::new(FailureCode::Busy, "Full"));
    assert_eq!(
        client.submit(4, 0, open()).unwrap_err().code,
        FailureCode::Busy
    );
    assert_eq!(client.pending(), 0);
    assert_eq!(client.submit(4, 0, open()).unwrap().request, 1);
    for _ in 0..15 {
        client.submit(4, 0, open()).unwrap();
    }
    assert_eq!(client.submit(4, 0, open()).unwrap().request, 17);
    assert_eq!(state.borrow().sent.len(), 17);
    assert_eq!(client.pending(), 17);
}
#[test]
fn disconnect_releases_every_pending_operation_as_an_explicit_failure() {
    let (mut client, state) = client();
    client.submit(4, 0, open()).unwrap();
    client.submit(5, 0, open()).unwrap();
    state
        .borrow_mut()
        .replies
        .push_back(Err(Failure::new(FailureCode::Disconnected, "Closed")));
    for _ in 0..2 {
        assert_eq!(
            client.poll().unwrap().result.unwrap_err().code,
            FailureCode::Disconnected
        );
    }
    assert!(client.poll().is_none());
    assert_eq!(client.pending(), 0);
}
#[test]
fn native_adapter_uses_real_codec_and_compile_without_blocking_the_frontend() {
    let mut client = NativeTransport::client(local());
    let stamp = client
        .submit(
            17,
            9,
            Operation::Compile {
                source: "semantics {\n A\n}\n".into(),
                address: "test.laml".into(),
                presentation: Presentation::Editor,
            },
        )
        .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    let response = loop {
        if let Some(response) = client.poll() {
            break response;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(response.stamp, stamp);
    let Reply::Compiled(Ok(preview)) = response.result.unwrap() else {
        panic!("Expected preview");
    };
    assert_eq!(preview.diagram.nodes[0].id, "A");
    assert!(preview.svg.contains("<svg"));
}
