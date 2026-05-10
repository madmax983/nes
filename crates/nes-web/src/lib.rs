//! WebAssembly (Wasm) bindings for `nes-core`.
//!
//! This crate provides the [`NesWebEmulator`] struct, an interface tailored for JavaScript consumers.
//! It exposes the core NES emulation capabilities through the `wasm-bindgen` boundary, mapping simple
//! strings and arrays into complex emulator commands. This bridges the gap between the strict Rust
//! emulation layer and the flexible, event-driven world of the browser.
//!
//! ## Examples
//!
//! While [`NesWebEmulator`] requires a WebAssembly environment, the underlying [`WebRuntime`] can be used
//! natively for testing or alternative integrations:
//!
//! ```rust
//! use nes_web::WebRuntime;
//!
//! let mut runtime = WebRuntime::new();
//! // The runtime manages its own internal framebuffer allocation.
//! let frame_bytes = runtime.frame_rgba();
//! assert_eq!(frame_bytes.len(), 256 * 240 * 4);
//! ```

/// Web bridge layer translating JS events to core commands.
pub mod bridge;
/// Core WebAssembly runtime container for the emulator.
pub mod runtime;

pub use runtime::WebRuntime;

use wasm_bindgen::prelude::*;

/// The main WebAssembly interface for the NES emulator.
///
/// This struct holds the underlying emulation runtime and exposes methods that are
/// directly callable from JavaScript. It ensures memory safety and handles the conversion
/// of internal emulator errors into browser-friendly `JsValue` strings.
#[wasm_bindgen]
pub struct NesWebEmulator {
    runtime: WebRuntime,
}

