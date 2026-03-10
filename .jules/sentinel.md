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
