use nes_mcp::{frame_chunk, publish_frame_with};
use std::thread;

#[test]
#[should_panic(expected = "output state lock")]
fn havoc_test_mutex_poison() {
    let t = thread::spawn(|| {
        publish_frame_with(256, 240, |_| {
            panic!("Havoc closure panic");
        });
    });

    let _ = t.join();

    let _ = frame_chunk(0);
}
