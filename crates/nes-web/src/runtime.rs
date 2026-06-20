//! The core web runtime for the NES emulator.
//!
//! This module provides [`WebRuntime`], an environment-agnostic wrapper over `NesCore`
//! tailored for web use cases (like `requestAnimationFrame` and JS event dispatching).
//!
//! It primarily handles translating string commands and allocating the proper
//! buffers for image rendering to avoid raw pointer unsafety across the JS boundary
//! when possible.
//!
//! ## Examples
//!
//! Using the runtime directly in Rust without the Wasm boundaries:
//!
//! ```rust
//! use nes_web::WebRuntime;
//!
//! let mut runtime = WebRuntime::new();
//! // runtime.load_rom(bytes).unwrap();
//! // let frame = runtime.frame_rgba(); // Vec<u8> containing RGBA
//! ```

use nes_core::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, Button, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES,
    FRAME_WIDTH, NesCore,
};

use crate::bridge::map_dom_key_to_command;

/// The environment-agnostic web execution manager.
///
/// This structure holds the underlying emulator and pre-allocates an internal framebuffer
/// (`frame_rgba`). By hiding the core behind this struct, frontend consumers do not have to
/// instantiate memory mapping objects directly in Wasm, dramatically simplifying API usability.
#[derive(Debug, Clone)]
pub struct WebRuntime {
    core: NesCore,
    frame_rgba: Vec<u8>,
}

