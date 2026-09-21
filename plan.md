1. **Sentry Journal Update**
   - Create or append to `.jules/sentry.md` documenting the pattern of testing edge cases in state restoration (mismatched buffer lengths) and mapper initialization (unaligned/undersized ROMs).
2. **CNROM Coverage**
   - Add a test `cnrom_read_prg_returns_mapped_value` in `crates/nes-core/src/mapper/cnrom.rs` to cover `read_prg`.
3. **ColorDreams Coverage**
   - Add a test `colordreams_pads_undersized_prg_rom` in `crates/nes-core/src/mapper/colordreams.rs` to cover undersized PRG padding (line 39).
4. **FME-7 Coverage**
   - Add tests `fme7_pads_unaligned_prg_rom`, `fme7_pads_undersized_chr_rom`, and `fme7_state_restore_with_mismatched_wram_len_resizes` in `crates/nes-core/src/mapper/fme7.rs`.
5. **MMC4 Coverage**
   - Add tests `mmc4_pads_unaligned_prg_rom` and `mmc4_state_restore_with_mismatched_prg_ram_len_resizes` in `crates/nes-core/src/mapper/mmc4.rs`.
6. **Namco 108 Coverage**
   - Add test `namco108_pads_unaligned_prg_rom` in `crates/nes-core/src/mapper/namco108.rs`.
7. **Run Tests & Coverage**
   - Run `cargo test -p nes-core` to verify the tests pass.
   - Run `cargo llvm-cov report -p nes-core` to verify that coverage on those mappers is 100% or significantly improved.
8. **Pre-commit Steps**
   - Run pre-commit steps to ensure proper testing, verification, review, and reflection are done (e.g., `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`).
9. **Submit PR**
   - Create a PR with persona format: "🛡️ Sentry: [test coverage improvement]"
