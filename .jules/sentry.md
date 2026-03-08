## 2026-02-28 - [Initial Audit]\n**Learning:** The MMC1 mapper logic has significant coverage gaps around bank switching modes and shift register commits.\n**Action:** Add comprehensive unit tests in crates/nes-core/src/mapper/mmc1.rs to cover these edge cases.
## 2026-03-01 - [MMC3 Edge Case Coverage]
**Learning:** The MMC3 mapper logic in `crates/nes-core/src/mapper/mmc3.rs` had missing coverage for scanline IRQ conditions (e.g., dot matching, scanline matching, rendering disabled) and PRG writes below `0x8000`, as well as PRG RAM protection.
**Action:** Added targeted integration tests in `crates/nes-core/tests/mapper_mmc3.rs` that explicitly check each condition (e.g., calling `on_ppu_dot` with different scanlines/dots/rendering states, and writing to `0x7FFF`).
