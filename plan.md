1. *Create `crates/nes-core/src/experimental/oscilloscope.rs`*
   - Implement `OscilloscopeVisualizer` which takes audio samples and renders an oscilloscope waveform directly onto an RGBA framebuffer.
   - Support different visual styles (`Line` vs `Filled`).
   - Include proper unit tests to ensure safe framebuffer manipulation.
2. *Expose the module in `crates/nes-core/src/experimental/mod.rs`*
   - Add `pub mod oscilloscope;` protected by `#[cfg(feature = "nova")]`.
3. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. *Submit the change.*
   - Submit PR with the "Nova" template, explaining "The Spark", "The Feature", "The Potential", and "Risk".
