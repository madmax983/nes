use nes_mcp::{publish_frame_with, frame_chunk};
use std::thread;

#[test]
#[ignore]
fn havoc_test_poisoned_mutex_on_panic() {
    let t1 = thread::spawn(|| {
        publish_frame_with(256, 240, |_| {
            panic!("Havoc closure panic");
        });
    });

    let _ = t1.join();

    // This will panic with "output state lock" because the mutex is poisoned
    let _ = frame_chunk(0);
}
