1. **Analyze performance bottlenecks related to audio chunk extraction**
   - Identify that `nes_core::api::NesCore::audio_chunk_i16` allocates a `Vec<i16>` containing `AUDIO_CHUNK_SAMPLES` (735 samples) every single frame.
   - At 60 FPS, this causes 60 heap allocations per second just to move audio data out of the core emulator.
   - `fill_audio_chunk_i16` already exists as an allocation-free alternative and is used by `nes-desktop`, but `nes_web` and `nes_test_harness` still call `audio_chunk_i16()`.

2. **Refactor `NesCore::audio_chunk_i16`**
   - Remove `audio_chunk_i16()` completely to eliminate the possibility of casual heap allocations by API users.
   - API users must use `fill_audio_chunk_i16(&mut buffer)` and manage their own buffers.

3. **Update `nes-web` to use a persistent buffer**
   - Add `audio_buffer: [i16; nes_core::AUDIO_CHUNK_SAMPLES]` to `WebRuntime` or expose an API that takes a memory pointer, or simply return a slice/pointer to an internal static/persistent buffer for JS.
   - `nes-web` currently returns `Vec<i16>` across the WASM boundary, which may get mapped to JS. Let's look at `wasm-bindgen` and see how it exposes this. If we change it to return a pointer or a slice, JS can read directly from the WASM heap without allocation.

4. **Update `nes-test-harness`**
   - Update `collect_audio_for_frames` to use a `Vec::with_capacity` and `fill_audio_chunk_i16` with a scratch buffer, or push directly.
   - Update `tests/rom_smb.rs` to use `fill_audio_chunk_i16` with a stack array instead of calling `audio_chunk_i16`.

5. **Verify changes**
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

6. **Complete pre commit steps**
   - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
