use nes_mcp::publish_frame_with;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "output state lock")]
#[ignore = "Havoc Mutex Poison Attack"]
fn havoc_test_mutex_poison() {
    let (tx, rx) = mpsc::channel();

    let _t = thread::spawn(move || {
        // We trigger a panic inside the `publish_frame_with` closure while it holds the state lock.
        // This simulates a bug in frame preparation that crashes while the lock is held.
        let _ = std::panic::catch_unwind(|| {
            publish_frame_with(256, 240, |_| {
                panic!("Havoc closure panic");
            });
        });

        tx.send(()).unwrap();
    });

    // Wait for the background thread to panic
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");

    // The state lock is now poisoned.
    // If the application attempts to read a frame later, it will panic unconditionally.
    let _ = nes_mcp::frame_chunk(0);
}
