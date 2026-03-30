1. **Refactor `NesCore::audio_chunk_i16` to force zero-allocation usage**
   - The current `nes_core::api::NesCore::audio_chunk_i16` method creates and returns a new `Vec<i16>` every frame, causing unnecessary heap allocations.
   - I will remove this method from `nes-core/src/api.rs`.
   - All users must use the existing allocation-free `fill_audio_chunk_i16(&mut self, samples: &mut [i16])` method instead.

2. **Update `nes-web` to preserve its API while avoiding core allocations**
   - In `crates/nes-web/src/runtime.rs`, I will change `audio_chunk_i16` to manually create the `Vec` and fill it:
     ```rust
     pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
         let mut buffer = vec![0; nes_core::AUDIO_CHUNK_SAMPLES];
         self.core.fill_audio_chunk_i16(&mut buffer);
         buffer
     }
     ```
   - This keeps `nes-web` exactly the same without changing the Web API bindings or `web/app.js`.

3. **Update `nes-test-harness` to use the allocation-free API**
   - In `crates/nes-test-harness/src/lib.rs`, `collect_audio_for_frames` will use a stack array and `extend_from_slice`:
     ```rust
     let mut chunk = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
     core.fill_audio_chunk_i16(&mut chunk);
     samples.extend_from_slice(&chunk);
     ```
   - In `crates/nes-test-harness/tests/rom_smb.rs`, replace `core.audio_chunk_i16()` with `let mut chunk = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES]; core.fill_audio_chunk_i16(&mut chunk);`.

4. **Verify the optimization and absence of regressions**
   - Use `cargo test --workspace` to ensure tests still pass.
   - Run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`.

5. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
