1. **Increase coverage in `cnrom.rs`:**
    - The `prg_offset_for` logic has uncovered paths, specifically the `if self.prg_rom.is_empty()` check. This can be resolved by padding PRG in `from_prg_chr` similar to `gxrom.rs`, avoiding the check entirely, or directly testing an empty prg initialized mapper. I will just test the empty prg case using `Cnrom::from_prg_chr(vec![], vec![])` and querying `prg_offset_for`.

2. **Increase coverage in `mmc3.rs`:**
    - The `irq_clock_dot` method has an untested branch handling 8x16 sprite timing (returning `Some(260)`), as well as a fallback when neither BG nor Sprites use the high table (returning `None`).
    - I will add targeted unit tests under `#[cfg(test)] mod tests` in `crates/nes-core/src/mapper/mmc3.rs` to exercise these specific configurations of `irq_clock_dot` and verify the `write_prg` edge cases (e.g. `0xA001`).

3. **Increase coverage in `api.rs` (Controller/Speed logic):**
    - The internal controller port reading (`consume_controller_read`), shifting logic, and speed setters are under-tested.
    - I will add a `mod tests` in `api.rs` to test `NesCore` controller strobe logic (latching and shifting out button presses sequentially) and `SetSpeed` error paths.
    - I will ensure we write values to the CPU bus `write_cpu_bus(0x4016, ...)` and then verify the read side effects using `apply_cpu_read_side_effect`.

4. **Complete pre-commit steps:**
    - I will run all standard quality commands (e.g. `cargo clippy`, `cargo test`, `cargo fmt`).
