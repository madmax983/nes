use nes_mcp::{publish_frame_with, frame_chunk};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn havoc_test_closure_deadlock() {
    let (tx, rx) = mpsc::channel();

    let _t = thread::spawn(move || {
        // The Trigger: Calling `frame_chunk` (which acquires the lock)
        // INSIDE the closure of `publish_frame_with` (which already holds the lock).
        publish_frame_with(256, 240, |_| {
            let _ = frame_chunk(0);
        });

        tx.send(()).unwrap();
    });

    // If it deadlocks, recv_timeout will fail with RecvTimeoutError::Timeout,
    // causing a panic with "timeout" which fulfills the #[should_panic].
    rx.recv_timeout(Duration::from_millis(500)).expect("timeout");
}
