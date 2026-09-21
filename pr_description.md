💡 **The Spark:** Audio is a critical part of the NES experience, but it's entirely invisible. We have `ThemeFilter` and `PpuVisualizer` for seeing the visual internals, but no way to actually "see" the APU's raw waveform output. Bridging the gap between the audio PCM buffer and the RGBA framebuffer gives players and developers a tangible look at the generated audio.

🚀 **The Feature:** Implemented `OscilloscopeVisualizer` in `src/experimental/oscilloscope.rs`. It reads `i16` audio samples and maps them directly into an RGBA framebuffer overlay, supporting both `Line` and `Filled` visual styles. It automatically normalizes and decimates/stretches samples to fit the target screen width.

🔮 **The Potential:** Can be directly plugged into the desktop or TUI runtime by intercepting the audio buffer right before playback, giving players a classic Winamp-style audio visualizer directly in the game window. It can also help debug APU channel bugs visually (e.g. "Is the DMC channel actually firing?").

⚠️ **Risk:** Low. Isolated in `src/experimental/oscilloscope.rs` behind the `nova` feature flag. Unit tests ensure it does not panic even with mismatched buffer dimensions or empty sample lists.
