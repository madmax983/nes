#![doc = include_str!("../README.md")]

mod api;
mod apu;
/// CPU bus region helpers and address classification.
pub mod bus;
/// 6502 CPU engine and status register helpers.
pub mod cpu;
/// Cartridge PRG mapper implementations and contracts.
pub mod mapper;
mod ppu;
mod rom;
mod scheduler;
mod serde_array;

#[cfg(feature = "nova")]
pub(crate) mod experimental;

pub use api::{
    Button, Command, CoreError, CoreQuery, CoreSnapshot,
    EmulatorState, MapperDelta, NesCore, QueryResult,
    RomLoadInfo,
};
pub use apu::{AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE};
pub use ppu::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