#[wasm_bindgen]
impl NesWebEmulator {
    /// Creates a fresh instance of the web emulator.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// ```
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime: WebRuntime::new(),
        }
    }

    /// Loads an iNES format ROM from a byte array.
    ///
    /// This initializes the console and prepares it for execution.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// // In reality, these bytes would be parsed from an uploaded .nes file.
    /// let dummy_rom = [0; 16]; // A failing, invalid rom
    /// assert!(runtime.load_rom(&dummy_rom).is_err());
    /// ```
    pub fn load_rom(&mut self, rom_bytes: &[u8]) -> Result<(), JsValue> {
        self.runtime.load_rom(rom_bytes).map_err(to_js_error)
    }

    /// Advances the emulation by exactly one full NES frame.
    ///
    /// This is the primary workhorse method, designed to be called exactly once per
    /// `requestAnimationFrame` loop in the browser.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// // Step emulation. Without a ROM, this executes an infinite loop.
    /// runtime.step_frame().unwrap();
    /// ```
    pub fn step_frame(&mut self) -> Result<(), JsValue> {
        self.runtime.step_frame().map_err(to_js_error)
    }

    /// Advances the emulation by one PPU scanline.
    ///
    /// Useful for debugging tools that need to visualize the rendering process
    /// step-by-step rather than observing only fully composited frames.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_scanline().unwrap();
    /// ```
    pub fn step_scanline(&mut self) -> Result<(), JsValue> {
        self.runtime.step_scanline().map_err(to_js_error)
    }

    /// Advances the emulation by a single CPU instruction.
    ///
    /// Allows granular inspection of the console state for disassemblers and debuggers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_cpu().unwrap();
    /// ```
    pub fn step_cpu(&mut self) -> Result<(), JsValue> {
        self.runtime.step_cpu().map_err(to_js_error)
    }

    /// Suspends active execution.
    ///
    /// Calling `step_frame` while paused will have no effect on the emulator's state.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.pause().unwrap();
    /// ```
    pub fn pause(&mut self) -> Result<(), JsValue> {
        self.runtime.pause().map_err(to_js_error)
    }

    /// Restores active execution after a suspension.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.resume().unwrap();
    /// ```
    pub fn resume(&mut self) -> Result<(), JsValue> {
        self.runtime.resume().map_err(to_js_error)
    }

    /// Triggers the physical NES Reset button.
    ///
    /// This jumps the CPU to the Reset Vector but preserves internal memory contents,
    /// similar to briefly pressing reset on the real hardware.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.reset().unwrap();
    /// ```
    pub fn reset(&mut self) -> Result<(), JsValue> {
        self.runtime.reset().map_err(to_js_error)
    }

    /// Simulates unplugging and reconnecting the NES power supply.
    ///
    /// This entirely clears internal RAM and re-initializes all hardware registers to zero.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.power_cycle().unwrap();
    /// ```
    pub fn power_cycle(&mut self) -> Result<(), JsValue> {
        self.runtime.power_cycle().map_err(to_js_error)
    }

    /// Adjusts the execution speed multiplier.
    ///
    /// The value is provided as a permille (1/1000) scale. Setting `1000` means normal 1.0x speed,
    /// while `2000` is double speed. This is crucial for implementing fast-forward functionality.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.set_speed(2000).unwrap(); // 2.0x speed
    /// ```
    pub fn set_speed(&mut self, speed_permille: u16) -> Result<(), JsValue> {
        self.runtime.set_speed(speed_permille).map_err(to_js_error)
    }

    /// Overwrites the exact binary bitfield for Player 1's controller.
    ///
    /// Allows the web frontend to supply pre-composed inputs rather than relying on
    /// individual button press events, which is preferred for netplay or TAS playback.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// // 0x80 usually maps to the 'A' button depending on polling implementation.
    /// runtime.set_controller_state(0x80).unwrap();
    /// ```
    pub fn set_controller_state(&mut self, bits: u8) -> Result<(), JsValue> {
        self.runtime.set_controller_state(bits).map_err(to_js_error)
    }

    /// Emulates pushing down on a specific gamepad button.
    ///
    /// Valid strings match the NES controller layout: `"A"`, `"B"`, `"Select"`, `"Start"`, `"Up"`, `"Down"`, `"Left"`, `"Right"`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.press_button("Start").unwrap();
    /// ```
    pub fn press_button(&mut self, button: &str) -> Result<(), JsValue> {
        self.runtime.press_button(button).map_err(to_js_error)
    }

    /// Emulates letting go of a specific gamepad button.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.release_button("Start").unwrap();
    /// ```
    pub fn release_button(&mut self, button: &str) -> Result<(), JsValue> {
        self.runtime.release_button(button).map_err(to_js_error)
    }

    /// Maps a standard DOM `KeyboardEvent.code` to the correct NES button natively.
    ///
    /// This offloads key mapping logic to Rust, guaranteeing consistent controls regardless of
    /// frontend changes. It returns `true` if the key was recognized and mapped, `false` otherwise.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// let handled = runtime.dispatch_dom_key("ArrowUp", true).unwrap();
    /// assert!(handled);
    /// ```
    pub fn dispatch_dom_key(&mut self, key_code: &str, pressed: bool) -> Result<bool, JsValue> {
        self.runtime
            .dispatch_dom_key(key_code, pressed)
            .map_err(to_js_error)
    }

    /// Flushes the internal PPU rendering buffer into the RGBA memory segment.
    ///
    /// For zero-copy architectures, the frontend must manually call this before attempting
    /// to read from `frame_rgba_ptr`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.refresh_frame_rgba();
    /// ```
    pub fn refresh_frame_rgba(&mut self) {
        let _ = self.runtime.refresh_frame_rgba();
    }

    /// Allocates and returns a distinct copy of the current video frame.
    ///
    /// Note: Creating a new vector per frame incurs overhead. For high-performance
    /// 60 FPS rendering, prefer zero-copy memory access via `frame_rgba_ptr`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// let copy = runtime.frame_rgba();
    /// assert_eq!(copy.len(), 256 * 240 * 4);
    /// ```
    pub fn frame_rgba(&mut self) -> Vec<u8> {
        self.runtime.frame_rgba()
    }

    /// Exposes the starting memory address of the internal RGBA video buffer.
    ///
    /// The WebAssembly host can use this pointer to instantiate an `Uint8ClampedArray`
    /// directly over the shared memory region, bypassing expensive byte copies.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let ptr = runtime.frame_rgba_ptr();
    /// assert!(!ptr.is_null());
    /// ```
    #[must_use]
    pub fn frame_rgba_ptr(&self) -> *const u8 {
        self.runtime.frame_rgba_ptr()
    }

    /// Exposes the expected byte length of the shared video buffer.
    ///
    /// Usually `256 * 240 * 4 = 245760`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.frame_rgba_len(), 245760);
    /// ```
    #[must_use]
    pub fn frame_rgba_len(&self) -> usize {
        self.runtime.frame_rgba_len()
    }

    /// Drains any synthesized audio samples into a vector.
    ///
    /// This consumes the internal audio buffer. If called sparsely, the emulator
    /// guarantees it will return all audio generated since the previous query.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_frame().unwrap();
    /// let chunk = runtime.audio_chunk_i16();
    /// assert!(!chunk.is_empty());
    /// ```
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        self.runtime.audio_chunk_i16()
    }

    /// Exposes the active instruction address of the CPU.
    ///
    /// This is an essential telemetry metric for debugging crashes or synchronization loss.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// // Without a ROM, the PC initializes to a default value like 0xC000.
    /// let pc = runtime.cpu_pc();
    /// ```
    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.runtime.cpu_pc()
    }

    /// Exposes the observed Frames Per Second.
    ///
    /// Represented as a permille. For instance, `60000` indicates exactly 60.000 FPS.
    /// Useful for telemetry and adjusting `requestAnimationFrame` budgets.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let fps = runtime.fps_milli();
    /// ```
    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        self.runtime.fps_milli()
    }

    /// Exposes the total number of frames rendered since initialization.
    ///
    /// This provides a stable timeline identifier for rollback states and recording.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_frame().unwrap();
    /// assert_eq!(runtime.ppu_frame_counter(), 1);
    /// ```
    #[must_use]
    pub fn ppu_frame_counter(&self) -> u64 {
        self.runtime.ppu_frame_counter()
    }

    /// Computes a lightweight structural hash of all internal state (RAM and Registers).
    ///
    /// This guarantees that two emulator instances with the exact same hash will simulate
    /// identical future states. Essential for verifying desyncs in peer-to-peer Netplay.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let hash1 = runtime.state_hash();
    /// let hash2 = runtime.state_hash();
    /// assert_eq!(hash1, hash2);
    /// ```
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        self.runtime.state_hash()
    }

    /// Exposes the active controller inputs as an 8-bit mask.
    ///
    /// Used by frontend UI overlays to visually demonstrate which buttons are currently
    /// held down.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.press_button("Start").unwrap();
    /// let bits = runtime.controller_bits();
    /// ```
    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.runtime.controller_bits()
    }

    /// The exact horizontal dimension of an NES display output.
    ///
    /// Always returns `256`. Web frontends use this value to configure `<canvas>` element dimensions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.frame_width(), 256);
    /// ```
    #[must_use]
    pub fn frame_width(&self) -> u32 {
        self.runtime.frame_width()
    }

    /// The exact vertical dimension of an NES display output.
    ///
    /// Always returns `240`. Web frontends use this value to configure `<canvas>` element dimensions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.frame_height(), 240);
    /// ```
    #[must_use]
    pub fn frame_height(&self) -> u32 {
        self.runtime.frame_height()
    }

    /// The specific sample rate the audio generator expects to consume.
    ///
    /// Typically `44100`. The browser's Web Audio API context must be configured to this frequency.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.audio_sample_rate(), 44100);
    /// ```
    #[must_use]
    pub fn audio_sample_rate(&self) -> u32 {
        self.runtime.audio_sample_rate()
    }

    /// The exact number of audio frames expected per execution chunk.
    ///
    /// Helps initialize `AudioBuffer` sizes in the frontend perfectly, preventing stuttering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let expected_samples = runtime.audio_chunk_samples();
    /// ```
    #[must_use]
    pub fn audio_chunk_samples(&self) -> u32 {
        self.runtime.audio_chunk_samples()
    }
}

impl Default for NesWebEmulator {
    fn default() -> Self {
        Self::new()
    }
}

fn to_js_error(err: String) -> JsValue {
    JsValue::from_str(&err)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Do not test to_js_error natively to prevent coverage panics in CI tools
    // that don't capture aborts well (e.g., cargo tarpaulin).
    // Mutants are handled by the wasm runner.
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_to_js_error_output() {
        let err_msg = "test error msg".to_string();
        let js_err = to_js_error(err_msg.clone());
        assert_eq!(js_err.as_string(), Some(err_msg));
    }
}
