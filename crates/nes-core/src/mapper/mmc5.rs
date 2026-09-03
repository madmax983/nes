use alloc::{vec, vec::Vec};

use crate::rom::NametableMirroring;
use serde::{Deserialize, Serialize};

use super::Mapper;

const PRG_BANK_8K: usize = 8 * 1024;
const CHR_BANK_1K: usize = 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;
/// MMC5 supports up to 64KB of PRG-RAM (eight 8KB banks). We always allocate the
/// maximum so bank selects can never index out of range.
const PRG_RAM_BYTES: usize = 64 * 1024;
const PRG_RAM_BANKS: usize = PRG_RAM_BYTES / PRG_BANK_8K;
const PRG_RAM_BASE: u16 = 0x6000;
const PRG_RAM_END: u16 = 0x7FFF;
/// 1KB of on-chip expansion RAM (ExRAM) at CPU `$5C00..=$5FFF`.
const EXRAM_BYTES: usize = 1024;
const EXRAM_BASE: u16 = 0x5C00;
const EXRAM_END: u16 = 0x5FFF;
/// Number of audio registers ($5000..=$5015) stored (but not synthesized).
const AUDIO_REG_COUNT: usize = 0x16;

/// $5102 must hold `0b10` and $5103 `0b01` (in their low two bits) for PRG-RAM
/// writes to be enabled.
const PRG_RAM_PROTECT1_MAGIC: u8 = 0b10;
const PRG_RAM_PROTECT2_MAGIC: u8 = 0b01;

/// $5204 status bits.
const IRQ_STATUS_PENDING: u8 = 0x80;
const IRQ_STATUS_IN_FRAME: u8 = 0x40;
/// $5204 write bit 7 enables the scanline IRQ.
const IRQ_ENABLE_BIT: u8 = 0x80;

/// Number of visible scanlines. IRQ / in-frame tracking only occurs on lines
/// `0..VISIBLE_SCANLINES`.
const VISIBLE_SCANLINES: u16 = 240;

/// Source of an 8KB PRG slot: either a ROM bank or a PRG-RAM bank (index in 8KB
/// units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrgSource {
    Rom(usize),
    Ram(usize),
}

/// Mapper 5 (MMC5 / `ExROM`): the most feature-rich official Nintendo mapper.
///
/// Implemented in full: banked PRG (all four PRG modes) with ROM/RAM selection
/// and the `$5102`/`$5103` write-protect handshake, banked PRG-RAM (`$5113`),
/// banked CHR (all four CHR modes plus the `$5130` upper-bank bits), the
/// `$5205`/`$5206` 8x8→16 unsigned multiplier, ExRAM storage with the `$5104`
/// CPU access-mode rules, the fill-mode registers, and the scanline IRQ
/// (`$5203`/`$5204`).
///
/// Also implemented: the 8x16-sprite CHR "A/B" bank split. In 8x16 sprite mode
/// (PPUCTRL bit 5 set) sprite pattern fetches use the "A" register set
/// (`$5120..=$5127`) while background fetches use the "B" register set
/// (`$5128..=$512B`, mirrored across the 8KB window); when 8x16 mode is off both
/// use the "A" set. The mapper exposes the background window via
/// [`Mmc5::chr_bg_window`] and latches the 8x16 flag from the PPUCTRL byte passed
/// to [`Mmc5::on_ppu_dot`].
///
/// Deferred / stubbed (see the crate PR notes): the 5B-style audio registers
/// (`$5000..=$5015`) are stored but not synthesized; the vertical split
/// (`$5200..=$5202`) is stored but not rendered; and ExRAM-as-nametable and
/// extended-attribute rendering are not wired into the PPU (ExRAM is still fully
/// readable/writable by the CPU).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mmc5 {
    prg_rom: Vec<u8>,
    prg_bank_count_8k: usize,
    prg_ram: Vec<u8>,
    prg_ram_bank_count: usize,
    chr_data: Vec<u8>,
    chr_bank_count_1k: usize,
    chr_writable: bool,

    /// `$5100` bits 0-1: PRG banking mode (0..=3).
    prg_mode: u8,
    /// `$5101` bits 0-1: CHR banking mode (0=8KB, 1=4KB, 2=2KB, 3=1KB).
    chr_mode: u8,
    /// `$5102` bits 0-1: PRG-RAM protect handshake 1.
    prg_ram_protect1: u8,
    /// `$5103` bits 0-1: PRG-RAM protect handshake 2.
    prg_ram_protect2: u8,
    /// `$5104` bits 0-1: ExRAM mode.
    exram_mode: u8,
    /// `$5105`: nametable mapping (2 bits per quadrant).
    nametable_mapping: u8,
    /// `$5106`: fill-mode tile byte.
    fill_tile: u8,
    /// `$5107` bits 0-1: fill-mode attribute (2-bit palette).
    fill_attr: u8,
    /// `$5113` low bits: PRG-RAM bank mapped at `$6000..=$7FFF`.
    prg_ram_bank: u8,
    /// `$5114..=$5117`: PRG bank registers for `$8000/$A000/$C000/$E000`.
    prg_reg: [u8; 4],
    /// `$5120..=$5127`: CHR "A" bank set (sprite / non-8x16).
    chr_a: [u8; 8],
    /// `$5128..=$512B`: CHR "B" bank set (background in 8x16 mode).
    chr_b: [u8; 4],
    /// `$5130` bits 0-1: upper CHR bank bits.
    chr_upper: u8,
    /// `$5205` multiplier operand A.
    mult_a: u8,
    /// `$5206` multiplier operand B.
    mult_b: u8,
    /// `$5203`: scanline IRQ compare target.
    irq_scanline: u8,
    /// `$5204` bit 7: scanline IRQ enable.
    irq_enabled: bool,
    /// Latched IRQ pending flag (set on compare match, cleared by a `$5204`
    /// read).
    irq_pending: bool,
    /// "In-frame" flag (readable via `$5204` bit 6).
    in_frame: bool,
    /// `$5000..=$5015`: audio registers (stored, not synthesized).
    audio_regs: [u8; AUDIO_REG_COUNT],
    /// `$5200..=$5202`: vertical-split registers (stored, not rendered).
    split_regs: [u8; 3],
    /// 1KB ExRAM (kept on the heap so the `LoadedMapper` enum variant stays
    /// small).
    exram: Vec<u8>,

    /// Scanline counter for the IRQ. Transient PPU-timing state; excluded from
    /// the save-state snapshot.
    scanline_counter: u16,
    /// Whether rendering was enabled at the last observed PPU dot; gates ExRAM
    /// writes in modes 0/1. Transient; not snapshotted.
    rendering_active: bool,
    /// Whether the PPU is in 8x16-sprite mode (PPUCTRL bit 5), latched from the
    /// most recent [`Mmc5::on_ppu_dot`]. Selects the CHR "B" background window.
    /// Transient; not snapshotted.
    sprite_8x16: bool,
    /// Set when `sprite_8x16` changes so the core re-pushes the background CHR
    /// window to the PPU. Transient; not snapshotted.
    chr_bg_dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Mmc5State {
    pub prg_mode: u8,
    pub chr_mode: u8,
    pub prg_ram_protect1: u8,
    pub prg_ram_protect2: u8,
    pub exram_mode: u8,
    pub nametable_mapping: u8,
    pub fill_tile: u8,
    pub fill_attr: u8,
    pub prg_ram_bank: u8,
    pub prg_reg: [u8; 4],
    pub chr_a: [u8; 8],
    pub chr_b: [u8; 4],
    pub chr_upper: u8,
    pub mult_a: u8,
    pub mult_b: u8,
    pub irq_scanline: u8,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub audio_regs: [u8; AUDIO_REG_COUNT],
    pub split_regs: [u8; 3],
    pub prg_ram: Vec<u8>,
    pub exram: Vec<u8>,
}

