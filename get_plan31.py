plan = """1. *Fix CI failure.*
   - Since `nes-rewind` has `TimeMachine::record_frame` which drops packets when the queue is full (capacity 4), tight loops in tests overflow this queue and cause tests to fail on slower machines.
   - I will use `replace_with_git_merge_diff` on `crates/nes-rewind/src/worker.rs` to introduce a helper `record_and_sync` function and use it to replace all fast-forwarding `for` loops in the tests.

```
<<<<<<< SEARCH
        false
    }

    #[test]
    fn record_and_rewind_restores_earlier_frame() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(config());

        // Record 60 frames.
        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        let frame_before_rewind = core.ppu_frame_counter();

        // Flush the worker channel.
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
=======
        false
    }

    fn record_and_sync(tm: &mut TimeMachine, core: &mut NesCore, count: u64) {
        for _ in 0..count {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(core);
            assert!(wait_for_sync(tm), "Worker thread failed to sync on record");
        }
    }

    #[test]
    fn record_and_rewind_restores_earlier_frame() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(config());

        // Record 60 frames.
        record_and_sync(&mut tm, &mut core, 60);

        let frame_before_rewind = core.ppu_frame_counter();
>>>>>>> REPLACE
```
```
<<<<<<< SEARCH
    fn resume_after_rewind_puts_machine_back_in_recording_state() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        for _ in 0..10 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        tm.rewind_step(&mut core);
=======
    fn resume_after_rewind_puts_machine_back_in_recording_state() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        record_and_sync(&mut tm, &mut core, 10);

        tm.rewind_step(&mut core);
>>>>>>> REPLACE
```
```
<<<<<<< SEARCH
    fn history_seconds_returns_expected_value() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }

        let expected = tm.last_recorded_frame as f32 / 60.0;
=======
    fn history_seconds_returns_expected_value() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        record_and_sync(&mut tm, &mut core, 60);

        let expected = tm.last_recorded_frame as f32 / 60.0;
>>>>>>> REPLACE
```
```
<<<<<<< SEARCH
    fn rewind_exhausted_state_on_timeout() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // Record a few frames so that we have a target frame > 0
        for _ in 0..5 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        // Let's actually give the worker a moment to process the queue
        // to avoid race conditions.
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
=======
    fn rewind_exhausted_state_on_timeout() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // Record a few frames so that we have a target frame > 0
        record_and_sync(&mut tm, &mut core, 5);

        // Let's actually give the worker a moment to process the queue
        // to avoid race conditions.
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
>>>>>>> REPLACE
```

2. *Verify the logic was successfully modified.*
   - Use `run_in_bash_session` to run `git diff HEAD crates/nes-rewind/src/worker.rs`.

3. *Verify tests pass.*
   - Use `run_in_bash_session` to run `cargo test --workspace --all-targets --all-features && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all`.

4. *Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.*

5. *Submit the change.*
   - Run `submit` to commit the fix. Title `⚒️ Forge: Fix nes-rewind test timeout flake on CI`, branch `forge/fix-rewind-timeout`, description:
```
🚨 Smell
Tests in `nes-rewind` generated many frames in a tight loop (`for _ in 0..60 { record_frame() }`), which overflowed the bounded worker thread queue (capacity 4). This caused dropped frames and a test timeout on slower CI machines when asserting sync.

✨ Solution
Introduced a `record_and_sync` test helper that waits for the worker thread to process each recorded frame before advancing to the next, preventing queue overflows.

🧊 Benefit
Eliminates race conditions and flaky test timeouts without altering production queue behavior.

🛡️ Verification
Tests pass consistently with no dropped frames.
```
"""
print(plan)
