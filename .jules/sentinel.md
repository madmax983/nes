# Sentinel Journal

## YYYY-MM-DD - Missing Test Coverage for WebAssembly Bindings
**Mutant:** Many mutations in `crates/nes-web/src/lib.rs` involving public functions exposed to Wasm via `wasm_bindgen` survive because they are untested.
**Diagnosis:** The public API surface exposed by Wasm bindgen is currently untesed. The integration tests cover `WebRuntime` but do not go through the `NesWebEmulator` facade.
**Kill Shot:** Add a test module `tests/wasm_bindgen_facade.rs` that explicitly instantiates and runs the basic `NesWebEmulator` API to close these test gaps.

# Sentinel Journal Update
## YYYY-MM-DD - Equivalent Mutants in `nes-web`
**Mutant:** `replace NesWebEmulator::pause -> Result<(), JsValue> with Ok(())`, `replace NesWebEmulator::resume -> Result<(), JsValue> with Ok(())`, `replace NesWebEmulator::refresh_frame_rgba with ()`
**Diagnosis:**
- `pause()` and `resume()` toggle an internal `paused` boolean in `nes-core` which affects the `WebRuntime` auto-run loop, but the auto-run loop is in JavaScript (`app.js`), not Rust. Thus, the mutation cannot be caught purely via the `wasm_bindgen` getter APIs since the core pause state isn`t exported.
- `refresh_frame_rgba()` generates the frame buffer internally, but `frame_rgba()` fetches the latest one, and the underlying implementation of `refresh_frame_rgba()` only sets the buffer when the emulator runs. In tests, we are stepping the emulator directly, which updates the buffer via `step_frame()` anyways, masking the mutation.
- `to_js_error` mapping string to JsValue.
**Kill Shot:** Mark as equivalent/untestable from Rust, since they exist solely to bridge with the external JS auto-run loop.

## YYYY-MM-DD - Equivalent Mutants in NROM `write_prg`
**Mutant:** `replace Nrom::write_prg with ()`
**Diagnosis:** `Nrom::write_prg` delegates to `<Self as Mapper>::write_prg`, which is documented as doing nothing ("NROM has fixed PRG bank mapping and ignores bank-select writes"). Replacing a do-nothing function with `()` is mathematically equivalent and cannot be caught by any test observing state changes, because no state changes occur.
**Kill Shot:** Identified as EQUIVALENT_MUTANT.

## YYYY-MM-DD - Equivalent Mutants in `rom.rs`
**Mutant:** `replace | with ^` on lines 139, 141, 158, 159, and 163 in `parse_ines`
**Diagnosis:** The bitwise OR operations `|` are combining strictly disjoint bits (e.g., `(flags6 >> 4) | (flags7 & 0xF0)`). Since the fields do not overlap, `|` is mathematically identical to `^` (and `+`).
**Kill Shot:** Documented as `EQUIVALENT_MUTANT`.

## YYYY-MM-DD - Equivalent Mutant / Suspected Bug in `nes-netplay` RollbackEngine
**Mutant:** `replace RollbackEngine::clear_from with ()` in `crates/nes-netplay/src/rollback.rs`
**Diagnosis:** `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. The `clear_from` method uses `BTreeMap::split_off` to remove all state mappings for frames `>= start_frame`. However, immediately after `clear_from` is called in `rollback_from`, the logic iterates up to `self.next_frame - 1`, calling `simulate_frame` which re-inserts and perfectly overwrites all of these "cleared" future values. Therefore, replacing `clear_from` with `()` does not change any observable state, leaving the mutant alive. The operation is mathematically functionally redundant and adds unnecessary `BTreeMap` node deallocation/reallocation overhead compared to simply overwriting the values.
**Kill Shot:** Documented as `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. No test can assert the absence of `clear_from` because the state is fully overwritten in standard execution.

## YYYY-MM-DD - Equivalent Mutant / Suspected Bug in `nes-netplay` RollbackEngine
**Mutant:** `replace RollbackEngine::clear_from with ()` in `crates/nes-netplay/src/rollback.rs`
**Diagnosis:** `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. The `clear_from` method uses `BTreeMap::split_off` to remove all state mappings for frames `>= start_frame`. However, immediately after `clear_from` is called in `sync_frame`, the logic iterates exactly from `start_frame..self.next_frame`, calling `simulate_frame` which re-inserts and perfectly overwrites all of these "cleared" future values. Therefore, replacing `clear_from` with `()` does not change any observable state, leaving the mutant alive. The operation is mathematically functionally redundant and adds unnecessary `BTreeMap` node deallocation/reallocation overhead compared to simply overwriting the values.
**Kill Shot:** Documented as `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. No test can assert the absence of `clear_from` because the state is fully overwritten in standard execution.

**mmc1_equivalent_shift_push_bit**
**Mutant:** `crates/nes-core/src/mapper/mmc1.rs:137:58: replace | with ^ in Mmc1::push_shift_bit`
**Diagnosis:** EQUIVALENT_MUTANT. The operation `(shift_register >> 1) | (incoming << 4)` is mutated to use `^`. Because `shift_register` is restricted to 5 active bits (max value `0x1F`), shifting right leaves bit 4 empty. Combining an empty bit 4 with `incoming << 4` yields mathematically identical results whether `|` or `^` is used.
**Kill Shot:** None. This mutant is mathematically equivalent and cannot be killed.

## 2024-03-21 - Equivalent Mutant / Suspected Bug in `nes-netplay` RollbackEngine
**Mutant:** `replace RollbackEngine::clear_from with ()` in `crates/nes-netplay/src/rollback.rs`
**Diagnosis:** `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. The `clear_from` method uses `BTreeMap::split_off` to remove all state mappings for frames `>= start_frame`. However, immediately after `clear_from` is called in `rollback_from`, the logic iterates exactly from `start_frame..self.next_frame`, calling `simulate_frame` which re-inserts and perfectly overwrites all of these "cleared" future values. Therefore, replacing `clear_from` with `()` does not change any observable state, leaving the mutant alive. The operation is mathematically functionally redundant and adds unnecessary `BTreeMap` node deallocation/reallocation overhead compared to simply overwriting the values.
**Kill Shot:** Documented as `EQUIVALENT_MUTANT` / `SUSPECTED_BUG`. No test can assert the absence of `clear_from` because the state is fully overwritten in standard execution.

