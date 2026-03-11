#![doc = include_str!("../README.md")]

mod api;
mod apu;
/// BMP image encoding utilities.
pub mod bmp;
/// CPU bus region helpers and address classification.
pub mod bus;
/// 6502 CPU engine and status register helpers.
pub mod cpu;
/// Cartridge PRG mapper implementations and contracts.
pub mod mapper;
/// PPM image encoding utilities.
pub mod ppm;
mod ppu;
mod rom;
mod scheduler;
mod serde_array;
/// Stable TAS movie/recorder primitives built on top of the deterministic core.
#[cfg(feature = "tas")]
pub mod tas;

#[cfg(feature = "nova")]
pub mod experimental;

pub use api::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, Button, Command, CoreError, CoreQuery, CoreSnapshot,
    EmulatorState, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, MapperDelta, NesCore, QueryResult,
    RomLoadInfo,
};
