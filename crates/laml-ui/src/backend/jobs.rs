//! Native job execution, kept outside the UI and separate from backend services.
use std::sync::mpsc::{self, Receiver, Sender};

pub type Channels<J, R> = (Sender<J>, Receiver<R>);

/// Keep the in-flight job, then coalesce waiting requests to the newest one.
/// Each caller owns an independent queue. Dropping the sender stops the worker;
/// dropping the receiver prevents publishing further results. Revision checks
/// remain the frontend's responsibility: this does not cancel in-flight work.
pub fn latest<J: Send + 'static, R: Send + 'static>(
    mut run: impl FnMut(J) -> R + Send + 'static,
) -> Channels<J, R> {
    let (requests, incoming) = mpsc::channel::<J>();
    let (outgoing, results) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(mut job) = incoming.recv() {
            while let Ok(newer) = incoming.try_recv() {
                job = newer;
            }
            if outgoing.send(run(job)).is_err() {
                break;
            }
        }
    });
    (requests, results)
}

pub fn once<R: Send + 'static>(run: impl FnOnce() -> R + Send + 'static) -> Receiver<R> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(run());
    });
    receive
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn finishes_in_flight_then_coalesces_and_exits_after_sender_drop() {
        let (started, wait_started) = mpsc::channel();
        let (release, wait_release) = mpsc::channel();
        let (jobs, results) = latest(move |n| {
            if n == 1 {
                started.send(()).unwrap();
                wait_release.recv().unwrap();
            }
            n
        });
        jobs.send(1).unwrap();
        wait_started.recv_timeout(Duration::from_secs(5)).unwrap();
        jobs.send(2).unwrap();
        jobs.send(3).unwrap();
        release.send(()).unwrap();
        assert_eq!(results.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
        assert_eq!(results.recv_timeout(Duration::from_secs(5)).unwrap(), 3);
        drop(jobs);
        assert_eq!(
            results.recv_timeout(Duration::from_secs(5)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn queues_are_independent_and_one_shot_delivers_once() {
        let (a, ra) = latest(|n: u64| n + 1);
        let (b, rb) = latest(|n: u64| n + 10);
        a.send(4).unwrap();
        b.send(4).unwrap();
        assert_eq!(ra.recv_timeout(Duration::from_secs(5)).unwrap(), 5);
        assert_eq!(rb.recv_timeout(Duration::from_secs(5)).unwrap(), 14);
        let result = once(|| 42);
        assert_eq!(result.recv_timeout(Duration::from_secs(5)).unwrap(), 42);
        assert_eq!(
            result.recv_timeout(Duration::from_secs(5)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        );
    }
}
