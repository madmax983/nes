use nes_mcp::{frame_chunk, publish_frame_with};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn havoc_test_deadlock_publish_frame_with() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        publish_frame_with(256, 240, |_| {
            // This attempts to acquire the output state lock again, which is already held
            // by publish_frame_with, causing a deadlock.
            let _ = frame_chunk(0);
        });
        let _ = tx.send(());
    });

    rx.recv_timeout(Duration::from_millis(100)).expect("timeout");
}
