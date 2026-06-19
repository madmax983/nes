use nes_mcp::{audio_chunk, publish_audio_with};
use std::thread;

#[test]
#[should_panic(expected = "output state lock")]
fn havoc_test_mutex_poison() {
    let t = thread::spawn(|| {
        publish_audio_with(256, |_| {
            panic!("Havoc closure panic");
        });
    });

    let _ = t.join();

    let _ = audio_chunk(0);
}
