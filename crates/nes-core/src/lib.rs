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

#[cfg(feature = "nova")]
pub mod experimental;

pub use api::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, Button, Command, CoreError, CoreQuery, CoreSnapshot,
    EmulatorState, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, MapperDelta, NesCore, QueryResult,
    RomLoadInfo,
};
