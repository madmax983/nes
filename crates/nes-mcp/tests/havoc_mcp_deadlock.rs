use nes_mcp::{publish_frame_with, frame_chunk};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "output state lock")]
fn havoc_test_mutex_poison() {
    let (tx, rx) = mpsc::channel();

    let _t = thread::spawn(move || {
        let _ = std::panic::catch_unwind(|| {
            publish_frame_with(256, 240, |_| {
                panic!("Havoc closure panic");
            });
        });
        tx.send(()).unwrap();
    });

    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");

    let _ = frame_chunk(0);
}
