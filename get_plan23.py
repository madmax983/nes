plan = """1. *Fix CI failure.*
   - The test `rewind_exhausted_state_on_timeout` fails at `assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");` at line 454 (the first wait_for_sync).
   - The `wait_for_sync` method blocks up to 5 seconds waiting for the `Reconstruct` reply.
   - If the worker thread takes a while, maybe the first 5 frames are taking long to process?
   - Wait, `core.execute(Command::StepFrame)` steps the core, then `tm.record_frame(&core)` sends a delta.
   - For `for _ in 0..5`, it generates 5 deltas. It should be very fast.
   - Why would it fail to sync *only* on Windows, and *only* randomly?
   - Oh, I see in the panic backtrace:
     `2: <core::iter::adapters::step_by::StepBy<I> as core::iter::traits::iterator::Iterator>::next`
     `3: nes_rewind::worker::tests::rewind_exhausted_state_on_timeout::{{closure}}`
   - Wait, `core::iter::adapters::step_by::StepBy`? There is no `step_by` in `rewind_exhausted_state_on_timeout`. Wait...
   - Oh! `nes-rewind` has `delta.rs` which uses `step_by`?
   - If the worker thread panicked during `record_frame`, it would panic inside `delta.rs`.
   - The panic backtrace I saw earlier:
     ```
     thread 'worker::tests::rewind_exhausted_state_on_timeout' (6628) panicked at crates\nes-rewind\src\worker.rs:454:9:
     Worker thread failed to sync
     stack backtrace:
        0: std::panicking::panic_handler
        1: core::panicking::panic_fmt
        2: <core::iter::adapters::step_by::StepBy<I> as core::iter::traits::iterator::Iterator>::next
        3: nes_rewind::worker::tests::rewind_exhausted_state_on_timeout::{{closure}}
     ```
   - No, `assert!` calls `panic_fmt`. Then `<core::iter::adapters::step_by::StepBy...>` is in the backtrace? Wait, `std::thread::sleep` or something?
   - No, that's just a bizarre backtrace artifact on Windows or a compiler artifact for the `assert!` macro or something. The panic is literally `Worker thread failed to sync` which is the text of the `assert!`.
   - Why does `wait_for_sync` fail?
   - In `wait_for_sync`:
     `while tm.rx.try_recv().is_ok() {}`
     `let start = Instant::now();`
     `while start.elapsed() < Duration::from_millis(5000) { ... }`
   - Wait! `try_recv()` might drop the *reply* to a previous `Reconstruct`! But there was no previous `Reconstruct` before the first `wait_for_sync` call.
   - However, `record_frame` also sends replies? No, `WorkerReply` is only sent for `Reconstruct` and `FastForward`.
   - Wait! `record_frame` doesn't return anything. But `WorkerReply::RecordingFailed` might be sent!
   - If `tm.rx.try_recv()` drops a `WorkerReply::RecordingFailed`, we'd never know it failed!
   - And the worker thread might have terminated if the channel disconnected.
   - Wait, `wait_for_sync` sends `WorkerMsg::Reconstruct` and waits for `WorkerReply::Reconstructed`.
   - But if `tm.rx.recv_timeout(100ms)` receives `WorkerReply::RecordingFailed`, it just ignores it because it only matches `if let Ok(WorkerReply::Reconstructed { ... }) = ...`.
   - So if the worker replied with an error or something else, `wait_for_sync` drops it and loops again, eventually timing out.
   - Or maybe the worker died, so `recv_timeout` returns `Disconnected`. Then it loops again until 5 seconds!
   - Yes! If the worker thread dies, `wait_for_sync` will just loop for 5 seconds and then return `false`.
"""
print(plan)