## YYYY-MM-DD - Equivalent Mutants in MMC3 `from_prg_chr`
**Mutant:** `replace < with <=` in `Mmc3::from_prg_chr` on line 89 (`if prg_rom.len() < min_prg_bytes`) and line 103 (`if chr_data.len() < CHR_WINDOW_BYTES`).
**Diagnosis:** `EQUIVALENT_MUTANT`. Changing `<` to `<=` causes the condition to evaluate to true when the ROM length is exactly the minimum required bytes. This leads to `resize(min_bytes, 0)` being called. However, since the vector's length is already equal to `min_bytes`, `resize` effectively does nothing, leaving the vector untouched. The operation is mathematically identical, meaning no test can catch this behavior change because there is no behavior change.
**Kill Shot:** Documented as `EQUIVALENT_MUTANT`.

## YYYY-MM-DD - Equivalent Mutants in GxROM `from_prg_chr`
**Mutant:** `replace < with <=` in `Gxrom::from_prg_chr` on line 35 (`if prg_rom.len() < PRG_BANK_32K`).
**Diagnosis:** `EQUIVALENT_MUTANT`. Changing `<` to `<=` causes the condition to evaluate to true when the ROM length is exactly 32KB. This leads to `resize(PRG_BANK_32K, 0)` being called. Since the vector's length is already equal to 32KB, `resize` does nothing. This mutation does not change observable behavior.
**Kill Shot:** Documented as `EQUIVALENT_MUTANT`.
**[Stall Penalty Boundary Logic]**
**Mutant:** `replace > with ==`, `<` and `>=` in `next.level_progress() > prev.level_progress()` at `crates/nes-ai/src/env.rs:163`
**Diagnosis:** `MISSING_COVERAGE` - Tests using the mock profile never advanced `level_progress` and never triggered the `stalled_frames >= stall_frames` penalty. Thus, mutants that inverted or changed the reset condition survived because `stalled_frames` never hit the penalty threshold or never reset correctly on a simulated positive progression.
**Kill Shot:** Exposed `core_mut()` in `ProfileEnv` to allow tests to manually write to the mock ROM memory location `0x006D` (the `level_progress` counter). Added `profile_env_applies_stall_penalty_when_no_progress_made` to test hitting the stall boundary and `profile_env_resets_stall_penalty_when_progress_increases` to test that the threshold resets to 0 when `level_progress` advances.
**Strengthened cheat_code and mapper state hashing**
**Mutant:** Replaced arithmetic and bitwise operators in `cheat_code_hash_component` and `mapper_hash_component`.
**Diagnosis:** Weak assertion. No tests existed to verify that adding different cheat codes produced distinctly perturbed state hashes.
**Kill Shot:** Wrote `cheat_code_hash_component_detects_shift_differences` and other rigorous tests. Wrote extensive mapper specific hash tests in `mapper_hash.rs`.
