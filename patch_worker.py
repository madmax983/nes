import re

content = open("crates/nes-rewind/src/worker.rs").read()

search = """            let _ = tm.tx.send(WorkerMsg::Reconstruct {
                target_frame: tm.last_recorded_frame,
            });
            if let Ok(WorkerReply::Reconstructed { frame_id, .. }) =
                tm.rx.recv_timeout(Duration::from_millis(100))
            {
                // The test core isn't mutated until `tm.rewind_step` does it.
                // But we just sent a raw message.
                // Since we received the reply, the worker is caught up.
                if frame_id == tm.last_recorded_frame {
                    return true;
                }
            }
        }
        false"""

replace = """            let _ = tm.tx.send(WorkerMsg::Reconstruct {
                target_frame: tm.last_recorded_frame,
            });
            if let Ok(WorkerReply::Reconstructed { frame_id, .. }) =
                tm.rx.recv_timeout(Duration::from_millis(500))
            {
                // The test core isn't mutated until `tm.rewind_step` does it.
                // But we just sent a raw message.
                // Since we received the reply, the worker is caught up.
                if frame_id == tm.last_recorded_frame {
                    return true;
                }
            }
            // Drain the queue to prevent filling it with stale `Reconstruct` requests
            while let Ok(_) = tm.rx.try_recv() {}
        }
        false"""

if search in content:
    content = content.replace(search, replace)
    open("crates/nes-rewind/src/worker.rs", "w").write(content)
    print("Replaced!")
else:
    print("Not found!")
