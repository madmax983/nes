#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

// `alloc` is used unconditionally: under `std` these are the same types, so a
// single set of imports serves both configurations.
extern crate alloc;

mod api;
mod apu;
/// BMP image encoding utilities.
pub mod bmp;
/// CPU bus region helpers and address classification.
pub mod bus;
/// NES cheat code decoding and patch metadata.
pub mod cheat_codes;
pub mod constants;
/// 6502 CPU engine and status register helpers.
pub mod cpu;
/// Cartridge PRG mapper implementations and contracts.
pub mod mapper;
/// PPM image encoding utilities.
///
/// Requires `std` for `std::io::Write`.
#[cfg(feature = "std")]
pub mod ppm;
mod ppu;
mod rom;
mod scheduler;
/// Workarounds for Serde's limitations with large arrays. Because sometimes you just need to serialize a 64KB block of RAM.
pub mod serde_array;
/// Stable TAS movie/recorder primitives built on top of the deterministic core.
#[cfg(feature = "tas")]
pub mod tas;

pub use api::{
    Button, Command, CoreError, CoreQuery, CoreSnapshot, EmulatorState, MapperDelta, NesCore,
    Player, QueryResult, RomLoadInfo,
};
pub use cheat_codes::{CheatCode, CheatCodeError};
pub use constants::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH,
};

#[cfg(feature = "nova")]
pub mod experimental;
