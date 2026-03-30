1. **Remove heap allocation in audio extraction**
   - The current `nes_core::api::NesCore::audio_chunk_i16` method creates and returns a new `Vec<i16>` every frame, which adds overhead.
   - We will remove this method entirely from `nes-core/src/api.rs`.
   - Callers will be forced to use the existing allocation-free `fill_audio_chunk_i16` method.

2. **Update `nes-web` crate**
   - In `crates/nes-web/src/runtime.rs`, we will add a new persistent array: `audio_chunk: [i16; nes_core::AUDIO_CHUNK_SAMPLES]`.
   - Update `WebRuntime` to fill this array using `core.fill_audio_chunk_i16(&mut self.audio_chunk)`.
   - Modify `WebRuntime::audio_chunk_i16` (and `WasmNesEmulator::audio_chunk_i16` in `lib.rs`) to just return a clone of the internal array data, OR better yet, we can't avoid `Vec` at the JS boundary without more complex WASM binding changes, so we will return `self.audio_chunk.to_vec()`. Wait! JS already takes it as `Int16Array`! Actually, if we return `Vec<i16>` from `wasm_bindgen`, it allocates. A better zero-copy way is `*const i16` and exposing `audio_chunk_ptr()` and `audio_chunk_len()`, mirroring how `frame_rgba` is handled. We will implement `audio_chunk_ptr` and `audio_chunk_len`, but `wasm-bindgen` already has a `Vec<i16>` interface. I will check how `web/app.js` consumes this. Wait, `web/app.js` is not provided in my search. Wait, since the prompt just asks for ONE small performance improvement, I can just replace `audio_chunk_i16` inside `WebRuntime` to return `Vec<i16>` from `self.audio_chunk.to_vec()`, OR we can just use `Vec::with_capacity` in the bindings instead of in `core`. But actually `WebRuntime` could just be:
     ```rust
     pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
         let mut buffer = vec![0; nes_core::AUDIO_CHUNK_SAMPLES];
         self.core.fill_audio_chunk_i16(&mut buffer);
         buffer
     }
     ```
     This keeps `nes-web` exactly the same API but moves the allocation responsibility out of `nes-core`.

3. **Update `nes-test-harness`**
   - In `crates/nes-test-harness/src/lib.rs`, `collect_audio_for_frames` uses `extend(core.audio_chunk_i16())`. We will replace it with:
     ```rust
     let mut chunk = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
     core.fill_audio_chunk_i16(&mut chunk);
     samples.extend_from_slice(&chunk);
     ```
   - In `crates/nes-test-harness/tests/rom_smb.rs`, replace `core.audio_chunk_i16()` with `fill_audio_chunk_i16`.

4. **Verify changes**
   - Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`.

5. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
