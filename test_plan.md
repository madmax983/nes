1. **Document `nes-core/src/experimental/spatial_bot.rs`**
   - Add missing doc comments to `BotRule` and its fields.
   - Add missing doc comments to `SpatialBot` and `SpatialBot::new()`.
2. **Document `nes-core/src/experimental/ppu_visualizer.rs`**
   - Add missing doc comments to `PpuVisualizer`.
3. **Document `nes-test-harness/src/lib.rs` and modules**
   - Add missing doc comments to `ApuWriteEvent`, `AudioStats`, `WaveformComparison` and their fields.
   - Add missing doc comments to exported functions (`apu_write_hash`, `waveform_hash`, etc.).
   - Add missing doc comments to `homebrew::default_homebrew_rom_path` and `rom_paths` functions.
4. **Verify Compilation and Documentation**
   - Run `cargo test --doc --all-features`.
   - Run `RUSTDOCFLAGS="-D missing_docs" cargo doc --all-features --no-deps` to verify no warnings remain in `nes-core` and `nes-test-harness`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo fmt --all`.
5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Call `pre_commit_instructions` and follow steps.
6. **Submit PR**
   - Submit the documentation update as PR with title "🎸 Bard: [documentation update]".
