use nes_mcp::{audio_chunk, frame_chunk, publish_audio_with, publish_frame_with};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn havoc_deadlock_publish_frame_with() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // The trigger: The closure passed to `publish_frame_with` is executed while holding the Mutex.
        // If the closure attempts to acquire the same lock (e.g., via `frame_chunk`), it deadlocks.
        publish_frame_with(256, 240, |_| {
            let _ = frame_chunk(0);
        });
        tx.send(()).unwrap();
    });

    // Wait for up to 500ms. If it doesn't finish, we've successfully proven the deadlock.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}

#[test]
#[should_panic(expected = "timeout")]
fn havoc_deadlock_publish_audio_with() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // The trigger: The closure passed to `publish_audio_with` is executed while holding the Mutex.
        // If the closure attempts to acquire the same lock (e.g., via `audio_chunk`), it deadlocks.
        publish_audio_with(256, |_| {
            let _ = audio_chunk(0);
        });
        tx.send(()).unwrap();
    });

    // Wait for up to 500ms. If it doesn't finish, we've successfully proven the deadlock.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}