impl Mmc5 {
    /// Creates a synthetic MMC5 mapper for tests.
    ///
    /// PRG banks are 8KB and CHR banks are 1KB; every bank is filled with its
    /// index (mirroring [`super::Mmc3::new`]) so mapping assertions are simple.
    #[must_use]
    pub fn new(prg_bank_count_8k: u8, chr_bank_count_1k: u16) -> Self {
        let prg_banks = usize::from(prg_bank_count_8k.max(4));
        let chr_banks = usize::from(chr_bank_count_1k.max(8));

        let mut prg_rom = vec![0_u8; prg_banks * PRG_BANK_8K];
        for bank in 0..prg_banks {
            let start = bank * PRG_BANK_8K;
            prg_rom[start..start + PRG_BANK_8K].fill(bank as u8);
        }

        let mut chr_data = vec![0_u8; chr_banks * CHR_BANK_1K];
        for bank in 0..chr_banks {
            let start = bank * CHR_BANK_1K;
            chr_data[start..start + CHR_BANK_1K].fill(bank as u8);
        }

        Self::assemble(prg_rom, chr_data, false, NametableMirroring::Horizontal)
    }

    /// Builds MMC5 from raw ROM metadata.
    #[must_use]
    pub fn from_prg_chr(
        mut prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        mirroring: NametableMirroring,
    ) -> Self {
        let min_prg_bytes = 4 * PRG_BANK_8K;
        if prg_rom.len() < min_prg_bytes {
            prg_rom.resize(min_prg_bytes, 0);
        }
        let prg_remainder = prg_rom.len() % PRG_BANK_8K;
        if prg_remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_8K - prg_remainder), 0);
        }

        let (mut chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        if chr_data.len() < CHR_WINDOW_BYTES {
            chr_data.resize(CHR_WINDOW_BYTES, 0);
        }
        let chr_remainder = chr_data.len() % CHR_BANK_1K;
        if chr_remainder != 0 {
            chr_data.resize(chr_data.len() + (CHR_BANK_1K - chr_remainder), 0);
        }

        Self::assemble(prg_rom, chr_data, chr_writable, mirroring)
    }

    fn assemble(
        prg_rom: Vec<u8>,
        chr_data: Vec<u8>,
        chr_writable: bool,
        mirroring: NametableMirroring,
    ) -> Self {
        let prg_bank_count_8k = (prg_rom.len() / PRG_BANK_8K).max(1);
        let chr_bank_count_1k = (chr_data.len() / CHR_BANK_1K).max(1);
        let last_bank = ((prg_bank_count_8k - 1) & 0x7F) as u8;

        Self {
            prg_rom,
            prg_bank_count_8k,
            prg_ram: vec![0_u8; PRG_RAM_BYTES],
            prg_ram_bank_count: PRG_RAM_BANKS,
            chr_data,
            chr_bank_count_1k,
            chr_writable,
            // Power-on: PRG mode 3 (four 8KB banks). Slots 0-2 default to ROM
            // bank 0 (bit 7 set = ROM) and slot 3 ($E000) to the last ROM bank
            // so the reset vector is visible before the game configures banking.
            prg_mode: 3,
            chr_mode: 0,
            prg_ram_protect1: 0,
            prg_ram_protect2: 0,
            exram_mode: 0,
            nametable_mapping: Self::default_nametable_mapping(mirroring),
            fill_tile: 0,
            fill_attr: 0,
            prg_ram_bank: 0,
            prg_reg: [0x80, 0x80, 0x80, last_bank],
            chr_a: [0, 1, 2, 3, 4, 5, 6, 7],
            chr_b: [0, 1, 2, 3],
            chr_upper: 0,
            mult_a: 0,
            mult_b: 0,
            irq_scanline: 0,
            irq_enabled: false,
            irq_pending: false,
            in_frame: false,
            audio_regs: [0; AUDIO_REG_COUNT],
            split_regs: [0; 3],
            exram: vec![0; EXRAM_BYTES],
            scanline_counter: 0,
            rendering_active: false,
            sprite_8x16: false,
            chr_bg_dirty: false,
        }
    }

    #[must_use]
    fn default_nametable_mapping(mirroring: NametableMirroring) -> u8 {
        match mirroring {
            // q0,q1,q2,q3 as 2-bit CIRAM page selects.
            NametableMirroring::Horizontal => 0b01_01_00_00, // 0,0,1,1
            NametableMirroring::Vertical => 0b01_00_01_00,   // 0,1,0,1
            NametableMirroring::OneScreenLower => 0b00_00_00_00,
            NametableMirroring::OneScreenUpper => 0b01_01_01_01,
        }
    }

    // --- Save state ---------------------------------------------------------

    #[must_use]
    pub(crate) fn state(&self) -> Mmc5State {
        Mmc5State {
            prg_mode: self.prg_mode,
            chr_mode: self.chr_mode,
            prg_ram_protect1: self.prg_ram_protect1,
            prg_ram_protect2: self.prg_ram_protect2,
            exram_mode: self.exram_mode,
            nametable_mapping: self.nametable_mapping,
            fill_tile: self.fill_tile,
            fill_attr: self.fill_attr,
            prg_ram_bank: self.prg_ram_bank,
            prg_reg: self.prg_reg,
            chr_a: self.chr_a,
            chr_b: self.chr_b,
            chr_upper: self.chr_upper,
            mult_a: self.mult_a,
            mult_b: self.mult_b,
            irq_scanline: self.irq_scanline,
            irq_enabled: self.irq_enabled,
            irq_pending: self.irq_pending,
            audio_regs: self.audio_regs,
            split_regs: self.split_regs,
            prg_ram: self.prg_ram.clone(),
            exram: self.exram.clone(),
        }
    }

    pub(crate) fn restore_state(&mut self, state: Mmc5State) {
        self.prg_mode = state.prg_mode;
        self.chr_mode = state.chr_mode;
        self.prg_ram_protect1 = state.prg_ram_protect1;
        self.prg_ram_protect2 = state.prg_ram_protect2;
        self.exram_mode = state.exram_mode;
        self.nametable_mapping = state.nametable_mapping;
        self.fill_tile = state.fill_tile;
        self.fill_attr = state.fill_attr;
        self.prg_ram_bank = state.prg_ram_bank;
        self.prg_reg = state.prg_reg;
        self.chr_a = state.chr_a;
        self.chr_b = state.chr_b;
        self.chr_upper = state.chr_upper;
        self.mult_a = state.mult_a;
        self.mult_b = state.mult_b;
        self.irq_scanline = state.irq_scanline;
        self.irq_enabled = state.irq_enabled;
        self.irq_pending = state.irq_pending;
        self.audio_regs = state.audio_regs;
        self.split_regs = state.split_regs;
        if state.prg_ram.len() == self.prg_ram.len() {
            self.prg_ram.copy_from_slice(&state.prg_ram);
        } else {
            self.prg_ram = state.prg_ram;
            self.prg_ram.resize(PRG_RAM_BYTES, 0);
        }
        if state.exram.len() == EXRAM_BYTES {
            self.exram.copy_from_slice(&state.exram);
        } else {
            self.exram = state.exram;
            self.exram.resize(EXRAM_BYTES, 0);
        }
        // Transient timing state is reset on restore.
        self.scanline_counter = 0;
        self.rendering_active = false;
        self.in_frame = false;
        self.sprite_8x16 = false;
        self.chr_bg_dirty = false;
    }

    // --- PRG ----------------------------------------------------------------

    #[must_use]
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_protect1 & 0x03 == PRG_RAM_PROTECT1_MAGIC
            && self.prg_ram_protect2 & 0x03 == PRG_RAM_PROTECT2_MAGIC
    }

    #[must_use]
    fn src_16k(reg: u8, half: usize) -> PrgSource {
        let bank = ((reg & 0x7F) as usize & !0x01) + half;
        if reg & 0x80 != 0 {
            PrgSource::Rom(bank)
        } else {
            PrgSource::Ram(bank)
        }
    }

    #[must_use]
    fn src_8k(reg: u8) -> PrgSource {
        let bank = (reg & 0x7F) as usize;
        if reg & 0x80 != 0 {
            PrgSource::Rom(bank)
        } else {
            PrgSource::Ram(bank)
        }
    }

    /// Resolves the source (ROM/RAM + 8KB bank index) for an `$8000`-space slot
    /// (0=`$8000`, 1=`$A000`, 2=`$C000`, 3=`$E000`). `$5117` (register index 3)
    /// is always ROM.
    #[must_use]
    fn prg_slot_source(&self, slot: usize) -> PrgSource {
        match self.prg_mode {
            0 => {
                // 32KB ROM at $8000-$FFFF from $5117 (low 2 bits ignored).
                let base = (self.prg_reg[3] & 0x7F) as usize & !0x03;
                PrgSource::Rom(base + slot)
            }
            1 => match slot {
                0 | 1 => Self::src_16k(self.prg_reg[1], slot & 1),
                _ => PrgSource::Rom(((self.prg_reg[3] & 0x7F) as usize & !0x01) + (slot & 1)),
            },
            2 => match slot {
                0 | 1 => Self::src_16k(self.prg_reg[1], slot & 1),
                2 => Self::src_8k(self.prg_reg[2]),
                _ => PrgSource::Rom((self.prg_reg[3] & 0x7F) as usize),
            },
            // Mode 3: four independent 8KB banks.
            _ => match slot {
                3 => PrgSource::Rom((self.prg_reg[3] & 0x7F) as usize),
                _ => Self::src_8k(self.prg_reg[slot]),
            },
        }
    }

    #[must_use]
    fn prg_ram_offset_6000(&self, addr: u16) -> usize {
        let bank = (self.prg_ram_bank as usize) % self.prg_ram_bank_count.max(1);
        bank * PRG_BANK_8K + usize::from(addr - PRG_RAM_BASE)
    }

    #[must_use]
    fn read_prg_high(&self, addr: u16) -> u8 {
        let slot = ((usize::from(addr) - 0x8000) / PRG_BANK_8K).min(3);
        let within = (usize::from(addr) - 0x8000) & 0x1FFF;
        match self.prg_slot_source(slot) {
            PrgSource::Rom(bank) => {
                let count = self.prg_bank_count_8k.max(1);
                self.prg_rom[(bank % count) * PRG_BANK_8K + within]
            }
            PrgSource::Ram(bank) => {
                let count = self.prg_ram_bank_count.max(1);
                self.prg_ram[(bank % count) * PRG_BANK_8K + within]
            }
        }
    }

    fn write_prg_high(&mut self, addr: u16, value: u8) {
        let slot = ((usize::from(addr) - 0x8000) / PRG_BANK_8K).min(3);
        let within = (usize::from(addr) - 0x8000) & 0x1FFF;
        if let PrgSource::Ram(bank) = self.prg_slot_source(slot)
            && self.prg_ram_writable()
        {
            let count = self.prg_ram_bank_count.max(1);
            self.prg_ram[(bank % count) * PRG_BANK_8K + within] = value;
        }
    }

    /// Reads the `$6000..=$7FFF` window (PRG-RAM bank `$5113`) for the CPU bus, or
    /// `None` when `addr` is outside it.
    #[must_use]
    pub fn read_prg_ram(&self, addr: u16) -> Option<u8> {
        (PRG_RAM_BASE..=PRG_RAM_END)
            .contains(&addr)
            .then(|| self.prg_ram[self.prg_ram_offset_6000(addr)])
    }

    /// Writes the `$6000..=$7FFF` window from the CPU bus (honoring the protect
    /// handshake); ignores addresses outside it.
    pub fn write_prg_ram(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) && self.prg_ram_writable() {
            let offset = self.prg_ram_offset_6000(addr);
            self.prg_ram[offset] = value;
        }
    }

    /// Reads PRG using MMC5 bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to MMC5 PRG space (only PRG-RAM windows are writable here; the
    /// mapper registers live in `$5000..=$5FFF`, handled by [`Mmc5::write_expansion`]).
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    // --- Expansion registers ($5000-$5FFF) ----------------------------------

    /// Handles a CPU write to the MMC5 register / ExRAM space (`$5000..=$5FFF`).
    pub fn write_expansion(&mut self, addr: u16, value: u8) {
        match addr {
            0x5000..=0x5015 => self.audio_regs[usize::from(addr - 0x5000)] = value,
            0x5100 => self.prg_mode = value & 0x03,
            0x5101 => self.chr_mode = value & 0x03,
            0x5102 => self.prg_ram_protect1 = value & 0x03,
            0x5103 => self.prg_ram_protect2 = value & 0x03,
            0x5104 => self.exram_mode = value & 0x03,
            0x5105 => self.nametable_mapping = value,
            0x5106 => self.fill_tile = value,
            0x5107 => self.fill_attr = value & 0x03,
            0x5113 => self.prg_ram_bank = value & 0x7F,
            0x5114..=0x5117 => self.prg_reg[usize::from(addr - 0x5114)] = value,
            0x5120..=0x5127 => self.chr_a[usize::from(addr - 0x5120)] = value,
            0x5128..=0x512B => self.chr_b[usize::from(addr - 0x5128)] = value,
            0x5130 => self.chr_upper = value & 0x03,
            0x5200..=0x5202 => self.split_regs[usize::from(addr - 0x5200)] = value,
            0x5203 => self.irq_scanline = value,
            0x5204 => self.irq_enabled = value & IRQ_ENABLE_BIT != 0,
            0x5205 => self.mult_a = value,
            0x5206 => self.mult_b = value,
            EXRAM_BASE..=EXRAM_END => self.write_exram(addr, value),
            _ => {}
        }
    }

    fn write_exram(&mut self, addr: u16, value: u8) {
        let allowed = match self.exram_mode {
            // CPU R/W.
            2 => true,
            // CPU read-only.
            3 => false,
            // Extra-nametable / extended-attribute: CPU may write only while
            // rendering is active (approximated from the last observed PPU dot).
            _ => self.rendering_active,
        };
        if allowed {
            self.exram[usize::from(addr - EXRAM_BASE)] = value;
        }
    }

    #[must_use]
    fn product(&self) -> u16 {
        u16::from(self.mult_a) * u16::from(self.mult_b)
    }

    #[must_use]
    fn status_5204(&self) -> u8 {
        let mut status = 0;
        if self.irq_pending {
            status |= IRQ_STATUS_PENDING;
        }
        if self.in_frame {
            status |= IRQ_STATUS_IN_FRAME;
        }
        status
    }

    /// Returns the byte the CPU observes when reading `$5000..=$5FFF`, or `None`
    /// for write-only / unmapped addresses (which the caller leaves as open bus).
    ///
    /// This is a pure query used to materialize the value into the CPU's flat
    /// image; the read side effect of `$5204` (clearing the pending flag) is
    /// applied separately via [`Mmc5::on_expansion_read`].
    #[must_use]
    pub fn expansion_read(&self, addr: u16) -> Option<u8> {
        match addr {
            0x5204 => Some(self.status_5204()),
            0x5205 => Some((self.product() & 0xFF) as u8),
            0x5206 => Some((self.product() >> 8) as u8),
            EXRAM_BASE..=EXRAM_END => Some(self.exram[usize::from(addr - EXRAM_BASE)]),
            _ => None,
        }
    }

    /// Applies the side effect of a CPU read of `$5000..=$5FFF`. Reading `$5204`
    /// clears the pending IRQ flag.
    pub fn on_expansion_read(&mut self, addr: u16) {
        if addr == 0x5204 {
            self.irq_pending = false;
        }
    }

    // --- CHR ----------------------------------------------------------------

    #[must_use]
    fn chr_bank_units(&self, reg: u8) -> usize {
        ((self.chr_upper as usize) << 8) | reg as usize
    }

    fn copy_chr(&self, bank_units: usize, unit_size: usize, dst_off: usize, dst: &mut [u8]) {
        let total_units = (self.chr_data.len() / unit_size).max(1);
        let src = (bank_units % total_units) * unit_size;
        dst[dst_off..dst_off + unit_size].copy_from_slice(&self.chr_data[src..src + unit_size]);
    }

    /// Returns the currently mapped 8KB CHR window built from the "A" register
    /// set (`$5120..=$5127`) per the `$5101` CHR mode. This is the sprite window
    /// in 8x16 mode and the window for everything otherwise.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        self.build_chr_window(false)
    }

    /// Returns the 8KB background CHR window built from the "B" register set
    /// (`$5128..=$512B`), or `None` when 8x16-sprite mode is inactive (in which
    /// case backgrounds share the "A" window from [`Mmc5::chr_window`]).
    ///
    /// The "B" set holds only four registers; they are mirrored across the 8KB
    /// window exactly as the corresponding "A" registers would map for the active
    /// `$5101` CHR mode (register index `i` of the "A" set becomes `i & 3` of the
    /// "B" set), matching MMC5 hardware.
    #[must_use]
    pub fn chr_bg_window(&self) -> Option<[u8; CHR_WINDOW_BYTES]> {
        self.sprite_8x16.then(|| self.build_chr_window(true))
    }

    /// Consumes the "background CHR window changed" flag (set when the 8x16-sprite
    /// mode latch flips). The core polls this per PPU dot to know when to re-push
    /// the background window to the PPU.
    pub fn take_chr_bg_dirty(&mut self) -> bool {
        core::mem::take(&mut self.chr_bg_dirty)
    }

    /// Builds a flat 8KB CHR window from either the "A" set (`use_b == false`) or
    /// the "B" set (`use_b == true`, mirrored across the window) per `$5101`.
    #[must_use]
    fn build_chr_window(&self, use_b: bool) -> [u8; CHR_WINDOW_BYTES] {
        // Register value for A-set index `idx`; the B-set has four registers and
        // is addressed by `idx & 3`.
        let reg = |idx: usize| -> u8 {
            if use_b {
                self.chr_b[idx & 3]
            } else {
                self.chr_a[idx]
            }
        };
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        match self.chr_mode {
            0 => {
                // 8KB: $5127 / $512B.
                self.copy_chr(self.chr_bank_units(reg(7)), 8 * 1024, 0, &mut window);
            }
            1 => {
                // 4KB: $5123/$5127 (A) or $512B mirrored (B).
                self.copy_chr(self.chr_bank_units(reg(3)), 4 * 1024, 0, &mut window);
                self.copy_chr(self.chr_bank_units(reg(7)), 4 * 1024, 4 * 1024, &mut window);
            }
            2 => {
                // 2KB: $5121, $5123, $5125, $5127 (A) or $5129/$512B mirrored (B).
                for (i, &reg_idx) in [1_usize, 3, 5, 7].iter().enumerate() {
                    self.copy_chr(
                        self.chr_bank_units(reg(reg_idx)),
                        2 * 1024,
                        i * 2 * 1024,
                        &mut window,
                    );
                }
            }
            _ => {
                // 1KB: $5120..=$5127 (A) or $5128..=$512B mirrored (B).
                for slot in 0..8 {
                    self.copy_chr(
                        self.chr_bank_units(reg(slot)),
                        CHR_BANK_1K,
                        slot * CHR_BANK_1K,
                        &mut window,
                    );
                }
            }
        }
        window
    }

    /// Returns `true` when mapped CHR should be writable by the PPU (CHR-RAM).
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Synchronizes writable CHR-RAM from the current PPU window, writing each
    /// mapped region back into its source bank (mirrors [`Mmc5::chr_window`]).
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }
        // Collect (dst_bank_offset, src_offset, len) then copy, to avoid holding
        // an immutable borrow of chr_data while mutating it.
        let regions = self.chr_regions();
        for (dst_off, src_off, len) in regions {
            self.chr_data[dst_off..dst_off + len].copy_from_slice(&window[src_off..src_off + len]);
        }
    }

    /// Returns `(dst_offset_in_chr_data, src_offset_in_window, len)` tuples that
    /// describe how the 8KB window maps back into `chr_data` for the active CHR
    /// mode.
    #[must_use]
    fn chr_regions(&self) -> Vec<(usize, usize, usize)> {
        let mut out = Vec::new();
        let mut push = |bank_units: usize, unit_size: usize, win_off: usize| {
            let total_units = (self.chr_data.len() / unit_size).max(1);
            let dst = (bank_units % total_units) * unit_size;
            out.push((dst, win_off, unit_size));
        };
        match self.chr_mode {
            0 => push(self.chr_bank_units(self.chr_a[7]), 8 * 1024, 0),
            1 => {
                push(self.chr_bank_units(self.chr_a[3]), 4 * 1024, 0);
                push(self.chr_bank_units(self.chr_a[7]), 4 * 1024, 4 * 1024);
            }
            2 => {
                for (i, &reg_idx) in [1_usize, 3, 5, 7].iter().enumerate() {
                    push(
                        self.chr_bank_units(self.chr_a[reg_idx]),
                        2 * 1024,
                        i * 2 * 1024,
                    );
                }
            }
            _ => {
                for slot in 0..8 {
                    push(
                        self.chr_bank_units(self.chr_a[slot]),
                        CHR_BANK_1K,
                        slot * CHR_BANK_1K,
                    );
                }
            }
        }
        out
    }

    // --- Nametable / mirroring ---------------------------------------------

    /// Returns the nametable mirroring implied by `$5105`, best-effort.
    ///
    /// Only the CIRAM page-select quadrant values (0/1) are mapped precisely;
    /// ExRAM-as-nametable (2) and fill-mode (3) quadrants are treated as CIRAM
    /// page 0 because those PPU integrations are deferred.
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        let page = |shift: u8| u8::from(((self.nametable_mapping >> shift) & 0x03) == 1);
        match (page(0), page(2), page(4), page(6)) {
            (0, 0, 1, 1) => NametableMirroring::Horizontal,
            (0, 1, 0, 1) => NametableMirroring::Vertical,
            (0, 0, 0, 0) => NametableMirroring::OneScreenLower,
            (1, 1, 1, 1) => NametableMirroring::OneScreenUpper,
            _ => NametableMirroring::Horizontal,
        }
    }

    // --- IRQ ----------------------------------------------------------------

    /// Returns the pending mapper IRQ level as seen by the CPU (pending AND
    /// enabled).
    #[must_use]
    pub fn irq_pending(&self) -> bool {
        self.irq_pending && self.irq_enabled
    }

    /// Advances the scanline IRQ / in-frame tracking for a single PPU dot.
    ///
    /// The real MMC5 detects scanlines by watching PPU fetches; this core
    /// reconstructs the boundary deterministically from `scanline`/`dot`. At the
    /// start (dot 0) of each visible rendering scanline the internal counter
    /// increments; when it equals `$5203` the pending flag latches. The in-frame
    /// flag is set while rendering visible lines and cleared at post-render /
    /// vblank. This is an accepted approximation (documented in the PR notes) and
    /// may differ from hardware by up to one scanline.
    pub fn on_ppu_dot(&mut self, scanline: u16, dot: u16, rendering_enabled: bool, ppu_ctrl: u8) {
        self.rendering_active = rendering_enabled;
        // Latch the 8x16-sprite flag (PPUCTRL bit 5). A change flips which CHR
        // "B" background window applies, so signal the core to re-push it.
        let sprite_8x16 = ppu_ctrl & 0x20 != 0;
        if sprite_8x16 != self.sprite_8x16 {
            self.sprite_8x16 = sprite_8x16;
            self.chr_bg_dirty = true;
        }
        if dot != 0 {
            return;
        }
        if rendering_enabled && scanline < VISIBLE_SCANLINES {
            if self.in_frame {
                self.scanline_counter = self.scanline_counter.wrapping_add(1);
            } else {
                self.in_frame = true;
                self.scanline_counter = 0;
            }
            if self.irq_scanline != 0 && self.scanline_counter == u16::from(self.irq_scanline) {
                self.irq_pending = true;
            }
        } else if scanline >= VISIBLE_SCANLINES {
            // Post-render / vblank: leave the frame.
            self.in_frame = false;
        }
    }
}

