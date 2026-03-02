use nes_core::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, Button, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES,
    FRAME_WIDTH, NesCore,
};

use crate::bridge::map_dom_key_to_command;

#[derive(Debug, Clone)]
pub struct WebRuntime {
    core: NesCore,
    frame_rgba: Vec<u8>,
}

impl WebRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: NesCore::new(),
            frame_rgba: vec![0; FRAME_RGBA_BYTES],
        }
    }

    pub fn load_rom(&mut self, rom_bytes: &[u8]) -> Result<(), String> {
        self.core
            .load_ines_rom(rom_bytes)
            .map(|_| ())
            .map_err(|err| format!("failed to load rom: {err}"))
    }

    pub fn step_frame(&mut self) -> Result<(), String> {
        self.execute(Command::StepFrame)
    }

    pub fn step_scanline(&mut self) -> Result<(), String> {
        self.execute(Command::StepScanline)
    }

    pub fn step_cpu(&mut self) -> Result<(), String> {
        self.execute(Command::StepCpu)
    }

    pub fn pause(&mut self) -> Result<(), String> {
        self.execute(Command::Pause)
    }

    pub fn resume(&mut self) -> Result<(), String> {
        self.execute(Command::Resume)
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.execute(Command::Reset)
    }

    pub fn power_cycle(&mut self) -> Result<(), String> {
        self.execute(Command::PowerCycle)
    }

    pub fn set_speed(&mut self, speed_permille: u16) -> Result<(), String> {
        self.execute(Command::SetSpeed(speed_permille))
    }

    pub fn set_controller_state(&mut self, bits: u8) -> Result<(), String> {
        self.execute(Command::SetControllerState(bits))
    }

    pub fn press_button(&mut self, button: &str) -> Result<(), String> {
        let parsed = parse_button(button)?;
        self.execute(Command::PressButton(parsed))
    }

    pub fn release_button(&mut self, button: &str) -> Result<(), String> {
        let parsed = parse_button(button)?;
        self.execute(Command::ReleaseButton(parsed))
    }

    pub fn dispatch_dom_key(&mut self, key_code: &str, pressed: bool) -> Result<bool, String> {
        let Some(mapped) = map_dom_key_to_command(key_code, pressed) else {
            return Ok(false);
        };
        self.execute(mapped.core)?;
        Ok(true)
    }

    #[must_use]
    pub fn refresh_frame_rgba(&mut self) -> &[u8] {
        self.core.fill_framebuffer_rgba(&mut self.frame_rgba);
        &self.frame_rgba
    }

    #[must_use]
    pub fn frame_rgba(&mut self) -> Vec<u8> {
        self.refresh_frame_rgba().to_vec()
    }

    #[must_use]
    pub fn frame_rgba_ptr(&self) -> *const u8 {
        self.frame_rgba.as_ptr()
    }

    #[must_use]
    pub fn frame_rgba_len(&self) -> usize {
        self.frame_rgba.len()
    }

    #[must_use]
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        self.core.audio_chunk_i16()
    }

    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.core.cpu_pc()
    }

    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        self.core.fps_milli()
    }

    #[must_use]
    pub fn ppu_frame_counter(&self) -> u64 {
        self.core.ppu_frame_counter()
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        self.core.state_hash()
    }

    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.core.controller_bits()
    }

    #[must_use]
    pub fn frame_width(&self) -> u32 {
        FRAME_WIDTH as u32
    }

    #[must_use]
    pub fn frame_height(&self) -> u32 {
        FRAME_HEIGHT as u32
    }

    #[must_use]
    pub fn audio_sample_rate(&self) -> u32 {
        AUDIO_SAMPLE_RATE
    }

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
