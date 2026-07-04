use nes_mcp::{audio_chunk, publish_audio_with};
use std::thread;

#[test]
#[should_panic(expected = "output state lock")]
fn havoc_test_poisoned_mutex_on_audio_panic() {
    let t = thread::spawn(|| {
        publish_audio_with(256, |_| {
            panic!("Havoc closure panic");
        });
    });

    let _ = t.join(); // Ignore the thread's panic

    // Detonate: Try to access the state again. This should panic because the lock is poisoned.
    let _ = audio_chunk(0);
}
