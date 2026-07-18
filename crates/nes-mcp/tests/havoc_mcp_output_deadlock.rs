use nes_mcp::{audio_chunk, frame_chunk, publish_audio_with, publish_frame_with};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn test_mcp_output_publish_frame_deadlock() {
    let (tx, rx) = mpsc::channel();

    let _handle = thread::spawn(move || {
        publish_frame_with(256, 240, |_buf| {
            // If publish_frame_with holds the mutex while executing this closure,
            // calling frame_chunk here will attempt to re-acquire the same mutex,
            // causing a deadlock.
            let _chunk = frame_chunk(0);
        });
        tx.send(()).unwrap();
    });

    // Wait for the thread to finish or timeout if deadlocked.
    let result = rx.recv_timeout(Duration::from_secs(1));

    if result.is_err() {
        panic!("timeout: Deadlock detected in publish_frame_with");
    }
}

#[test]
#[should_panic(expected = "timeout")]
fn test_mcp_output_publish_audio_deadlock() {
    let (tx, rx) = mpsc::channel();

    let _handle = thread::spawn(move || {
        publish_audio_with(735, |_buf| {
            // If publish_audio_with holds the mutex while executing this closure,
            // calling audio_chunk here will attempt to re-acquire the same mutex,
            // causing a deadlock.
            let _chunk = audio_chunk(0);
        });
        tx.send(()).unwrap();
    });

    // Wait for the thread to finish or timeout if deadlocked.
    let result = rx.recv_timeout(Duration::from_secs(1));

    if result.is_err() {
        panic!("timeout: Deadlock detected in publish_audio_with");
    }
}
