//! Asynchronous same-origin browser transport. This supplies a client adapter,
//! not a server or an authentication scheme. Never use synchronous browser IO.
use super::{
    async_client::Transport,
    protocol::{Failure, FailureCode, MAX_RESPONSE_BYTES, Response, Stamp, codec},
};
use js_sys::{
    Reflect, Uint8Array,
    futures::{JsFuture, spawn_local},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{AbortController, ReadableStreamDefaultReader, Request, RequestInit, RequestMode};

struct Flight {
    abort: AbortController,
    timer: i32,
    _callback: Closure<dyn FnMut()>,
    discardable: bool,
}
#[derive(Default)]
struct State {
    flights: BTreeMap<Stamp, Flight>,
    replies: VecDeque<Vec<u8>>,
    closed: bool,
}
pub struct WebTransport {
    endpoint: String,
    timeout_ms: i32,
    limits: bool,
    state: Rc<RefCell<State>>,
}
impl WebTransport {
    pub fn new(endpoint: &str, timeout_ms: i32) -> Result<Self, Failure> {
        // A rooted path only: no credential/query/fragment or cross-origin URL.
        if !endpoint.starts_with('/')
            || endpoint.starts_with("//")
            || endpoint
                .chars()
                .any(|c| c == '\\' || c == '?' || c == '#' || c.is_control())
            || timeout_ms < 0
        {
            return Err(Failure::protocol(
                "Invalid same-origin editor endpoint or timeout",
            ));
        }
        if web_sys::window().is_none() {
            return Err(Failure::backend("Editor browser window is unavailable"));
        }
        Ok(Self {
            endpoint: endpoint.into(),
            timeout_ms,
            limits: false,
            state: Rc::new(RefCell::new(State::default())),
        })
    }
    pub fn with_limits(mut self, limits: bool) -> Self {
        self.limits = limits;
        self
    }
}
fn network_error() -> Failure {
    Failure::new(FailureCode::Disconnected, "Editor server request failed")
}
fn stop(flight: Flight) {
    if let Some(window) = web_sys::window() {
        window.clear_timeout_with_handle(flight.timer);
    }
    flight.abort.abort();
}
impl Transport for WebTransport {
    fn send(&mut self, bytes: Vec<u8>) -> Result<(), Failure> {
        let input = codec::decode_request(&bytes)?;
        let stamp = input.stamp;
        {
            let state = self.state.borrow();
            if state.closed {
                return Err(network_error());
            }
            if state.flights.contains_key(&stamp) {
                return Err(Failure::protocol("Duplicate browser request"));
            }
        }
        let window = web_sys::window().ok_or_else(network_error)?;
        let abort = AbortController::new().map_err(|_| network_error())?;
        let init = RequestInit::new();
        init.set_method("POST");
        init.set_mode(RequestMode::SameOrigin);
        if bytes.len() <= 4096
            && matches!(
                &input.operation,
                super::protocol::Operation::CommitExport { .. }
                    | super::protocol::Operation::CancelExport { .. }
                    | super::protocol::Operation::CloseDocument
                    | super::protocol::Operation::RecoverySync { dirty: false, .. }
                    | super::protocol::Operation::RecoveryDiscard { .. }
            )
        {
            Reflect::set(
                init.as_ref(),
                &JsValue::from_str("keepalive"),
                &JsValue::TRUE,
            )
            .map_err(|_| network_error())?;
        }
        init.set_signal(Some(&abort.signal()));
        init.set_body(&Uint8Array::from(bytes.as_slice()).into());
        let request =
            Request::new_with_str_and_init(&self.endpoint, &init).map_err(|_| network_error())?;
        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(|_| network_error())?;
        request
            .headers()
            .set("X-LAML-Protocol", "1")
            .map_err(|_| network_error())?;
        let expired = Rc::new(Cell::new(false));
        let timeout = expired.clone();
        let controller = abort.clone();
        let callback = Closure::<dyn FnMut()>::new(move || {
            timeout.set(true);
            controller.abort();
        });
        let timer = if self.timeout_ms > 0 {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    self.timeout_ms,
                )
                .map_err(|_| network_error())?
        } else {
            -1
        };
        self.state.borrow_mut().flights.insert(
            stamp,
            Flight {
                abort,
                timer,
                _callback: callback,
                discardable: input.operation.discardable(),
            },
        );
        let state = self.state.clone();
        let limits = self.limits;
        spawn_local(async move {
            let result = fetch(&request, limits).await.map_err(|error| {
                if expired.get() {
                    Failure::new(FailureCode::Timeout, "Editor server request timed out")
                } else {
                    error
                }
            });
            let mut state = state.borrow_mut();
            let Some(flight) = state.flights.remove(&stamp) else {
                return;
            };
            stop(flight);
            if state.closed {
                return;
            }
            let bytes = match result {
                Ok(bytes) => Ok(bytes),
                Err(error) => codec::encode_response(&Response {
                    stamp,
                    result: Err(error),
                }),
            };
            // The fallback error envelope is tiny and has an already valid stamp.
            if let Ok(bytes) = bytes {
                state.replies.push_back(bytes);
            }
        });
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        Ok(self.state.borrow_mut().replies.pop_front())
    }
    fn forget(&mut self, stamp: Stamp) {
        let mut state = self.state.borrow_mut();
        if state
            .flights
            .get(&stamp)
            .is_some_and(|flight| flight.discardable)
            && let Some(flight) = state.flights.remove(&stamp)
        {
            stop(flight);
        }
    }
}
impl Drop for WebTransport {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.closed = true;
        // Outstanding writes retain their async task/timer until completion.
        // Dropping a tab is never permission to abort an accepted write.
        let reads = state
            .flights
            .iter()
            .filter(|(_, f)| f.discardable)
            .map(|(stamp, _)| *stamp)
            .collect::<Vec<_>>();
        for stamp in reads {
            if let Some(flight) = state.flights.remove(&stamp) {
                stop(flight);
            }
        }
        state.replies.clear();
    }
}
async fn fetch(request: &Request, limits: bool) -> Result<Vec<u8>, Failure> {
    let window = web_sys::window().ok_or_else(network_error)?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_request(request))
        .await
        .map_err(|_| network_error())?
        .dyn_into()
        .map_err(|_| network_error())?;
    if !response.ok() {
        return Err(Failure::backend(format!(
            "Editor server rejected request (HTTP {})",
            response.status()
        )));
    }
    let content_type = response
        .headers()
        .get("Content-Type")
        .map_err(|_| network_error())?
        .unwrap_or_default();
    if !content_type
        .split(';')
        .next()
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("application/json"))
    {
        return Err(Failure::protocol("Editor server did not return JSON"));
    }
    let stream = response.body().ok_or_else(network_error)?;
    let reader: ReadableStreamDefaultReader = stream
        .get_reader()
        .dyn_into()
        .map_err(|_| network_error())?;
    let mut bytes = Vec::new();
    loop {
        let chunk = JsFuture::from(reader.read())
            .await
            .map_err(|_| network_error())?;
        if Reflect::get(&chunk, &JsValue::from_str("done"))
            .map_err(|_| network_error())?
            .as_bool()
            == Some(true)
        {
            break;
        }
        let value =
            Reflect::get(&chunk, &JsValue::from_str("value")).map_err(|_| network_error())?;
        let array = Uint8Array::new(&value);
        if limits && bytes.len().saturating_add(array.length() as usize) > MAX_RESPONSE_BYTES {
            let _ = reader.cancel();
            return Err(Failure::new(
                FailureCode::Limit,
                "Editor response exceeds size limit",
            ));
        }
        let offset = bytes.len();
        bytes.resize(offset + array.length() as usize, 0);
        array.copy_to(&mut bytes[offset..]);
    }
    reader.release_lock();
    Ok(bytes)
}
