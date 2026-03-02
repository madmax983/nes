import re

with open("crates/nes-netplay/src/rollback.rs", "r") as f:
    data = f.read()

data = re.sub(r'#\[cfg\(test\)\]\s*mod proptests \{.*', '', data, flags=re.DOTALL)

with open("crates/nes-netplay/src/rollback.rs", "w") as f:
    f.write(data)

with open("crates/nes-netplay/src/rollback.rs", "a") as f:
    f.write("""
#[cfg(test)]
mod proptests {
    use super::*;
    use nes_core::NesCore;
    use proptest::prelude::*;
    use crate::rollback::tests::make_core_for_rollback;

    proptest! {
        #[test]
        fn fuzz_rollback_engine(
            local_inputs in proptest::collection::vec(0..255u8, 10..50),
            remote_inputs in proptest::collection::vec(0..255u8, 10..50),
            delay_frames in 0..10u32,
            max_rollback in 1..20u32,
            random_seed in 0..100u32,
        ) {
            let mut core = make_core_for_rollback();
            let config = RollbackConfig {
                local_player: 1,
                input_delay_frames: delay_frames.min(max_rollback),
                max_rollback_frames: max_rollback,
            };
            let mut engine = RollbackEngine::new(config).unwrap();

            let mut frame = 0;
            for (local, remote) in local_inputs.iter().zip(remote_inputs.iter()) {
                engine.schedule_local_input(*local);

                // Add noise to the inputs to fuzz remote inputs behavior
                if random_seed % 3 == 0 {
                    engine.ingest_remote_input(frame, *remote);
                } else if random_seed % 3 == 1 {
                    engine.ingest_remote_input(frame.saturating_add(1), *remote);
                } else {
                    engine.ingest_remote_input(frame.saturating_sub(1), *remote);
                }

                // Call advance frame and ignore potential errors because we intentionally throw bad inputs
                let _ = engine.advance_frame(&mut core);
                frame += 1;
            }
        }
    }
}
""")
