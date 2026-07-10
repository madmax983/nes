//! Cartridge PRG mapper abstractions and implementations.

mod axrom;
mod camerica;
mod cnrom;
mod colordreams;
mod fme7;
mod gxrom;
mod mmc1;
mod mmc2;
mod mmc3;
mod mmc4;
mod mmc5;
mod namco108;
mod nrom;
mod uxrom;

pub use axrom::Axrom;
pub(crate) use axrom::AxromState;
pub use camerica::Camerica;
pub(crate) use camerica::CamericaState;
pub use cnrom::Cnrom;
pub(crate) use cnrom::CnromState;
pub use colordreams::ColorDreams;
pub(crate) use colordreams::ColorDreamsState;
pub use fme7::Fme7;
pub(crate) use fme7::Fme7State;
pub use gxrom::Gxrom;
pub(crate) use gxrom::GxromState;
pub use mmc1::Mmc1;
pub(crate) use mmc1::Mmc1State;
pub use mmc2::Mmc2;
pub(crate) use mmc2::Mmc2State;
pub use mmc3::Mmc3;
pub(crate) use mmc3::Mmc3State;
pub use mmc4::Mmc4;
pub(crate) use mmc4::Mmc4State;
pub use mmc5::Mmc5;
pub(crate) use mmc5::Mmc5State;
pub use namco108::Namco108;
pub(crate) use namco108::Namco108State;
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
