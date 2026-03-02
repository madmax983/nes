//! Cartridge PRG mapper abstractions and implementations.

mod axrom;
mod cnrom;
mod mmc1;
mod mmc3;
mod nrom;
mod uxrom;

pub use axrom::Axrom;
pub use cnrom::Cnrom;
pub use mmc1::Mmc1;
pub use mmc3::Mmc3;
pub use nrom::Nrom;
pub use uxrom::Uxrom;

/// Common PRG read/write contract for cartridge mappers.
///
/// Addresses are CPU PRG-space addresses (`0x8000..=0xFFFF`).
pub trait Mapper {
    /// Reads a byte from mapped PRG space.
    fn read_prg(&self, addr: u16) -> u8;
    /// Handles mapper register writes in PRG space.
    fn write_prg(&mut self, addr: u16, value: u8);
}
