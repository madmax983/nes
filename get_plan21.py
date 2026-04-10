plan = """1. *Fix CI failure.*
   - Let's check `crates/nes-rewind/src/worker.rs:454`. It is the `assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");` call right after the `for _ in 0..5` loop.
   - Wait, `wait_for_sync` is at line 278. Let's look at `rewind_exhausted_state_on_timeout` at line 445:
   ```rust
    #[test]
    fn rewind_exhausted_state_on_timeout() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // Record a few frames so that we have a target frame > 0
        for _ in 0..5 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
   ```
   - In `wait_for_sync`:
   ```rust
            let _ = tm.tx.send(WorkerMsg::Reconstruct {
                target_frame: tm.last_recorded_frame,
            });
   ```
   - Wait, `tm.tx.send` might be failing. `tm.tx` is a `SyncSender`. If the queue is full and `send` blocks, or if the worker thread is dead, `send` returns an error. If `send` fails, `wait_for_sync` just ignores it `let _ = ...` and then waits.
   - If the worker thread panics during the `for` loop, `send` returns `Err` and `recv_timeout` returns `Disconnected`. It loops until the 5s timeout, then returns `false` causing the assert to panic.
   - Why would the worker thread panic?
   - Look at the panic log from CI:
   ```
   thread 'worker::tests::rewind_exhausted_state_on_timeout' (6628) panicked at crates\nes-rewind\src\worker.rs:454:9:
   Worker thread failed to sync
   ```
   - Wait, I see `2: <core::iter::adapters::step_by::StepBy<I> as core::iter::traits::iterator::Iterator>::next` in the backtrace for the worker panic? No, that's just the test thread panicking on `assert!`. The worker thread panic isn't in the log, OR it didn't panic.
   - If it didn't panic, maybe it's just stuck? Or the queue is full?
   - Wait, could it be that `tm.last_recorded_frame` is 0? `record_frame` increments it if the core steps.
   - Wait, the issue might be that `wait_for_sync` is sending multiple `WorkerMsg::Reconstruct` requests! If the worker is slow on Windows, `wait_for_sync` loops:
     - `send(Reconstruct)`
     - `while 500ms { recv_timeout(100ms) }`
     - Loop around and `send(Reconstruct)` again!
   - This can queue up multiple `Reconstruct` requests in `tm.tx`. The worker processes them sequentially.
   - If `tm.tx` is full (size is 32), `send` will block the main thread!
   - Wait, `tm.tx.send` is a synchronous send. But wait, `let _ = tm.tx.send(...)` might block if the queue is full!
   - But wait, `tm.tx.try_send(...)` should be used instead so it doesn't block!
   - Let's check `try_send` vs `send`.
"""
print(plan)
