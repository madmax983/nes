//! Cartridge PRG mapper abstractions and implementations.

mod axrom;
mod cnrom;
mod gxrom;
mod mmc1;
mod mmc3;
mod nrom;
mod uxrom;

pub use axrom::Axrom;
pub(crate) use axrom::AxromState;
pub use cnrom::Cnrom;
pub(crate) use cnrom::CnromState;
pub use gxrom::Gxrom;
pub(crate) use gxrom::GxromState;
pub use mmc1::Mmc1;
pub(crate) use mmc1::Mmc1State;
pub use mmc3::Mmc3;
pub(crate) use mmc3::Mmc3State;
pub use nrom::Nrom;
pub use uxrom::Uxrom;
pub(crate) use uxrom::UxromState;

/// Common PRG read/write contract for cartridge mappers.
///
/// Addresses are CPU PRG-space addresses (`0x8000..=0xFFFF`).
///
/// Mappers intercept CPU reads and writes to provide bank switching and external hardware logic.
///
/// ## Examples
///
/// ```
/// use nes_core::mapper::Mapper;
///
/// struct DummyMapper { prg: Vec<u8> }
/// impl Mapper for DummyMapper {
///     fn read_prg(&self, addr: u16) -> u8 {
///         self.prg[(addr - 0x8000) as usize % self.prg.len()]
///     }
///     fn write_prg(&mut self, _addr: u16, _value: u8) {}
/// }
/// ```
pub trait Mapper {
    /// Reads a byte from mapped PRG space.
    fn read_prg(&self, addr: u16) -> u8;
    /// Handles mapper register writes in PRG space.
    fn write_prg(&mut self, addr: u16, value: u8);
}
