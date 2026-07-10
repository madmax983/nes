use nes_mcp::{publish_audio_with, audio_chunk};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn havoc_test_reentrant_deadlock_audio() {
    let (tx, rx) = mpsc::channel();

    let _t = thread::spawn(move || {
        publish_audio_with(735, |_| {
            // Trigger deadlock! We hold the output lock in publish_audio_with,
            // and audio_chunk attempts to acquire the exact same lock.
            let _ = audio_chunk(0);
        });

        // This will never be reached because we are deadlocked.
        tx.send(()).unwrap();
    });

    // Wait for the background thread to finish. Since it deadlocks,
    // it will time out, triggering our expected panic.
    rx.recv_timeout(Duration::from_millis(100)).expect("timeout");
}
