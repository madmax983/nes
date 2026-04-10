plan = """1. *Fix CI failure in `nes-rewind`.*
   - A completely unrelated test failed: `worker::tests::rewind_exhausted_state_on_timeout` panicked at `crates/nes-rewind/src/worker.rs:454`.
   - The error is "Worker thread failed to sync". This suggests a timeout wait failed, possibly due to CI being slow on Windows.
   - Let's check `crates/nes-rewind/src/worker.rs:454`.
"""
print(plan)
