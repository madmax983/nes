plan = """1. *Fix CI failure.*
   - If `timeline.reconstruct(target_frame)` returns `None`, `WorkerMsg::Reconstruct` sends NOTHING back!
   - `wait_for_sync` is expecting a `Reconstructed` reply. If it gets nothing, it times out after 500ms, sends another `Reconstruct`, waits another 500ms, and eventually times out and panics.
   - Why would `timeline.reconstruct(target_frame)` return `None`?
     `timeline.reconstruct(target_frame)` searches for the snapshot.
     `timeline.push(frame_id, *snapshot);` adds it.
   - Why would it fail to reconstruct a frame it JUST pushed and reconstructed 50ms ago?
   - Wait! `core.ppu_frame_counter()` is used as the `frame_id`.
     In `rewind_exhausted_state_on_timeout`, we do:
     ```rust
     for _ in 0..5 {
         core.execute(Command::StepFrame).unwrap();
         tm.record_frame(&core);
     }
     ```
     `core.ppu_frame_counter()` increments per frame. After 5 steps, it is 5.
   - The first `wait_for_sync(&mut tm)` sends `Reconstruct(5)`. The worker reconstructs it and returns `Reconstructed`. The test succeeds.
   - Then the test sleeps 50ms.
   - Then it calls `wait_for_sync(&mut tm)` AGAIN!
   - This sends `Reconstruct(5)`. The worker reconstructs it and returns `Reconstructed`. The test succeeds?
   - Then why did it panic on CI?
   - Wait, `tm.tx.try_send(WorkerMsg::Record { ... })` drops the message if the queue is full!
   - `let (work_tx, work_rx) = mpsc::sync_channel::<WorkerMsg>(4);`
   - THE QUEUE SIZE IS 4!
   - We did a `for _ in 0..5` loop. That's 5 messages!
   - `core.execute` and `tm.record_frame` are very fast. The worker thread might not wake up in time to process the first message.
   - So the 5th message (frame 5) gets DROPPED because the queue is full!
   - Then `wait_for_sync` sends `Reconstruct(5)`. Wait, `wait_for_sync` uses `tm.tx.send` which BLOCKS until there's space!
   - If message 5 was dropped, the worker only has frames 1..4.
   - When `wait_for_sync` asks for `Reconstruct(5)`, `timeline.reconstruct(5)` returns `None`!
   - So the worker thread NEVER REPLIES!
   - And `wait_for_sync` times out and panics!
   - This is absolutely a race condition! On Windows CI, the worker thread was scheduled too late, so `work_tx` filled up, the 5th frame was dropped, and `wait_for_sync` timed out waiting for a frame that was never recorded!
   - Oh my god, that's it!
"""
print(plan)
