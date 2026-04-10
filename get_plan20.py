plan = """1. *Fix CI failure.*
   - The test `rewind_exhausted_state_on_timeout` panicked at line 454: `assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");` which is the FIRST `wait_for_sync` call.
   - Wait, `core.execute(Command::StepFrame)` steps the core, then `tm.record_frame(&core)` records the frame.
   - The `wait_for_sync` method drains the queue: `while tm.rx.try_recv().is_ok() {}` and then sends `WorkerMsg::Reconstruct` and waits up to 5000ms.
   - Why would the worker thread fail to reconstruct frame 5 on Windows CI?
   - Wait, `wait_for_sync` sends a request, then loops for 500ms waiting for a response, then sends *another* request. But if `tm.rx` is drained, maybe it's dropping the response to the *first* request if the worker hasn't processed it yet?
   - No, `while tm.rx.try_recv().is_ok() {}` is outside the 5000ms polling loop.
   - Inside the polling loop:
     ```rust
     let _ = tm.tx.send(WorkerMsg::Reconstruct { target_frame: tm.last_recorded_frame });
     let inner_start = std::time::Instant::now();
     while inner_start.elapsed() < Duration::from_millis(500) {
         if let Ok(WorkerReply::Reconstructed { frame_id, .. }) = tm.rx.recv_timeout(Duration::from_millis(100)) {
             if frame_id == tm.last_recorded_frame { return true; }
         }
     }
     ```
   - If the worker takes > 100ms, `recv_timeout` returns `Err(RecvTimeoutError::Timeout)`. The `if let Ok(...)` fails, and we loop again until 500ms. If it replies at 200ms, the next iteration of the inner loop will catch it. This seems fine.
   - Wait, if the worker is still processing `record_frame` (from the `for` loop), the `Reconstruct` message gets queued. The worker will eventually reply.
   - Wait, what if the `record_frame` messages fill the bounded queue? The worker uses a bounded queue: `std::sync::mpsc::sync_channel(16)` for `tx`, wait, `WorkerMsg` is sent to `tm.tx`. The `TimeMachine` uses a `std::sync::mpsc::sync_channel(32)` or something for `tx`.
   - Is it possible the worker panicked? If the worker panicked, `tm.tx.send` would return an error, and `tm.rx.recv_timeout` would return `RecvTimeoutError::Disconnected` or `Timeout`. It would then loop until 5 seconds.
   - Why did the worker panic? Or did it just freeze?
   - Let's check if the worker thread panics on Windows. We can use `--nocapture` to see. Wait, CI doesn't show standard output if it panicked inside a thread unless we join.
"""
print(plan)
