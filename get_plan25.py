plan = """1. *Fix CI failure.*
   - If the worker thread panics during the first `wait_for_sync`, it could be because `wait_for_sync` drops the `WorkerReply` queue while the worker is trying to send to it?
   - Wait, `WorkerReply` is sent over `reply_tx` (which is `tm.rx` on the client side). The channel is created with `mpsc::channel()` or `sync_channel()`?
   - In `worker.rs:34`:
     ```rust
     let (reply_tx, reply_rx) = std::sync::mpsc::channel();
     let (work_tx, work_rx) = std::sync::mpsc::sync_channel(32);
     ```
   - The `reply_tx` is unbound (`channel`). So sending to it never blocks and never panics unless the receiver is dropped.
   - The `work_tx` is bounded (32). `tm.tx` is `work_tx`.
   - `tm.rx` is never dropped until `tm` is dropped.
   - Wait... if the first `wait_for_sync` is passed, `Reconstruct(5)` was successfully processed!
   - Then `std::thread::sleep(50ms)`
   - Then `wait_for_sync` is called AGAIN!
   - It sends `Reconstruct(5)` AGAIN.
   - Is there a bug in the worker thread when it receives `Reconstruct(5)` for a frame it already reconstructed?
   - Let's check the worker's `Reconstruct` handler in `crates/nes-rewind/src/worker.rs`.
"""
print(plan)
