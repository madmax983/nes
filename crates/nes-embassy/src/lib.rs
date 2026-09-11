//! Hardware-neutral `no_std` adapter between [`nes_core::NesCore`] and tiny
//! Embassy targets.
//!
//! [`EmuDriver`] owns the emulator plus its two reusable frame buffers (one
//! RGB565 video frame, one audio chunk) so firmware only wires up platform
//! pieces: poll buttons, call [`EmuDriver::step_frame`], push pixels to the
//! display, push samples to the DAC/I2S peripheral, and pace to 60 Hz with
//! `embassy_time::Timer`.
//!
//! The driver itself has no Embassy dependency and no `std` requirement beyond
//! `alloc`, so the exact same code runs in host tests.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod driver;

pub use driver::{buttons, EmuDriver};
pub use nes_core::{Button, CoreError, CoreQuery, QueryResult, RomLoadInfo};
pub use nes_core::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, FRAME_HEIGHT, FRAME_RGB565_BYTES, FRAME_WIDTH,
};
