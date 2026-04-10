plan = """1. *Fix CI failure.*
   - If tests loop `for _ in 0..60` they send 60 messages to a channel of capacity 4! Since `record_frame` uses `try_send`, 56 of these messages are DROPPED.
   - The test that failed is `rewind_exhausted_state_on_timeout`. Wait, it did `for _ in 0..5` and sent 5 messages. 5 > 4! The last message was dropped!
   - So `wait_for_sync` timed out waiting for `Reconstruct(5)`.
   - The fix is to add `record_and_sync` helper and replace the `for` loops in:
     - `record_and_rewind_restores_earlier_frame`
     - `resume_after_rewind_puts_machine_back_in_recording_state`
     - `history_seconds_returns_expected_value`
     - `rewind_exhausted_state_on_timeout`
   - Wait, `record_frame_does_nothing_when_not_recording` just does a single frame, so it's fine.
   - `rewind_faster_accelerates_speed_and_clamps` does two individual `record_frame` calls, so it's fine (2 < 4).
   - I will use `replace_with_git_merge_diff` to add the `record_and_sync` function inside `mod tests` and update the 4 test cases that loop.
   - Actually, a much cleaner fix is just `std::thread::yield_now()` or similar inside the loop, or `thread::sleep(Duration::from_millis(5))` to let the worker thread process. But `record_and_sync` guarantees no drops without arbitrary sleeps.
"""
print(plan)
