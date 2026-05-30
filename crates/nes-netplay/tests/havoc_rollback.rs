use nes_core::NesCore;
use nes_netplay::{RollbackConfig, RollbackEngine};

#[test]
#[should_panic(expected = "RollbackWindowExceeded")]
#[ignore = "Havoc Deadlock/Crash"]
fn havoc_test_rollback_window_exceeded() {
    let mut core = NesCore::new();
    let config = RollbackConfig {
        local_player: 1,
        input_delay_frames: 1,
        max_rollback_frames: 5,
    };
    let mut engine = RollbackEngine::new(config).unwrap();

    // Advance 10 frames
    for _ in 0..10 {
        let _ = engine.advance_frame(&mut core);
    }

    // Attempt to inject an input 6 frames ago, which exceeds max_rollback_frames (5).
    // This will trigger a rollback on the next frame advance.
    let _ = engine.ingest_remote_input(3, 0xFF);

    // The rollback will execute and return an error. A crash happens if we unwrapped.
    // The test is simulating a scenario where the application unwraps.
    let _ = engine.advance_frame(&mut core).unwrap();
}
