plan = """1. *Fix CI failure.*
   - The test `rewind_exhausted_state_on_timeout` fails randomly because `work_tx` is a `sync_channel(4)` and the test sends 5 frames without waiting for the worker thread to catch up.
   - To fix this reliably, we just need to wait for sync after every single frame or increase the channel capacity, but waiting for sync makes more sense. Alternatively, just send 4 frames instead of 5, or do `wait_for_sync` *inside* the loop.
   - Let's change the loop in `rewind_exhausted_state_on_timeout` to:
     ```rust
     for _ in 0..5 {
         core.execute(Command::StepFrame).unwrap();
         tm.record_frame(&core);
         assert!(wait_for_sync(&mut tm), "Worker thread failed to sync on record");
     }
     ```
   - Actually, a better fix is `wait_for_sync` after each frame for *all* tests that record multiple frames.
   - Wait, `record_and_rewind_restores_earlier_frame` has:
     ```rust
        // Fast forward 100 frames to build history
        for _ in 0..100 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
     ```
   - If that's the case, why didn't `record_and_rewind_restores_earlier_frame` fail on Windows? It dropped up to 96 frames! But it only cares about ONE earlier frame. Wait, `wait_for_sync` asserts `reconstruct(100)`! If frame 100 was dropped, `wait_for_sync` would timeout and panic.
   - Then why didn't `record_and_rewind_restores_earlier_frame` fail on CI? Maybe it did, or maybe it usually completes before timing out?
   - Actually, wait, `tm.tx.try_send` drops messages. BUT in tests we should use `send`? No, `record_frame` is hardcoded to `try_send`.
   - If `record_frame` drops frames, `tm.last_recorded_frame` is still updated to 100.
   - When `wait_for_sync(&mut tm)` asks for frame 100, if frame 100 was dropped, it will timeout and panic!
   - So ANY test that records more than 4 frames in a tight loop is flaky!
   - To fix this, we should add a `sync_step` helper to the tests:
     ```rust
     fn sync_step(tm: &mut TimeMachine, core: &mut NesCore) {
         core.execute(Command::StepFrame).unwrap();
         tm.record_frame(core);
         assert!(wait_for_sync(tm), "Worker thread failed to sync");
     }
     ```
     No, `wait_for_sync` sleeps and polls for 5 seconds. Doing it 100 times would take a long time, but it's fast if the worker catches up quickly.
   - Alternatively, we can just replace `for _ in 0..100 { ... }` with a loop that does `wait_for_sync` periodically, or `std::thread::yield_now()`, or just wait after every frame.
   - In `crates/nes-rewind/src/worker.rs`, the `wait_for_sync` uses `try_recv()` and `send()`.
   - Let's use `replace_with_git_merge_diff` to fix the test loops in `crates/nes-rewind/src/worker.rs`.
"""
print(plan)
