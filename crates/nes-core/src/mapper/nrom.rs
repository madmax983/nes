use super::Mapper;

#[derive(Debug, Clone)]
pub struct Nrom {
    prg_rom: Vec<u8>,
}

impl Nrom {
    #[must_use]
    pub fn new_32k() -> Self {
        let mut prg_rom = vec![0_u8; 32 * 1024];
        for (idx, byte) in prg_rom.iter_mut().enumerate() {
            *byte = (idx & 0xFF) as u8;
        }
        Self { prg_rom }
    }

    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn offset_for(&self, addr: u16) -> usize {
        let base = addr.saturating_sub(0x8000) as usize;
        base % self.prg_rom.len()
    }
}

impl Mapper for Nrom {
    fn read_prg(&self, addr: u16) -> u8 {
        self.prg_rom[self.offset_for(addr)]
    }

    fn write_prg(&mut self, _addr: u16, _value: u8) {
        // NROM has fixed PRG bank mapping and ignores bank-select writes.
    }
}
