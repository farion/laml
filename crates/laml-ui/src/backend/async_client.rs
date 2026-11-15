//! Nonblocking frontend contract, independent of native threads and HTTP.
use super::protocol::{
    self, Failure, FailureCode, Operation, ReplyKind, Request, Response, Stamp, codec,
};
use std::collections::{BTreeMap, VecDeque};

/// send/poll must return immediately. Browser adapters use asynchronous fetch or
/// sockets and deposit completed frames for poll; never synchronous HTTP.
pub trait Transport {
    fn send(&mut self, bytes: Vec<u8>) -> Result<(), Failure>;
    fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure>;
    /// Advisory only: implementations may skip queued read-only work. Already
    /// running work and side-effecting operations must not be assumed cancelled.
    fn forget(&mut self, _stamp: Stamp) {}
}
struct Pending {
    stamp: Stamp,
    kind: ReplyKind,
}
pub struct Client {
    session: u64,
    next_request: u64,
    transport: Box<dyn Transport>,
    pending: BTreeMap<u64, Pending>,
    ready: VecDeque<Response>,
    failure: Option<Failure>,
    limits: bool,
}
impl Client {
    pub fn new(session: u64, transport: Box<dyn Transport>) -> Result<Self, Failure> {
        if session == 0 {
            return Err(Failure::protocol("Invalid editor session"));
        }
        Ok(Self {
            session,
            next_request: 1,
            transport,
            pending: BTreeMap::new(),
            ready: VecDeque::new(),
            failure: None,
            limits: false,
        })
    }
    pub fn with_limits(mut self, limits: bool) -> Self {
        self.limits = limits;
        self
    }
    pub fn submit(
        &mut self,
        document: u64,
        revision: u64,
        operation: Operation,
    ) -> Result<Stamp, Failure> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        let stamp = Stamp {
            version: protocol::VERSION,
            session: self.session,
            document,
            request: self.next_request,
            revision,
        };
        let next = self
            .next_request
            .checked_add(1)
            .ok_or_else(|| Failure::new(FailureCode::Limit, "Request IDs exhausted"))?;
        let kind = operation.reply_kind();
        let bytes = codec::encode_request_with_limits(&Request { stamp, operation }, self.limits)?;
        self.transport.send(bytes)?;
        self.next_request = next;
        self.pending.insert(stamp.request, Pending { stamp, kind });
        Ok(stamp)
    }
    /// Forget a reply, not an already accepted side effect. Saving/export writing
    /// must use explicit pending UI states rather than relying on cancellation.
    pub fn forget(&mut self, stamp: Stamp) {
        if self
            .pending
            .get(&stamp.request)
            .is_some_and(|p| p.stamp == stamp)
        {
            self.pending.remove(&stamp.request);
            self.transport.forget(stamp);
        }
        self.ready.retain(|r| r.stamp != stamp);
    }
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
    pub fn failed(&self) -> bool {
        self.failure.is_some()
    }
    pub fn poll(&mut self) -> Option<Response> {
        if let Some(response) = self.ready.pop_front() {
            return Some(response);
        }
        if self.failed() {
            return None;
        }
        // Limit work per frame even when a transport contains many obsolete replies.
        for _ in 0..32 {
            let bytes = match self.transport.poll() {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return None,
                Err(error) => {
                    self.fail(error);
                    return self.ready.pop_front();
                }
            };
            let response = match codec::decode_response_with_limits(&bytes, self.limits) {
                Ok(response) => response,
                Err(error) => {
                    self.fail(error);
                    return self.ready.pop_front();
                }
            };
            let Some(pending) = self.pending.get(&response.stamp.request) else {
                continue; // forgotten, duplicate or unsolicited; never apply it.
            };
            if response.stamp != pending.stamp
                || response
                    .result
                    .as_ref()
                    .is_ok_and(|r| r.kind() != pending.kind)
            {
                self.fail(Failure::protocol(
                    "Editor response does not match its request",
                ));
                return self.ready.pop_front();
            }
            self.pending.remove(&response.stamp.request);
            return Some(response);
        }
        None
    }
    fn fail(&mut self, error: Failure) {
        self.failure = Some(error.clone());
        for (_, pending) in std::mem::take(&mut self.pending) {
            self.ready.push_back(Response {
                stamp: pending.stamp,
                result: Err(error.clone()),
            });
        }
    }
}
#[cfg(test)]
mod tests;
