1. **Optimize `VecDeque::new()` to `VecDeque::with_capacity(8)` in `nes-core/src/ppu.rs`**
   - The files have already been updated using `python3` search and replace. Verify the changes using `git diff HEAD~1` or `cat crates/nes-core/src/ppu.rs`.
2. **Verify Changes**
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` using `run_in_bash_session`.
3. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Submit PR**
   - Run `git status` to verify the active branch.
   - Submit a PR titled "⚡ Bolt: [pre-allocate PPU update queues]" detailing the bottleneck, optimization, and impact:
     - 💡 What: Initialize `pending_live_chr_updates` and `pending_live_bg_updates` with `VecDeque::with_capacity(8)`.
     - 🎯 Why: `VecDeque::new()` initializes these vectors with no capacity, forcing heap allocations when frame updates are pushed during rendering.
     - 📊 Impact: Eliminates unneccessary small heap allocations during frame rendering.
     - 🔬 Measurement: Observe that tests and builds pass cleanly.
