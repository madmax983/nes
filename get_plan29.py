plan = """1. *Fix CI failure.*
   - If tests loop `for _ in 0..60` they send 60 messages to a channel of capacity 4! Since `record_frame` uses `try_send`, 56 of these messages are DROPPED.
   - The final `wait_for_sync` requests `Reconstruct(60)`. BUT frame 60 is dropped!
   - So `wait_for_sync` will timeout.
   - Why didn't it ALWAYS fail locally? Because maybe the worker thread was fast enough to drain some messages, but not 60 messages. Wait, if the worker thread is fast, it might process frame 60, but on a slow CI machine it drops it.
   - We must do `assert!(wait_for_sync(&mut tm))` *inside* the loops for the tests to be deterministic, OR we change the test to `std::thread::yield_now()` or similar, OR we change the test capacity.
   - The easiest fix is to put `wait_for_sync` in the recording loop in tests, but it might be slow.
   - Let's check how many tests have a loop:
     - `record_and_rewind_restores_earlier_frame`: `for _ in 0..60`
     - `resume_after_rewind_puts_machine_back_in_recording_state`: `for _ in 0..10`
     - `rewind_faster_accelerates_speed_and_clamps`: `for _ in 0..100`
     - `history_seconds_returns_expected_value`: `for _ in 0..120`
     - `rewind_exhausted_state_on_timeout`: `for _ in 0..5`
   - Doing `wait_for_sync` 100 times in a test might take up to 100 * 5ms = 0.5s, which is fine for tests.
   - Let's write a helper in `tests` module:
     ```rust
     fn record_and_sync(tm: &mut TimeMachine, core: &mut NesCore, count: u64) {
         for _ in 0..count {
             core.execute(Command::StepFrame).unwrap();
             tm.record_frame(core);
             assert!(wait_for_sync(tm), "Worker thread failed to sync on record");
         }
     }
     ```
     Wait, `wait_for_sync` will send `Reconstruct` for EVERY frame. Is that ok? Yes, it's just a test.
"""
print(plan)
