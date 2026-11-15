//! Admission leases contain only opaque browser IDs and counters, never documents.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const IDLE: Duration = Duration::from_secs(600);
struct Session {
    seen: Instant,
    running: usize,
}
pub(super) struct Sessions {
    limit: usize,
    active: BTreeMap<String, Session>,
}
impl Sessions {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            active: BTreeMap::new(),
        }
    }
    fn admit(&mut self, id: &str, now: Instant) -> bool {
        self.active
            .retain(|_, s| s.running > 0 || now.duration_since(s.seen) < IDLE);
        if !self.active.contains_key(id) && self.active.len() >= self.limit {
            return false;
        }
        self.active
            .entry(id.into())
            .or_insert(Session {
                seen: now,
                running: 0,
            })
            .seen = now;
        true
    }
    pub fn heartbeat(&mut self, id: &str) -> bool {
        self.admit(id, Instant::now())
    }
}
pub(super) struct Lease {
    sessions: Arc<Mutex<Sessions>>,
    id: String,
}
impl Lease {
    pub fn acquire(sessions: &Arc<Mutex<Sessions>>, id: &str) -> Option<Self> {
        let mut state = sessions.lock().ok()?;
        if !state.admit(id, Instant::now()) {
            return None;
        }
        state.active.get_mut(id)?.running += 1;
        Some(Self {
            sessions: sessions.clone(),
            id: id.into(),
        })
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.sessions.lock()
            && let Some(s) = state.active.get_mut(&self.id)
        {
            s.running = s.running.saturating_sub(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_tabs_timeout_and_inflight() {
        let mut s = Sessions::new(1);
        let now = Instant::now();
        assert!(s.admit("browser-a", now));
        assert!(s.admit("browser-a", now));
        assert!(!s.admit("browser-b", now + Duration::from_secs(599)));
        s.active.get_mut("browser-a").unwrap().running = 1;
        assert!(!s.admit("browser-b", now + IDLE));
        s.active.get_mut("browser-a").unwrap().running = 0;
        assert!(s.admit("browser-b", now + IDLE));
    }
}
