## 2026-02-28 - [Initial Audit]
**Learning:** The MMC1 mapper logic has significant coverage gaps around bank switching modes and shift register commits.
**Action:** Add comprehensive unit tests in crates/nes-core/src/mapper/mmc1.rs to cover these edge cases.
## 2026-03-01 - [MMC3 Edge Case Coverage]
**Learning:** The MMC3 mapper logic in `crates/nes-core/src/mapper/mmc3.rs` had missing coverage for scanline IRQ conditions (e.g., dot matching, scanline matching, rendering disabled) and PRG writes below `0x8000`, as well as PRG RAM protection.
**Action:** Added targeted integration tests in `crates/nes-core/tests/mapper_mmc3.rs` that explicitly check each condition (e.g., calling `on_ppu_dot` with different scanlines/dots/rendering states, and writing to `0x7FFF`).
## 2026-03-12 - [LoadedMapper Coverage Improvements]
**Learning:** `LoadedMapper` in `crates/nes-core/src/api.rs` is an internal enum wrapping various mapper variants. Many of its methods (`chr_window`, `mirroring_override`, `irq_pending`, etc.) were missing unit test coverage, which allowed mutations to survive.
**Action:** Wrote explicit unit tests instantiating mapper implementations directly (`Mmc3::from_prg_chr`, `Axrom::from_prg_rom`, etc.) and wrapping them in `LoadedMapper` variants to assert execution paths, reducing mutation survivability and ensuring correct integration behavior.
## 2026-03-12 - [GxROM Coverage Improvements]
**Learning:** `sync_chr_ram_from_ppu_window` in `crates/nes-core/src/mapper/gxrom.rs` was missing unit tests covering whether the PPU window data was correctly synced to the CHR array when `chr_writable` is true or false.
**Action:** Wrote explicit unit tests instantiating `Gxrom` with empty and non-empty `chr_rom` to cover the true and false branch of `chr_writable` and asserting the correct CHR syncing behavior.## 2026-03-26 - [RollbackEngine Clear History]
**Learning:** `RollbackEngine::clear_from` in `crates/nes-netplay/src/rollback.rs` lacked specific test coverage, making mutations to its clearing logic (e.g. replacing it with `()`) survivable. This logic is crucial for ensuring that predicted future states are correctly purged upon a network desync and rollback.
**Action:** Wrote an explicit unit test `rollback_engine_clears_from_frame_on_rollback` that manually constructs a rollback engine's internal buffers and directly asserts that `clear_from` drops future state arrays while preserving history before the target frame.
## 2026-03-29 - [API Core Command and Query Gaps]
**Learning:** `nes-core/src/api.rs` had coverage gaps around `Command::ReleaseButton`, `Command::PowerCycle`, invalid `Command::SetSpeed(0)`, and querying `CoreQuery` variants. Because these are critical pathways for client applications to control the emulator, untested behavior here could result in unpredictable frontend state.
**Action:** Wrote explicit unit tests in `api.rs` asserting correct behavior of button releases clearing bitmasks, power cycling resetting speeds, zero speed returning an error, and all `CoreQuery` variants correctly matching their results. Removed trivial getters/setters tests as per strict "Sentry" policy.
## 2024-05-19 - Mutant testing boolean logic operators

**Learning:** When using `cargo mutants`, testing `||` conditions by asserting true/false isn't enough; you need to write specific tests that hit each side of the `||` independently to kill mutations that replace `||` with `&&`.
**Action:** Always create separate test cases for each independent condition within an `||` statement instead of grouping them together.
## 2024-05-19 - [API command execution edge cases]
**Learning:** Certain `NesCore::execute` paths like clearing button presses natively, or executing `Command::StepCpu` and hitting the `service_irq()` condition at the end of the step loop weren't covered. Similarly, tests for checking if `chr_writable` applies to GxROM and UxROM padding logic were missing.
**Action:** Wrote explicit tests simulating execution of specific controller combinations and forcing IRQ interrupts in `command_query_contract.rs`. Also added mapper tests to verify `chr_writable` behavior on `Gxrom` and padding rules in `Uxrom`. The experimental `SpriteExtractor` missing match coverage on `color_idx` was fixed by mocking PPU state.
