//! Embedded iNES ROM.
//!
//! Ships the repo's homebrew test ROM by default. To play something else,
//! drop your own `.nes` file next to this module and update the path.
//! (Only cartridges your heap can hold: ~320 KiB internal, more with the
//! `psram` feature.)

pub const ROM: &[u8] = include_bytes!("../../../../../roms/homebrew/homebrew.nes");
