plan = """1. *Fix CI failure.*
   - Let's check `worker.rs:445,465` again. The test replaces `tm.rx` and `tm.tx` with a dummy channel:
     ```rust
     let (_dummy_reply_tx, dummy_rx) = std::sync::mpsc::sync_channel(1);
     let (dummy_work_tx, _dummy_work_rx) = std::sync::mpsc::sync_channel(1);

     tm.rx = dummy_rx;
     let old_tx = std::mem::replace(&mut tm.tx, dummy_work_tx);
     ```
   - When this happens, the old `tm.rx` and `tm.tx` are dropped. Wait, `old_tx` is kept alive until the end of the test.
   - BUT the original `tm.rx` is OVERWRITTEN! `tm.rx = dummy_rx`. The original receiver is DROPPED.
   - When `tm.rx` is dropped, the worker thread might panic because it tries to send `WorkerReply` but the receiver is disconnected!
   - Wait! `worker.rs` line 445 is before the mock channels are created!
     `assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");` is BEFORE the dummy channels!
   - Wait, `wait_for_sync(&mut tm)` fails BEFORE the receiver is replaced!
   - Oh, look:
     ```rust
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        // Let's actually give the worker a moment to process the queue
        // to avoid race conditions.
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");
     ```
   - The panic might be on the SECOND `wait_for_sync`!
   - Ah! "Worker thread failed to sync". Both asserts have the same string! So it could be the second one!
   - If it is the second one, why did it fail?
   - The first `wait_for_sync` succeeds, so the worker catches up to frame 5.
   - Then the test sleeps 50ms.
   - Then it calls `wait_for_sync` again.
   - Wait, `wait_for_sync` drains `tm.rx`. Then it sends `Reconstruct(5)`.
   - If the worker receives `Reconstruct(5)`, it replies `Reconstructed(5)`.
   - But wait, `wait_for_sync` has a bug:
     ```rust
     if tm.last_recorded_frame == 0 {
         return true;
     }
     ```
     This early return only happens if `last_recorded_frame` is 0. Since it's 5, it proceeds.
   - Wait, if the worker is ALREADY synced, `wait_for_sync` sends `Reconstruct(5)`. The worker thread processes it and replies `Reconstructed(5)`.
   - `wait_for_sync` reads `WorkerReply::Reconstructed` from `tm.rx.recv_timeout(100)`. It checks if `frame_id == 5`. It is 5. So it returns `true`.
   - Why would it fail?
   - Wait, does `wait_for_sync` drain the queue properly?
     ```rust
     while tm.rx.try_recv().is_ok() {}
     ```
     This drains any *existing* replies.
   - Then it loops sending `Reconstruct` every 500ms?
     Wait, the inner loop:
     ```rust
     let inner_start = std::time::Instant::now();
     while inner_start.elapsed() < Duration::from_millis(500) {
         if let Ok(WorkerReply::Reconstructed { frame_id, .. }) =
             tm.rx.recv_timeout(Duration::from_millis(100))
         {
             if frame_id == tm.last_recorded_frame {
                 return true;
             }
         }
     }
     ```
     This loops up to 5 times (500 / 100), but if `recv_timeout` gets an error (like `Timeout`), it goes to the next iteration of the inner loop immediately?
     Wait! If `recv_timeout` times out, it takes 100ms. So it loops 5 times.
     If `recv_timeout` gets an `Ok` but it's a different message (e.g. `RecordingFailed`), the `if let` fails, and it goes to the next iteration IMMEDIATELY! Without waiting 100ms!
     It then calls `recv_timeout` again, which blocks for 100ms.
     Wait, if `tm.rx` is CLOSED, `recv_timeout` returns `Err(RecvTimeoutError::Disconnected)` IMMEDIATELY!
     Then the inner loop spins extremely fast for 500ms!
     Then it goes to the outer loop, sends another `Reconstruct` (which fails and is ignored), and spins again for 500ms. It will do this until 5 seconds pass.
     So if the worker thread panicked or disconnected, `wait_for_sync` will spin for 5 seconds and then return `false`.
"""
print(plan)
