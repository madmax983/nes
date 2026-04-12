1. **Refactor `resolve_bits` in `crates/nes-netplay/src/rollback.rs`:**
   - The `resolve_bits` function suffers from "Boolean Blindness" (`local: bool`) and uses `if/else` logic to decide which maps to query. This should be extracted into two separate helper functions: `resolve_local_bits` and `resolve_remote_bits`.
   - Update `simulate_frame` to call the new helper functions instead of passing `true`/`false`.
2. **Complete pre commit steps:**
   - Run `pre_commit_instructions` to ensure proper testing, verification, review, and reflection are done.
3. **Submit the change.**
