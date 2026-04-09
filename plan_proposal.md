1. **Optimize PPU Update Queues (`crates/nes-core/src/ppu.rs`)**
    - The `pending_live_chr_updates` and `pending_live_bg_updates` fields in `Ppu` and `PpuSnapshot` are currently `Vec`s.
    - During the emulator's execution loop (specifically in `apply_due_live_chr_updates` and `apply_due_live_bg_updates`), elements are popped from the front of these vectors using `.remove(0)`.
    - This creates an unnecessary O(N) memory shift on the hot path.
    - I will replace `Vec<PendingLiveChrWindowUpdate>` and `Vec<PendingLiveBgStateUpdate>` with `VecDeque<...>` in both `Ppu` and `PpuSnapshot`.
    - I will import `std::collections::VecDeque` in `ppu.rs`.
    - I will update `.remove(0)` to `.pop_front().unwrap()` in both functions.
    - I will update `Vec::new()` to `VecDeque::new()` and `.push(...)` to `.push_back(...)` (where needed).
    - I will provide exact diff hunks via `replace_with_git_merge_diff` for all edits to `crates/nes-core/src/ppu.rs`.
    - **Note:** Because `PpuSnapshot` needs to remain serializable, `VecDeque` perfectly implements `Serialize`/`Deserialize` natively via `serde`, so no custom bounds are needed.
2. **Verify Changes via Git**
    - Use `run_in_bash_session` to run `git diff HEAD crates/nes-core/src/ppu.rs` to confirm the edits were applied correctly.
3. **Verify Changes via Cargo**
    - Run the full workspace verification suite to ensure everything builds, tests pass, and linting rules are followed: `cargo test --workspace --all-targets --all-features && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all`.
4. **Complete Pre-Commit Steps**
    - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Submit PR**
    - I will create a PR with title `⚡ Bolt: [Replace Vec with VecDeque on PPU update queues to eliminate O(N) shifts]`
    - The description will contain the exact sections required by Bolt ('💡 What', '🎯 Why', '📊 Impact', '🔭 Measurement').