impl Mapper for Mmc5 {
    fn read_prg(&self, addr: u16) -> u8 {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            return self.prg_ram[self.prg_ram_offset_6000(addr)];
        }
        if addr < 0x8000 {
            return 0xFF;
        }
        self.read_prg_high(addr)
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            self.write_prg_ram(addr, value);
            return;
        }
        if addr >= 0x8000 {
            self.write_prg_high(addr, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOTS_PER_SCANLINE: u16 = 341;

    fn run_scanline(m: &mut Mmc5, scanline: u16, rendering: bool) {
        for dot in 0..DOTS_PER_SCANLINE {
            m.on_ppu_dot(scanline, dot, rendering, 0);
        }
    }

    fn enable_prg_ram_writes(m: &mut Mmc5) {
        m.write_expansion(0x5102, PRG_RAM_PROTECT1_MAGIC);
        m.write_expansion(0x5103, PRG_RAM_PROTECT2_MAGIC);
    }

    #[test]
    fn prg_mode3_four_independent_8k_banks() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5100, 3); // PRG mode 3
        m.write_expansion(0x5114, 0x80 | 2); // $8000 <- ROM bank 2
        m.write_expansion(0x5115, 0x80 | 3); // $A000 <- ROM bank 3
        m.write_expansion(0x5116, 0x80 | 4); // $C000 <- ROM bank 4
        m.write_expansion(0x5117, 5); // $E000 <- ROM bank 5 (always ROM)
        assert_eq!(m.read_prg(0x8000), 2);
        assert_eq!(m.read_prg(0xA000), 3);
        assert_eq!(m.read_prg(0xC000), 4);
        assert_eq!(m.read_prg(0xE000), 5);
    }

    #[test]
    fn prg_mode0_maps_32k_from_5117() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5100, 0); // PRG mode 0 (32KB)
        m.write_expansion(0x5117, 4); // 32KB aligned -> banks 4,5,6,7
        assert_eq!(m.read_prg(0x8000), 4);
        assert_eq!(m.read_prg(0xA000), 5);
        assert_eq!(m.read_prg(0xC000), 6);
        assert_eq!(m.read_prg(0xE000), 7);
    }

    #[test]
    fn prg_mode1_two_16k_banks() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5100, 1); // PRG mode 1 (16KB)
        m.write_expansion(0x5115, 0x80 | 2); // $8000 16KB -> banks 2,3
        m.write_expansion(0x5117, 4); // $C000 16KB -> banks 4,5
        assert_eq!(m.read_prg(0x8000), 2);
        assert_eq!(m.read_prg(0xA000), 3);
        assert_eq!(m.read_prg(0xC000), 4);
        assert_eq!(m.read_prg(0xE000), 5);
    }

    #[test]
    fn prg_mode2_16k_plus_two_8k() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5100, 2); // PRG mode 2
        m.write_expansion(0x5115, 0x80 | 2); // $8000 16KB -> banks 2,3
        m.write_expansion(0x5116, 0x80 | 6); // $C000 8KB -> bank 6
        m.write_expansion(0x5117, 7); // $E000 8KB -> bank 7
        assert_eq!(m.read_prg(0x8000), 2);
        assert_eq!(m.read_prg(0xA000), 3);
        assert_eq!(m.read_prg(0xC000), 6);
        assert_eq!(m.read_prg(0xE000), 7);
    }

    #[test]
    fn prg_ram_bank_select_and_protect() {
        let mut m = Mmc5::new(8, 8);
        // Writes blocked until the protect handshake is satisfied.
        m.write_prg_ram(0x6000, 0x11);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x00));

        enable_prg_ram_writes(&mut m);
        m.write_expansion(0x5113, 0); // PRG-RAM bank 0
        m.write_prg_ram(0x6000, 0x22);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x22));

        // A different bank sees separate storage.
        m.write_expansion(0x5113, 1);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x00));
        m.write_prg_ram(0x6000, 0x33);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x33));

        // Back to bank 0.
        m.write_expansion(0x5113, 0);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x22));
    }

    #[test]
    fn prg_ram_slot_at_8000_when_ram_selected() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5100, 3); // PRG mode 3
        enable_prg_ram_writes(&mut m);
        // $8000 <- RAM (bit7 clear) bank 0.
        m.write_expansion(0x5114, 0x00);
        m.write_prg(0x8000, 0xAB);
        assert_eq!(m.read_prg(0x8000), 0xAB);

        // Now make $8000 ROM: the write no longer lands on RAM, and reads show
        // ROM bank 1.
        m.write_expansion(0x5114, 0x80 | 1);
        assert_eq!(m.read_prg(0x8000), 1);
        m.write_prg(0x8000, 0xEE); // ignored (ROM)
        assert_eq!(m.read_prg(0x8000), 1);
    }

    #[test]
    fn chr_banking_1k_mode() {
        let mut m = Mmc5::new(8, 16);
        m.write_expansion(0x5101, 3); // 1KB mode
        for slot in 0..8u16 {
            m.write_expansion(0x5120 + slot, (slot + 8) as u8);
        }
        let window = m.chr_window();
        for slot in 0..8usize {
            assert_eq!(window[slot * CHR_BANK_1K], (slot + 8) as u8);
        }
    }

    #[test]
    fn chr_banking_8k_mode_uses_reg7() {
        let mut m = Mmc5::new(8, 16);
        m.write_expansion(0x5101, 0); // 8KB mode
        m.write_expansion(0x5127, 1); // 8KB bank 1 -> 1KB banks 8..16
        let window = m.chr_window();
        assert_eq!(window[0], 8);
        assert_eq!(window[CHR_BANK_1K], 9);
    }

    #[test]
    fn chr_banking_4k_and_2k_modes() {
        let mut m = Mmc5::new(8, 32);
        m.write_expansion(0x5101, 1); // 4KB mode
        m.write_expansion(0x5123, 1); // low 4KB -> banks 4..8
        m.write_expansion(0x5127, 2); // high 4KB -> banks 8..12
        let window = m.chr_window();
        assert_eq!(window[0], 4);
        assert_eq!(window[4 * 1024], 8);

        m.write_expansion(0x5101, 2); // 2KB mode
        m.write_expansion(0x5121, 3); // $0000 2KB -> banks 6..8
        m.write_expansion(0x5127, 5); // $1800 2KB -> banks 10..12
        let window = m.chr_window();
        assert_eq!(window[0], 6);
        assert_eq!(window[6 * 1024], 10);
    }

    #[test]
    fn chr_upper_bits_extend_bank_number() {
        // 512 1KB banks so the upper bits are observable.
        let mut m = Mmc5::new(8, 512);
        m.write_expansion(0x5101, 3); // 1KB mode
        m.write_expansion(0x5130, 1); // upper bits = 1 -> +256
        m.write_expansion(0x5120, 4); // slot0 bank = 256 + 4 = 260
        let window = m.chr_window();
        assert_eq!(window[0], 260u16 as u8);
    }

    #[test]
    fn chr_bg_window_uses_b_set_only_in_8x16_mode() {
        let mut m = Mmc5::new(8, 16);
        m.write_expansion(0x5101, 0); // CHR mode 0 (8KB)
        m.write_expansion(0x5127, 0); // A-set 8KB bank 0 (sprite window)
        m.write_expansion(0x512B, 1); // B-set 8KB bank 1 (background window)

        // 8x8 mode (PPUCTRL bit 5 clear): no separate background window.
        m.on_ppu_dot(0, 0, true, 0x00);
        assert!(m.chr_bg_window().is_none());
        assert_eq!(m.chr_window()[0], 0); // A-set bank 0

        // 8x16 mode (PPUCTRL bit 5 set): background window comes from the B-set.
        m.on_ppu_dot(0, 0, true, 0x20);
        assert!(m.take_chr_bg_dirty(), "flag latches on 8x16 transition");
        let bg = m
            .chr_bg_window()
            .expect("8x16 mode exposes a background window");
        // 8KB bank 1 begins at 1KB-bank index 8; `Mmc5::new` fills each 1KB bank
        // with its own index, so the first byte reads 8.
        assert_eq!(bg[0], 8); // B-set 8KB bank 1
        assert_eq!(m.chr_window()[0], 0); // sprite window still A-set 8KB bank 0
    }

    #[test]
    fn chr_bg_window_b_set_mirrors_in_1k_mode() {
        let mut m = Mmc5::new(8, 16);
        m.write_expansion(0x5101, 3); // CHR mode 3 (1KB)
        m.on_ppu_dot(0, 0, true, 0x20); // enable 8x16
        for reg in 0..4u16 {
            m.write_expansion(0x5128 + reg, (reg + 8) as u8); // B-set banks 8..12
        }
        let bg = m.chr_bg_window().unwrap();
        // Four B registers mirror across the eight 1KB slots.
        for slot in 0..8usize {
            assert_eq!(bg[slot * CHR_BANK_1K], (8 + (slot & 3)) as u8);
        }
    }

    #[test]
    fn multiplier_returns_product_bytes() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5205, 200);
        m.write_expansion(0x5206, 3);
        // 200 * 3 = 600 = 0x0258.
        assert_eq!(m.expansion_read(0x5205), Some(0x58));
        assert_eq!(m.expansion_read(0x5206), Some(0x02));
    }

    #[test]
    fn nametable_mapping_decode() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5105, 0b01_01_00_00);
        assert_eq!(m.mirroring(), NametableMirroring::Horizontal);
        m.write_expansion(0x5105, 0b01_00_01_00);
        assert_eq!(m.mirroring(), NametableMirroring::Vertical);
        m.write_expansion(0x5105, 0b00_00_00_00);
        assert_eq!(m.mirroring(), NametableMirroring::OneScreenLower);
        m.write_expansion(0x5105, 0b01_01_01_01);
        assert_eq!(m.mirroring(), NametableMirroring::OneScreenUpper);
    }

    #[test]
    fn fill_mode_registers_store() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5106, 0xAA);
        m.write_expansion(0x5107, 0xFF);
        assert_eq!(m.fill_tile, 0xAA);
        assert_eq!(m.fill_attr, 0x03); // masked to 2 bits
    }

    #[test]
    fn exram_mode2_cpu_read_write() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5104, 2); // CPU R/W
        m.write_expansion(0x5C00, 0x42);
        assert_eq!(m.expansion_read(0x5C00), Some(0x42));
        assert_eq!(m.expansion_read(0x5FFF), Some(0x00));
    }

    #[test]
    fn exram_mode3_is_read_only() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5104, 2);
        m.write_expansion(0x5C00, 0x11);
        m.write_expansion(0x5104, 3); // read-only
        m.write_expansion(0x5C00, 0x99); // ignored
        assert_eq!(m.expansion_read(0x5C00), Some(0x11));
    }

    #[test]
    fn exram_mode0_requires_rendering_for_writes() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5104, 0); // extra-nametable mode
        // Not rendering: write dropped.
        m.write_expansion(0x5C00, 0x55);
        assert_eq!(m.expansion_read(0x5C00), Some(0x00));
        // Simulate active rendering, then the write lands.
        m.on_ppu_dot(10, 5, true, 0);
        m.write_expansion(0x5C00, 0x55);
        assert_eq!(m.expansion_read(0x5C00), Some(0x55));
    }

    #[test]
    fn scanline_irq_fires_at_compare_and_read_clears() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5203, 5); // compare = scanline 5
        m.write_expansion(0x5204, 0x80); // enable IRQ

        // Scanlines 0..5 (counter 0..4): no IRQ yet.
        for scanline in 0..5 {
            run_scanline(&mut m, scanline, true);
            assert!(!m.irq_pending(), "no IRQ before compare (line {scanline})");
        }
        // Scanline 5 (counter == 5): IRQ latches.
        run_scanline(&mut m, 5, true);
        assert!(m.irq_pending());
        assert_eq!(m.expansion_read(0x5204).unwrap() & IRQ_STATUS_PENDING, 0x80);
        assert_eq!(
            m.expansion_read(0x5204).unwrap() & IRQ_STATUS_IN_FRAME,
            0x40
        );

        // Reading $5204 clears the pending flag.
        m.on_expansion_read(0x5204);
        assert!(!m.irq_pending());
        assert_eq!(m.expansion_read(0x5204).unwrap() & IRQ_STATUS_PENDING, 0x00);
    }

    #[test]
    fn irq_disabled_suppresses_cpu_line_but_status_shows_pending() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5203, 2);
        // IRQ NOT enabled.
        for scanline in 0..=2 {
            run_scanline(&mut m, scanline, true);
        }
        // Pending flag latches internally (visible in status) but the CPU line
        // stays low because enable is clear.
        assert!(!m.irq_pending());
        assert_eq!(m.expansion_read(0x5204).unwrap() & IRQ_STATUS_PENDING, 0x80);
    }

    #[test]
    fn in_frame_clears_during_vblank() {
        let mut m = Mmc5::new(8, 8);
        run_scanline(&mut m, 0, true);
        assert_eq!(
            m.expansion_read(0x5204).unwrap() & IRQ_STATUS_IN_FRAME,
            0x40
        );
        run_scanline(&mut m, 240, true); // post-render
        assert_eq!(
            m.expansion_read(0x5204).unwrap() & IRQ_STATUS_IN_FRAME,
            0x00
        );
    }

    #[test]
    fn state_round_trips() {
        let mut m = Mmc5::new(8, 16);
        m.write_expansion(0x5100, 3);
        m.write_expansion(0x5114, 0x80 | 2);
        m.write_expansion(0x5101, 3);
        m.write_expansion(0x5120, 9);
        m.write_expansion(0x5104, 2);
        m.write_expansion(0x5C00, 0x7E);
        enable_prg_ram_writes(&mut m);
        m.write_prg_ram(0x6000, 0x5A);
        m.write_expansion(0x5205, 12);
        m.write_expansion(0x5206, 12);
        m.write_expansion(0x5203, 30);
        m.write_expansion(0x5204, 0x80);

        let state = m.state();
        let mut restored = Mmc5::new(8, 16);
        restored.restore_state(state);

        assert_eq!(restored.read_prg(0x8000), 2);
        assert_eq!(restored.chr_window()[0], 9);
        assert_eq!(restored.expansion_read(0x5C00), Some(0x7E));
        assert_eq!(restored.read_prg_ram(0x6000), Some(0x5A));
        assert_eq!(restored.expansion_read(0x5205), Some(144));
        assert_eq!(restored.irq_scanline, 30);
        assert!(restored.irq_enabled);
    }

    #[test]
    fn chr_ram_sync_round_trip() {
        let mut m = Mmc5::from_prg_chr(vec![0; 32 * 1024], vec![], NametableMirroring::Vertical);
        assert!(m.chr_writable());
        m.write_expansion(0x5101, 3); // 1KB mode
        for slot in 0..8u16 {
            m.write_expansion(0x5120 + slot, slot as u8);
        }
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        for (i, byte) in window.iter_mut().enumerate() {
            *byte = (i / CHR_BANK_1K) as u8 + 1;
        }
        m.sync_chr_ram_from_ppu_window(&window);
        assert_eq!(m.chr_window()[0], 1);
        assert_eq!(m.chr_window()[CHR_BANK_1K], 2);
    }

    #[test]
    fn read_prg_below_8000_outside_ram_is_open_bus() {
        let m = Mmc5::new(8, 8);
        assert_eq!(m.read_prg(0x4020), 0xFF);
    }

    #[test]
    fn audio_and_split_registers_store_without_panic() {
        let mut m = Mmc5::new(8, 8);
        m.write_expansion(0x5000, 0x12);
        m.write_expansion(0x5015, 0x34);
        m.write_expansion(0x5200, 0x80);
        m.write_expansion(0x5202, 0x10);
        assert_eq!(m.audio_regs[0], 0x12);
        assert_eq!(m.audio_regs[0x15], 0x34);
        assert_eq!(m.split_regs[0], 0x80);
        assert_eq!(m.split_regs[2], 0x10);
    }
}
