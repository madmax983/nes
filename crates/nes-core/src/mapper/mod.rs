mod nrom;
mod uxrom;

pub use nrom::Nrom;
pub use uxrom::Uxrom;

pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, value: u8);
}
