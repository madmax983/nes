use nes_mcp::{audio_chunk, publish_audio_with};
use std::sync::mpsc;
use std::time::Duration;
use std::thread;

#[test]
fn havoc_test_publish_audio_deadlock() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        publish_audio_with(256, |_| {
            // This attempts to acquire the lock again, deadlocking the thread!
            let _ = audio_chunk(0);
        });
        let _ = tx.send(());
    });

    // If it deadlocks, we will timeout waiting for the response
    if rx.recv_timeout(Duration::from_millis(500)).is_err() {
        panic!("💀 Havoc win: DEADLOCK DETECTED! The thread is stuck waiting on the mutex.");
    }
}
