plan = """1. *Fix CI failure.*
   - In `nes-rewind`, `wait_for_sync` timed out. The test is `rewind_exhausted_state_on_timeout`.
   - Why did it timeout? It says "Worker thread failed to sync". `wait_for_sync` waits for 5 seconds. Is 5 seconds not enough?
   - Wait, `rewind_exhausted_state_on_timeout` is a specific test. Let's look at `crates/nes-rewind/src/worker.rs:445,465`.
"""
print(plan)
