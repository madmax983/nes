use nes_mcp::publish_frame_with;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "timeout")]
fn havoc_test_mutex_poison_deadlock_frame() {
    let (tx, rx) = mpsc::channel();

    let _t = thread::spawn(move || {
        let _ = std::panic::catch_unwind(|| {
            publish_frame_with(256, 240, |_| {
                let _ = nes_mcp::frame_chunk(0);
            });
        });
        tx.send(()).unwrap();
    });

    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}
