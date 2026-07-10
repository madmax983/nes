1. **Remove unwrap from `Ppu::apply_due_live_chr_updates` and `Ppu::apply_due_live_bg_updates`**:
   - In `crates/nes-core/src/ppu.rs`, the methods `apply_due_live_chr_updates` and `apply_due_live_bg_updates` use `.unwrap()` when popping from `pending_live_chr_updates` and `pending_live_bg_updates`, respectively.
   - I will replace the `unwrap()` with a safe extraction using a `while let Some(update) = self.pending_live_chr_updates.pop_front()` pattern (or similar logic maintaining the time check).

2. **Add Tests for Empty Queue (Doc test style / unit test style)**:
   - I will add targeted unit tests ensuring these methods do not panic when called with an empty queue, adhering to Sentry's philosophy that `.unwrap()` is a risk and untested logic is broken.

3. **Run tests and coverage**:
   - `cargo test --workspace --all-features` to ensure no regressions.
   - `cargo llvm-cov -p nes-core --show-missing-lines` to ensure coverage on this module has increased or not decreased.

Complete pre-commit steps
