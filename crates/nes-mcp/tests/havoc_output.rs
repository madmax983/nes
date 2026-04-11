use nes_mcp::{frame_chunk, publish_frame_with};
use std::thread;

#[test]
#[should_panic(expected = "output state lock")]
fn havoc_test_poisoned_mutex_on_panic() {
    // If the closure passed to `publish_frame_with` panics, the Mutex holding
    // the global output state becomes poisoned.
    // Any subsequent access (like `frame_chunk`) will `expect` on the poisoned lock
    // and crash the process.

    let t = thread::spawn(|| {
        publish_frame_with(256, 240, |_| {
            panic!("Havoc closure panic");
        });
    });

    let _ = t.join(); // Ignore the thread's panic

    // Detonate: Try to access the state again. This should panic because the lock is poisoned.
    let _ = frame_chunk(0);
}