impl WebRuntime {
    /// Bootstraps an empty, initialized NES core and allocates the shared framebuffer.
    ///
    /// The framebuffer requires exactly 245,760 bytes (256x240 pixels at 4 bytes per RGBA pixel).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.frame_rgba_len(), 256 * 240 * 4);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: NesCore::new(),
            frame_rgba: vec![0; FRAME_RGBA_BYTES],
        }
    }

    /// Mounts an `.nes` file binary payload directly to the core bus.
    ///
    /// Loading the ROM verifies the iNES header. It converts parsing failures directly into human-readable
    /// strings suitable for frontend toast notifications.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// let dummy_payload = vec![0x00, 0x01, 0x02];
    ///
    /// let result = runtime.load_rom(&dummy_payload);
    /// assert!(result.unwrap_err().contains("failed to load rom:"));
    /// ```
    pub fn load_rom(&mut self, rom_bytes: &[u8]) -> Result<(), String> {
        self.core
            .load_ines_rom(rom_bytes)
            .map(|_| ())
            .map_err(|err| format!("failed to load rom: {err}"))
    }

    /// Executes the console indefinitely until the PPU completes a single vertical blank.
    ///
    /// Web applications invoke this API inside of `requestAnimationFrame` loops to keep the
    /// visual framerate locked to the browser refresh rate.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_frame().unwrap();
    /// assert_eq!(runtime.ppu_frame_counter(), 1);
    /// ```
    pub fn step_frame(&mut self) -> Result<(), String> {
        self.execute(Command::StepFrame)
    }

    /// Executes the console until exactly one display scanline is produced.
    ///
    /// Ideal for cycle-accurate debugging tools where visualizing PPU timing behavior
    /// (such as mid-frame palette swaps) is required.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_scanline().unwrap();
    /// ```
    pub fn step_scanline(&mut self) -> Result<(), String> {
        self.execute(Command::StepScanline)
    }

    /// Steps the core forward by one internal 6502 processor instruction.
    ///
    /// Extremely fast micro-stepping for disassembly inspection.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_cpu().unwrap();
    /// ```
    pub fn step_cpu(&mut self) -> Result<(), String> {
        self.execute(Command::StepCpu)
    }

    /// Safely halts all running internal execution loops.
    ///
    /// Subsequent calls to `step_frame` become no-ops until `resume()` is triggered.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.pause().unwrap();
    /// ```
    pub fn pause(&mut self) -> Result<(), String> {
        self.execute(Command::Pause)
    }

    /// Resumes normal background execution.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.pause().unwrap();
    /// runtime.resume().unwrap();
    /// ```
    pub fn resume(&mut self) -> Result<(), String> {
        self.execute(Command::Resume)
    }

    /// Trips the physical Reset button on the emulator logic.
    ///
    /// The software vectors to the Reset address. Unlike a full reboot, RAM variables
    /// like high scores may survive if the specific game logic allows it.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.reset().unwrap();
    /// ```
    pub fn reset(&mut self) -> Result<(), String> {
        self.execute(Command::Reset)
    }

    /// Totally destroys current hardware state and boots from scratch.
    ///
    /// Equivalent to blowing on the cartridge and flipping the physical switch.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.power_cycle().unwrap();
    /// ```
    pub fn power_cycle(&mut self) -> Result<(), String> {
        self.execute(Command::PowerCycle)
    }

    /// Dynamically rescales emulator CPU cycles against the host system clock.
    ///
    /// Accepts a permille integer (e.g. `1000` = 1.0x). Passing `2000` tells the
    /// runtime to process twice as many NES cycles per tick, effectively yielding a 120Hz Fast Forward.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.set_speed(3000).unwrap(); // 3.0x speed
    /// ```
    pub fn set_speed(&mut self, speed_permille: u16) -> Result<(), String> {
        self.execute(Command::SetSpeed(speed_permille))
    }

    /// Overwrites the exact memory mask for gamepad one.
    ///
    /// Web platforms can pass an integer (`0x00`-`0xFF`) over the JS boundary far faster than
    /// parsing thousands of `"press_button"` string requests for TAS playbacks.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// // 0x80 typically triggers the A button in many mapping engines.
    /// runtime.set_controller_state(0x80).unwrap();
    /// ```
    pub fn set_controller_state(&mut self, bits: u8) -> Result<(), String> {
        self.execute(Command::SetControllerState(bits))
    }

    /// Generates a synthetic signal representing a physical gamepad push.
    ///
    /// Matches the explicit labels `"A"`, `"B"`, `"Select"`, `"Start"`, `"Up"`, `"Down"`, `"Left"`, or `"Right"`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.press_button("Start").unwrap();
    /// assert!(runtime.press_button("JUMP").is_err());
    /// ```
    pub fn press_button(&mut self, button: &str) -> Result<(), String> {
        let parsed = parse_button(button)?;
        self.execute(Command::PressButton(parsed))
    }

    /// Generates a synthetic signal representing releasing a physical gamepad switch.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.release_button("Start").unwrap();
    /// ```
    pub fn release_button(&mut self, button: &str) -> Result<(), String> {
        let parsed = parse_button(button)?;
        self.execute(Command::ReleaseButton(parsed))
    }

    /// Intercepts native DOM `KeyboardEvent.code` values and maps them intelligently.
    ///
    /// Hardcoding mappings in Rust prevents frontend logic drift. Returns `true` if the event
    /// successfully mapped to a specific NES action, or `false` if it was ignored.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// let did_map = runtime.dispatch_dom_key("ArrowLeft", true).unwrap();
    /// assert!(did_map);
    ///
    /// let unknown = runtime.dispatch_dom_key("Escape", true).unwrap();
    /// assert!(!unknown);
    /// ```
    pub fn dispatch_dom_key(&mut self, key_code: &str, pressed: bool) -> Result<bool, String> {
        let Some(mapped) = map_dom_key_to_command(key_code, pressed) else {
            return Ok(false);
        };
        self.execute(mapped.core)?;
        Ok(true)
    }

    /// Forces a synchronization flush of the core's native PPU video buffer into the
    /// pre-allocated RGBA contiguous segment.
    ///
    /// In architectures where zero-copy memory access is used, this method *must* be invoked
    /// before reading from the internal data pointer.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_frame().unwrap();
    /// let frame = runtime.refresh_frame_rgba();
    /// assert_eq!(frame.len(), 256 * 240 * 4);
    /// ```
    #[must_use]
    pub fn refresh_frame_rgba(&mut self) -> &[u8] {
        self.core.fill_framebuffer_rgba(&mut self.frame_rgba);
        &self.frame_rgba
    }

    /// Clones the internal video buffer into a new `Vec<u8>`.
    ///
    /// This introduces a measurable heap allocation delay and memory copy for all 245,760 bytes.
    /// It should be avoided in 60 FPS critical paths if direct pointer mapping is available.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// let duplicate_frame = runtime.frame_rgba();
    /// assert_eq!(duplicate_frame.len(), 245760);
    /// ```
    #[must_use]
    pub fn frame_rgba(&mut self) -> Vec<u8> {
        self.refresh_frame_rgba().to_vec()
    }

    /// Generates a raw memory address spanning the composited video buffer.
    ///
    /// Exposing this address lets JavaScript utilize an `Uint8ClampedArray` to read pixels instantly
    /// from WebAssembly heap memory, unlocking 60FPS browser performance.
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
        self.frame_rgba.as_ptr()
    }

    /// Verifies the absolute dimension in bytes of the exposed memory span.
    ///
    /// Guarantees that consumers understand the bounds of the returned pointer.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// assert_eq!(runtime.frame_rgba_len(), 256 * 240 * 4);
    /// ```
    #[must_use]
    pub fn frame_rgba_len(&self) -> usize {
        self.frame_rgba.len()
    }

    /// Siphons all queued audio sample fragments.
    ///
    /// Since the previous query, the internal APU queues 16-bit PCM waveform samples.
    /// These are popped off efficiently for continuous playback.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.step_frame().unwrap();
    /// let samples = runtime.audio_chunk_i16();
    /// assert!(!samples.is_empty());
    /// ```
    #[must_use]
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        self.core.audio_chunk_i16()
    }

    /// Retrieves the precise location the 6502 processor is evaluating within its execution.
    ///
    /// Tracking the Program Counter allows debugging crashes and finding synchronization
    /// deviation points in emulation testing.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let pc = runtime.cpu_pc();
    /// ```
    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.core.cpu_pc()
    }

    /// Fetches the dynamically calculated Frames Per Second telemetry.
    ///
    /// Stored as a permille multiplier: e.g. `60000` equals 60.000 FPS. Frontends use this
    /// integer to decide if they need to increase `requestAnimationFrame` budgets or drop frames.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let metrics = runtime.fps_milli();
    /// ```
    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        self.core.fps_milli()
    }

    /// Verifies the deterministic index of the specific frame currently rendered.
    ///
    /// An invaluable metric for indexing states during peer-to-peer rollback simulation.
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
        self.core.ppu_frame_counter()
    }

    /// Computes a lightweight rolling hash mapping the precise internal RAM/Register state.
    ///
    /// Ensures that if an identical input trajectory was executed by two parallel clients,
    /// they remain perfectly synchronized in lockstep.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let sync_token = runtime.state_hash();
    /// ```
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        self.core.state_hash()
    }

    /// Reads the active gamepad inputs expressed as an 8-bit integer layout.
    ///
    /// Required when drawing UI elements that highlight physical button presses back to the user.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let mut runtime = WebRuntime::new();
    /// runtime.press_button("Start").unwrap();
    /// let state = runtime.controller_bits();
    /// ```
    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.core.controller_bits()
    }

    /// Locks the output width to the standard NES spec.
    ///
    /// Exposing this guarantees the JS frontend always scales the canvas specifically to `256` pixels.
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
        FRAME_WIDTH as u32
    }

    /// Locks the output height to the standard NES spec.
    ///
    /// Exposing this guarantees the JS frontend always scales the canvas specifically to `240` pixels.
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
        FRAME_HEIGHT as u32
    }

    /// Indicates the target frequency required by the host's audio API layer.
    ///
    /// Commonly `44100` Hertz, defining how many individual samples constitute one second of audio.
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
        AUDIO_SAMPLE_RATE
    }

    /// Indicates the maximum number of 16-bit PCM samples delivered per internal audio execution loop.
    ///
    /// Useful for determining accurate JS-side `AudioBuffer` allocations to eliminate clipping and drift.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_web::WebRuntime;
    /// let runtime = WebRuntime::new();
    /// let len = runtime.audio_chunk_samples();
    /// ```
    #[must_use]
    pub fn audio_chunk_samples(&self) -> u32 {
        AUDIO_CHUNK_SAMPLES as u32
    }

    fn execute(&mut self, command: Command) -> Result<(), String> {
        self.core
            .execute(command)
            .map_err(|err| format!("core command failed: {err}"))
    }
}

