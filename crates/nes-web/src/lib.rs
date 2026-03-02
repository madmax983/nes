pub mod bridge;
pub mod runtime;

pub use runtime::WebRuntime;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NesWebEmulator {
    runtime: WebRuntime,
}

#[wasm_bindgen]
impl NesWebEmulator {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime: WebRuntime::new(),
        }
    }

    pub fn load_rom(&mut self, rom_bytes: &[u8]) -> Result<(), JsValue> {
        self.runtime.load_rom(rom_bytes).map_err(to_js_error)
    }

    pub fn step_frame(&mut self) -> Result<(), JsValue> {
        self.runtime.step_frame().map_err(to_js_error)
    }

    pub fn step_scanline(&mut self) -> Result<(), JsValue> {
        self.runtime.step_scanline().map_err(to_js_error)
    }

    pub fn step_cpu(&mut self) -> Result<(), JsValue> {
        self.runtime.step_cpu().map_err(to_js_error)
    }

    pub fn pause(&mut self) -> Result<(), JsValue> {
        self.runtime.pause().map_err(to_js_error)
    }

    pub fn resume(&mut self) -> Result<(), JsValue> {
        self.runtime.resume().map_err(to_js_error)
    }

    pub fn reset(&mut self) -> Result<(), JsValue> {
        self.runtime.reset().map_err(to_js_error)
    }

    pub fn power_cycle(&mut self) -> Result<(), JsValue> {
        self.runtime.power_cycle().map_err(to_js_error)
    }

    pub fn set_speed(&mut self, speed_permille: u16) -> Result<(), JsValue> {
        self.runtime.set_speed(speed_permille).map_err(to_js_error)
    }

    pub fn set_controller_state(&mut self, bits: u8) -> Result<(), JsValue> {
        self.runtime.set_controller_state(bits).map_err(to_js_error)
    }

    pub fn press_button(&mut self, button: &str) -> Result<(), JsValue> {
        self.runtime.press_button(button).map_err(to_js_error)
    }

    pub fn release_button(&mut self, button: &str) -> Result<(), JsValue> {
        self.runtime.release_button(button).map_err(to_js_error)
    }

    pub fn dispatch_dom_key(&mut self, key_code: &str, pressed: bool) -> Result<bool, JsValue> {
        self.runtime
            .dispatch_dom_key(key_code, pressed)
            .map_err(to_js_error)
    }

    pub fn refresh_frame_rgba(&mut self) {
        let _ = self.runtime.refresh_frame_rgba();
    }

    pub fn frame_rgba(&mut self) -> Vec<u8> {
        self.runtime.frame_rgba()
    }

    #[must_use]
    pub fn frame_rgba_ptr(&self) -> *const u8 {
        self.runtime.frame_rgba_ptr()
    }

    #[must_use]
    pub fn frame_rgba_len(&self) -> usize {
        self.runtime.frame_rgba_len()
    }

    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        self.runtime.audio_chunk_i16()
    }

    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.runtime.cpu_pc()
    }

    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        self.runtime.fps_milli()
    }

    #[must_use]
    pub fn ppu_frame_counter(&self) -> u64 {
        self.runtime.ppu_frame_counter()
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        self.runtime.state_hash()
    }

    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.runtime.controller_bits()
    }

    #[must_use]
    pub fn frame_width(&self) -> u32 {
        self.runtime.frame_width()
    }

    #[must_use]
    pub fn frame_height(&self) -> u32 {
        self.runtime.frame_height()
    }

    #[must_use]
    pub fn audio_sample_rate(&self) -> u32 {
        self.runtime.audio_sample_rate()
    }

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
