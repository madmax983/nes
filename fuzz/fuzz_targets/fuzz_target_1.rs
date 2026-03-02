#![no_main]

use libfuzzer_sys::fuzz_target;
use nes_netplay::{RollbackEngine, RollbackConfig};
use nes_core::{NesCore, Command};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let max_frames = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) % 60 + 1;
    let config = RollbackConfig {
        local_player: 1,
        input_delay_frames: 1,
        max_rollback_frames: max_frames,
    };
    if let Ok(mut engine) = RollbackEngine::new(config) {
        let mut core = NesCore::new();
        // Just ingest inputs to see if it panics
        for (i, byte) in data.iter().skip(4).take(20).enumerate() {
            let _ = engine.schedule_local_input(*byte);
            let _ = engine.ingest_remote_input(i as u64, *byte);
            let _ = engine.advance_frame(&mut core);
        }
    }
});