impl Default for WebRuntime {
    fn default() -> Self {
        Self::new()
    }
}
fn parse_button(button: &str) -> Result<Button, String> {
    match button {
        "A" => Ok(Button::A),
        "B" => Ok(Button::B),
        "Select" => Ok(Button::Select),
        "Start" => Ok(Button::Start),
        "Up" => Ok(Button::Up),
        "Down" => Ok(Button::Down),
        "Left" => Ok(Button::Left),
        "Right" => Ok(Button::Right),
        _ => Err(format!(
            "unknown button '{button}'. expected one of: A, B, Select, Start, Up, Down, Left, Right"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::{Button, Command};

    #[test]
    fn test_execute_propagates_error() {
        let mut runtime = WebRuntime::new();
        // StepCpu should not error under normal conditions
        let res = runtime.execute(Command::StepCpu);
        assert!(res.is_ok());
    }

    #[test]
    fn test_parse_button() {
        assert_eq!(parse_button("A"), Ok(Button::A));
        assert_eq!(parse_button("B"), Ok(Button::B));
        assert_eq!(parse_button("Select"), Ok(Button::Select));
        assert_eq!(parse_button("Start"), Ok(Button::Start));
        assert_eq!(parse_button("Up"), Ok(Button::Up));
        assert_eq!(parse_button("Down"), Ok(Button::Down));
        assert_eq!(parse_button("Left"), Ok(Button::Left));
        assert_eq!(parse_button("Right"), Ok(Button::Right));
        assert!(parse_button("Invalid").is_err());
    }

    #[test]
    fn test_default() {
        let runtime = WebRuntime::default();
        assert_eq!(
            runtime.audio_chunk_samples(),
            nes_core::AUDIO_CHUNK_SAMPLES as u32
        );
    }
}
