//! Bounded native adapter. The same JSON codec runs in both directions, so a
//! direct call cannot silently bypass the version/correlation/serialization contract.
use super::{
    Handle,
    async_client::{Client, Transport},
    protocol::{Failure, FailureCode, Response, Stamp, codec, dispatch::Dispatcher},
};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};

pub struct NativeTransport {
    requests: mpsc::Sender<Vec<u8>>,
    responses: mpsc::Receiver<Result<Vec<u8>, Failure>>,
    queued: Arc<Mutex<BTreeMap<Stamp, bool>>>,
}
impl NativeTransport {
    pub fn new(backend: Handle) -> Self {
        Self::start(backend, || {})
    }
    fn start(backend: Handle, before: impl FnOnce() + Send + 'static) -> Self {
        let (requests, incoming) = mpsc::channel::<Vec<u8>>();
        let (outgoing, responses) = mpsc::channel();
        let queued = Arc::new(Mutex::new(BTreeMap::new()));
        let pending = queued.clone();
        std::thread::spawn(move || {
            before();
            let mut dispatcher = Dispatcher::new(backend);
            while let Ok(bytes) = incoming.recv() {
                let request = match codec::decode_request(&bytes) {
                    Ok(request) => request,
                    Err(error) => {
                        if outgoing.send(Err(error)).is_err() {
                            break;
                        }
                        continue;
                    }
                };
                let wanted = pending
                    .lock()
                    .expect("Queued editor requests")
                    .remove(&request.stamp)
                    .unwrap_or(true);
                if !wanted && request.operation.discardable() {
                    continue;
                }
                let result = {
                    let stamp = request.stamp;
                    let response = dispatcher.execute(request);
                    match codec::encode_response(&response) {
                        Ok(bytes) => Ok(bytes),
                        // Return a bounded correlated error even if the payload is
                        // too large or contains unrepresentable compiler data.
                        Err(error) => codec::encode_response(&Response {
                            stamp,
                            result: Err(error),
                        }),
                    }
                };
                if outgoing.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            requests,
            responses,
            queued,
        }
    }
    pub fn client(backend: Handle) -> Client {
        static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);
        let session = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .expect("Editor session IDs exhausted");
        Client::new(session, Box::new(Self::new(backend))).expect("Nonzero native session")
    }
}
impl Transport for NativeTransport {
    fn send(&mut self, bytes: Vec<u8>) -> Result<(), Failure> {
        let stamp = codec::decode_request(&bytes)?.stamp;
        let mut queued = self.queued.lock().expect("Queued editor requests");
        if queued.contains_key(&stamp) {
            return Err(Failure::protocol("Duplicate native request"));
        }
        queued.insert(stamp, true);
        self.requests.send(bytes).map_err(|_| {
            queued.remove(&stamp);
            Failure::new(FailureCode::Disconnected, "Editor backend stopped")
        })
    }

    fn forget(&mut self, stamp: Stamp) {
        if let Some(wanted) = self
            .queued
            .lock()
            .expect("Queued editor requests")
            .get_mut(&stamp)
        {
            *wanted = false;
        }
    }
    fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        match self.responses.try_recv() {
            Ok(result) => result.map(Some),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(Failure::new(
                FailureCode::Disconnected,
                "Editor backend stopped",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{
        Presentation, local,
        protocol::{self, Operation, Request},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn obsolete_queued_compile_is_skipped_but_forgetting_a_save_cannot_cancel_it() {
        let (release, wait) = mpsc::channel();
        let mut transport = NativeTransport::start(local(), move || wait.recv().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("saved.laml");
        let a = Stamp {
            version: protocol::VERSION,
            session: 1,
            document: 1,
            request: 1,
            revision: 0,
        };
        let b = Stamp { request: 2, ..a };
        transport
            .send(
                codec::encode_request(&Request {
                    stamp: a,
                    operation: Operation::Compile {
                        source: "invalid".into(),
                        address: "test.laml".into(),
                        presentation: Presentation::Editor,
                    },
                })
                .unwrap(),
            )
            .unwrap();
        transport.forget(a);
        transport
            .send(
                codec::encode_request(&Request {
                    stamp: b,
                    operation: Operation::SaveSource {
                        address: protocol::address(&output).unwrap(),
                        source: "saved".into(),
                        expected: None,
                    },
                })
                .unwrap(),
            )
            .unwrap();
        transport.forget(b);
        release.send(()).unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        let response = loop {
            if let Some(bytes) = transport.poll().unwrap() {
                break codec::decode_response(&bytes).unwrap();
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(response.stamp, b);
        assert!(response.result.is_ok());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "saved");
        assert!(transport.queued.lock().unwrap().is_empty());
    }
}
