use alloc::boxed::Box;
use nes_core::{
    Button, Command, CoreError, NesCore, RomLoadInfo, AUDIO_CHUNK_SAMPLES, FRAME_HEIGHT,
    FRAME_WIDTH,
};

/// Pixels per video frame (`256 * 240`).
pub const FRAME_PIXELS: usize = FRAME_WIDTH * FRAME_HEIGHT;

/// Builds a controller bitfield from the set of currently pressed buttons.
///
/// Bit order matches the core: A, B, Select, Start, Up, Down, Left, Right in
/// bits 0-7.
///
/// ## Examples
///
/// ```
/// use nes_embassy::{Button, buttons};
/// assert_eq!(buttons(&[Button::A, Button::Start]), 0b0000_1001);
/// ```
#[must_use]
pub fn buttons(pressed: &[Button]) -> u8 {
    pressed
        .iter()
        .fold(0_u8, |bits, button| bits | button.bit_mask())
}

/// Owns a [`NesCore`] plus reusable frame buffers for tiny targets.
///
/// The firmware loop is:
///
/// 1. poll the controller GPIOs into two bitfields,
/// 2. [`EmuDriver::set_controllers`],
/// 3. [`EmuDriver::step_frame`] — pushes inputs, emulates exactly one frame,
///    and captures the RGB565 video frame and the audio chunk,
/// 4. push [`EmuDriver::frame_rgb565`] to the display and
///    [`EmuDriver::audio_chunk`] to the audio peripheral,
/// 5. pace to 60 Hz.
///
/// All buffers are heap-allocated once in [`EmuDriver::new`]; `step_frame`
/// never allocates.
pub struct EmuDriver {
    core: Box<NesCore>,
    pad1: u8,
    pad2: u8,
    frame: Box<[u16; FRAME_PIXELS]>,
    audio: Box<[i16; AUDIO_CHUNK_SAMPLES]>,
}

impl EmuDriver {
    /// Creates a driver with power-on core defaults and zeroed frame buffers.
    ///
    /// The [`NesCore`] (~78 KiB) is heap-allocated. In release mode the
    /// compiler constructs it directly on the heap via box placement,
    /// avoiding a stack temporary that would overflow RTOS task stacks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Box::new(NesCore::new()),
            pad1: 0,
            pad2: 0,
            frame: Box::new([0; FRAME_PIXELS]),
            audio: Box::new([0; AUDIO_CHUNK_SAMPLES]),
        }
    }

    /// Loads an iNES ROM into the core.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::RomLoadFailed`] when the ROM is not a supported
    /// iNES image.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<RomLoadInfo, CoreError> {
        self.core.load_ines_rom(rom)
    }

    /// Records the controller bitfields applied by the next
    /// [`EmuDriver::step_frame`]. Build them with [`buttons`].
    pub fn set_controllers(&mut self, pad1: u8, pad2: u8) {
        self.pad1 = pad1;
        self.pad2 = pad2;
    }

    /// Emulates exactly one frame (1/60 s): pushes the recorded controller
    /// state, steps the core, then captures the RGB565 frame and audio chunk
    /// into the driver's reusable buffers.
    ///
    /// # Errors
    ///
    /// Propagates [`CoreError`] from the core when stepping fails.
    pub fn step_frame(&mut self) -> Result<(), CoreError> {
        self.core.execute(Command::SetControllerState(self.pad1))?;
        self.core.execute(Command::SetController2State(self.pad2))?;
        self.core.execute(Command::StepFrame)?;
        self.core.fill_framebuffer_rgb565(&mut self.frame[..]);
        self.core.fill_audio_chunk_i16(&mut self.audio[..]);
        Ok(())
    }

    /// The most recent frame as RGB565 pixels (5-6-5 bits), row-major,
    /// `256 * 240` entries. Valid after [`EmuDriver::step_frame`].
    #[must_use]
    pub fn frame_rgb565(&self) -> &[u16] {
        &self.frame[..]
    }

    /// The most recent audio chunk: [`AUDIO_CHUNK_SAMPLES`] mono `i16` samples
    /// at [`AUDIO_SAMPLE_RATE`][nes_core::AUDIO_SAMPLE_RATE] Hz. Valid after
    /// [`EmuDriver::step_frame`].
    #[must_use]
    pub fn audio_chunk(&self) -> &[i16] {
        &self.audio[..]
    }

    /// Borrows the underlying core for queries the driver does not wrap
    /// (save states, TAS, debugging).
    #[must_use]
    pub fn core(&self) -> &NesCore {
        &self.core
    }

    /// Mutably borrows the underlying core.
    pub fn core_mut(&mut self) -> &mut NesCore {
        &mut self.core
    }
}

impl Default for EmuDriver {
    fn default() -> Self {
        Self::new()
    }
}
