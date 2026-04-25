//! Delta encoding for NES emulator state snapshots.
//!
//! Three layers of delta:
//! - [`ArrayDelta`]: byte-level runs of changes in raw arrays (RAM, CHR, etc.).
//! - [`FieldDelta`]: scalar register/timing changes between `CoreSnapshot`s.
//! - [`FrameDelta`]: full per-frame delta combining array and field deltas.

use nes_core::cpu::CpuSnapshot;
use nes_core::{CoreSnapshot, MapperDelta};
use smallvec::SmallVec;

// ---------------------------------------------------------------------------
// ArrayDelta
// ---------------------------------------------------------------------------

/// A contiguous run of changed bytes at a given offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayDelta {
    /// Starting byte offset within the array.
    pub offset: u16,
    /// The new byte values for the run.
    pub data: SmallVec<[u8; 16]>,
}

/// Compute changed-byte runs between two equal-length slices.
///
/// Adjacent changed bytes are merged into a single [`ArrayDelta`].
#[must_use]
pub fn diff_array(before: &[u8], after: &[u8]) -> Vec<ArrayDelta> {
    debug_assert_eq!(before.len(), after.len());
    let mut deltas = Vec::new();
    let mut i = 0;
    let len = before.len().min(after.len());

    while i < len {
        if before[i] == after[i] {
            i += 1;
        } else {
            let start = i;
            let mut data = SmallVec::new();
            while i < len && before[i] != after[i] {
                data.push(after[i]);
                i += 1;
            }
            #[allow(clippy::cast_possible_truncation)]
            deltas.push(ArrayDelta {
                offset: start as u16,
                data,
            });
        }
    }

    deltas
}

/// Apply a set of [`ArrayDelta`]s to a mutable byte slice in-place.
pub fn apply_deltas(target: &mut [u8], deltas: &[ArrayDelta]) {
    for d in deltas {
        let start = d.offset as usize;
        let end = start + d.data.len();
        target[start..end].copy_from_slice(&d.data);
    }
}

// ---------------------------------------------------------------------------
// FieldDelta
// ---------------------------------------------------------------------------

/// Timing-related PPU fields that change frequently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuTimingDelta {
    /// Current scanline.
    pub scanline: u16,
    /// Current dot within scanline.
    pub dot: u16,
    /// Completed frame count.
    pub frame_counter: u64,
    /// Odd/even frame toggle.
    pub odd_frame: bool,
}

/// PPU address/scroll registers (the "Loopy" registers).
///
/// SMB writes these every frame for smooth side-scrolling. Without tracking
/// them in deltas, delta-reconstructed frames have stale scroll state from
/// the nearest keyframe, producing background glitches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuScrollDelta {
    /// Current VRAM address (v register).
    pub vram_addr: u16,
    /// Temporary VRAM address (t register).
    pub temp_addr: u16,
    /// Fine X scroll (3 bits).
    pub fine_x: u8,
    /// Write toggle for PPUSCROLL/PPUADDR.
    pub write_toggle: bool,
    /// Coarse X scroll approximation.
    pub scroll_x: u8,
    /// Coarse Y scroll approximation.
    pub scroll_y: u8,
    /// Buffered value for $2007 reads.
    pub read_buffer: u8,
    /// X scroll captured at VBlank start (before NMI handler overwrites it).
    pub render_scroll_x: u8,
    /// PPUCTRL captured at VBlank start.
    pub render_ctrl: u8,
}

/// Scalar-field delta between two `CoreSnapshot`s.
///
/// Each `Option` field is `None` when unchanged, `Some(new_value)` when changed.
#[derive(Debug, Clone)]
pub struct FieldDelta {
    /// CPU register state (including `work_ram`) if any register changed.
    pub cpu_regs: Option<CpuSnapshot>,
    /// PPU control registers `[ctrl, mask, status, oam_addr]` if any changed.
    pub ppu_ctrl: Option<[u8; 4]>,
    /// PPU timing fields if any changed.
    pub ppu_timing: Option<PpuTimingDelta>,
    /// PPU address/scroll (Loopy) registers if any changed.
    pub ppu_scroll: Option<PpuScrollDelta>,
    /// Mapper-specific runtime state (banking/IRQ/etc.) if any changed.
    pub mapper: Option<MapperDelta>,
}

impl FieldDelta {
    /// Compute the field-level delta between two snapshots.
    #[must_use]
    pub fn compute(before: &CoreSnapshot, after: &CoreSnapshot) -> Self {
        let cpu_regs = (before.cpu != after.cpu).then_some(after.cpu);

        let ppu_ctrl = {
            let b = [
                before.ppu.ctrl,
                before.ppu.mask,
                before.ppu.status,
                before.ppu.oam_addr,
            ];
            let a = [
                after.ppu.ctrl,
                after.ppu.mask,
                after.ppu.status,
                after.ppu.oam_addr,
            ];
            if b == a { None } else { Some(a) }
        };

        let ppu_timing = {
            let changed = before.ppu.scanline != after.ppu.scanline
                || before.ppu.dot != after.ppu.dot
                || before.ppu.frame_counter != after.ppu.frame_counter
                || before.ppu.odd_frame != after.ppu.odd_frame;
            if changed {
                Some(PpuTimingDelta {
                    scanline: after.ppu.scanline,
                    dot: after.ppu.dot,
                    frame_counter: after.ppu.frame_counter,
                    odd_frame: after.ppu.odd_frame,
                })
            } else {
                None
            }
        };

        let ppu_scroll = {
            let changed = before.ppu.vram_addr != after.ppu.vram_addr
                || before.ppu.temp_addr != after.ppu.temp_addr
                || before.ppu.fine_x != after.ppu.fine_x
                || before.ppu.write_toggle != after.ppu.write_toggle
                || before.ppu.scroll_x != after.ppu.scroll_x
                || before.ppu.scroll_y != after.ppu.scroll_y
                || before.ppu.read_buffer != after.ppu.read_buffer
                || before.ppu.render_scroll_x != after.ppu.render_scroll_x
                || before.ppu.render_ctrl != after.ppu.render_ctrl;
            if changed {
                Some(PpuScrollDelta {
                    vram_addr: after.ppu.vram_addr,
                    temp_addr: after.ppu.temp_addr,
                    fine_x: after.ppu.fine_x,
                    write_toggle: after.ppu.write_toggle,
                    scroll_x: after.ppu.scroll_x,
                    scroll_y: after.ppu.scroll_y,
                    read_buffer: after.ppu.read_buffer,
                    render_scroll_x: after.ppu.render_scroll_x,
                    render_ctrl: after.ppu.render_ctrl,
                })
            } else {
                None
            }
        };

        let mapper = before.mapper_delta(after);

        Self {
            cpu_regs,
            ppu_ctrl,
            ppu_timing,
            ppu_scroll,
            mapper,
        }
    }

    /// Apply this field delta to a mutable snapshot in-place.
    pub fn apply(&self, target: &mut CoreSnapshot) {
        if let Some(cpu) = &self.cpu_regs {
            target.cpu = *cpu;
        }
        if let Some([ctrl, mask, status, oam_addr]) = self.ppu_ctrl {
            target.ppu.ctrl = ctrl;
            target.ppu.mask = mask;
            target.ppu.status = status;
            target.ppu.oam_addr = oam_addr;
        }
        if let Some(timing) = &self.ppu_timing {
            target.ppu.scanline = timing.scanline;
            target.ppu.dot = timing.dot;
            target.ppu.frame_counter = timing.frame_counter;
            target.ppu.odd_frame = timing.odd_frame;
        }
        if let Some(scroll) = &self.ppu_scroll {
            target.ppu.vram_addr = scroll.vram_addr;
            target.ppu.temp_addr = scroll.temp_addr;
            target.ppu.fine_x = scroll.fine_x;
            target.ppu.write_toggle = scroll.write_toggle;
            target.ppu.scroll_x = scroll.scroll_x;
            target.ppu.scroll_y = scroll.scroll_y;
            target.ppu.read_buffer = scroll.read_buffer;
            target.ppu.render_scroll_x = scroll.render_scroll_x;
            target.ppu.render_ctrl = scroll.render_ctrl;
        }
        if let Some(mapper) = &self.mapper {
            target.apply_mapper_delta(mapper);
        }
    }
}

// ---------------------------------------------------------------------------
// FrameDelta
// ---------------------------------------------------------------------------

/// Complete per-frame delta combining array-level and field-level changes.
#[derive(Debug, Clone)]
pub struct FrameDelta {
    /// Logical frame identifier (typically `ppu.frame_counter` of the *after* snapshot).
    pub frame_id: u64,
    /// Changed runs in CPU work RAM (2KB).
    pub ram_deltas: Vec<ArrayDelta>,
    /// Changed runs in CHR memory (8KB).
    pub chr_deltas: Vec<ArrayDelta>,
    /// Changed runs in nametable RAM (2KB).
    pub nametable_deltas: Vec<ArrayDelta>,
    /// Changed runs in OAM (256B).
    pub oam_deltas: Vec<ArrayDelta>,
    /// Changed runs in palette RAM (32B).
    pub palette_deltas: Vec<ArrayDelta>,
    /// Scalar field changes.
    pub fields: FieldDelta,
    /// Approximate compressed byte size of this delta (sum of all data runs).
    compressed_size: u32,
}

impl FrameDelta {
    /// Compute a full frame delta between two snapshots.
    #[must_use]
    pub fn compute(before: &CoreSnapshot, after: &CoreSnapshot) -> Self {
        let ram_deltas = diff_array(&before.cpu.work_ram, &after.cpu.work_ram);
        let chr_deltas = diff_array(&before.ppu.chr, &after.ppu.chr);
        let nametable_deltas = diff_array(&before.ppu.nametable_ram, &after.ppu.nametable_ram);
        let oam_deltas = diff_array(&before.ppu.oam, &after.ppu.oam);
        let palette_deltas = diff_array(&before.ppu.palette_ram, &after.ppu.palette_ram);
        let fields = FieldDelta::compute(before, after);

        let compressed_size = [
            &ram_deltas,
            &chr_deltas,
            &nametable_deltas,
            &oam_deltas,
            &palette_deltas,
        ]
        .iter()
        .flat_map(|ds| ds.iter())
        .map(|d| {
            #[allow(clippy::cast_possible_truncation)]
            let size = d.data.len() as u32;
            size
        })
        .sum();

        Self {
            frame_id: after.ppu.frame_counter,
            ram_deltas,
            chr_deltas,
            nametable_deltas,
            oam_deltas,
            palette_deltas,
            fields,
            compressed_size,
        }
    }

    /// Approximate byte cost of this delta.
    #[must_use]
    pub fn compressed_size(&self) -> u32 {
        self.compressed_size
    }

    /// Apply this frame delta to a mutable snapshot in-place.
    pub fn apply(&self, target: &mut CoreSnapshot) {
        apply_deltas(&mut target.cpu.work_ram, &self.ram_deltas);
        apply_deltas(&mut target.ppu.chr, &self.chr_deltas);
        apply_deltas(&mut target.ppu.nametable_ram, &self.nametable_deltas);
        apply_deltas(&mut target.ppu.oam, &self.oam_deltas);
        apply_deltas(&mut target.ppu.palette_ram, &self.palette_deltas);
        self.fields.apply(target);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;

    // -----------------------------------------------------------------------
    // ArrayDelta tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_arrays_produce_no_deltas() {
        let a: [u8; 0] = [];
        let b: [u8; 0] = [];
        assert!(diff_array(&a, &b).is_empty());
    }

    #[test]
    fn identical_arrays_produce_no_deltas() {
        let a = [1, 2, 3, 4, 5];
        assert!(diff_array(&a, &a).is_empty());
    }

    #[test]
    fn single_byte_change() {
        let before = [0, 0, 0, 0];
        let after = [0, 0, 42, 0];
        let deltas = diff_array(&before, &after);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].offset, 2);
        assert_eq!(deltas[0].data.as_slice(), &[42]);
    }

    #[test]
    fn adjacent_changes_merged() {
        let before = [0, 0, 0, 0, 0];
        let after = [0, 1, 2, 3, 0];
        let deltas = diff_array(&before, &after);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].offset, 1);
        assert_eq!(deltas[0].data.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn non_adjacent_changes_separate_runs() {
        let before = [0, 0, 0, 0, 0];
        let after = [1, 0, 0, 0, 2];
        let deltas = diff_array(&before, &after);
        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0].offset, 0);
        assert_eq!(deltas[0].data.as_slice(), &[1]);
        assert_eq!(deltas[1].offset, 4);
        assert_eq!(deltas[1].data.as_slice(), &[2]);
    }

    #[test]
    fn roundtrip_diff_then_apply() {
        let before = [10, 20, 30, 40, 50];
        let after = [10, 99, 30, 88, 77];
        let deltas = diff_array(&before, &after);
        let mut buf = before;
        apply_deltas(&mut buf, &deltas);
        assert_eq!(buf, after);
    }

    #[test]
    fn large_array_roundtrip_2kb() {
        let mut before = [0u8; 2048];
        let mut after = [0u8; 2048];
        // Scatter some changes
        for i in (0..2048).step_by(7) {
            after[i] = (i & 0xFF) as u8;
        }
        before[100] = 0xFF;
        after[100] = 0xAA;

        let deltas = diff_array(&before, &after);
        assert!(!deltas.is_empty());

        let mut buf = before;
        apply_deltas(&mut buf, &deltas);
        assert_eq!(buf, after);
    }

    // -----------------------------------------------------------------------
    // FieldDelta tests
    // -----------------------------------------------------------------------

    fn make_snapshot() -> CoreSnapshot {
        let core = NesCore::new();
        core.save_state()
    }

    fn sample_ines(mapper_id: u8, prg_banks: u8) -> Vec<u8> {
        let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024];
        rom[0] = 0x4E;
        rom[1] = 0x45;
        rom[2] = 0x53;
        rom[3] = 0x1A;
        rom[4] = prg_banks;
        rom[5] = 0;
        rom[6] = (mapper_id & 0x0F) << 4;
        rom[7] = mapper_id & 0xF0;
        rom
    }

    fn make_mmc3_snapshot() -> CoreSnapshot {
        let rom = sample_ines(4, 4);
        let mut core = NesCore::new();
        core.load_ines_rom(&rom).unwrap();
        core.save_state()
    }

    fn make_cnrom_chr_ram_snapshot() -> CoreSnapshot {
        let rom = sample_ines(3, 1);
        let mut core = NesCore::new();
        core.load_ines_rom(&rom).unwrap();
        core.save_state()
    }

    fn write_ppu_data(core: &mut NesCore, addr: u16, data: &[u8]) {
        core.write_cpu_bus(0x2006, (addr >> 8) as u8);
        core.write_cpu_bus(0x2006, addr as u8);
        for &byte in data {
            core.write_cpu_bus(0x2007, byte);
        }
    }

    #[test]
    fn ppu_timing_change_single_field() {
        let before = make_snapshot();

        let mut after = before.clone();
        after.ppu.scanline = before.ppu.scanline.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_timing.is_some());

        let mut after = before.clone();
        after.ppu.dot = before.ppu.dot.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_timing.is_some());

        let mut after = before.clone();
        after.ppu.frame_counter = before.ppu.frame_counter.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_timing.is_some());

        let mut after = before.clone();
        after.ppu.odd_frame = !before.ppu.odd_frame;
        assert!(FieldDelta::compute(&before, &after).ppu_timing.is_some());
    }

    #[test]
    fn ppu_scroll_change_single_fields() {
        let before = make_snapshot();

        let mut after = before.clone();
        after.ppu.vram_addr = before.ppu.vram_addr.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.temp_addr = before.ppu.temp_addr.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.fine_x = before.ppu.fine_x.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.write_toggle = !before.ppu.write_toggle;
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.scroll_x = before.ppu.scroll_x.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.scroll_y = before.ppu.scroll_y.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.read_buffer = before.ppu.read_buffer.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.render_scroll_x = before.ppu.render_scroll_x.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());

        let mut after = before.clone();
        after.ppu.render_ctrl = before.ppu.render_ctrl.wrapping_add(1);
        assert!(FieldDelta::compute(&before, &after).ppu_scroll.is_some());
    }
    #[test]
    fn identical_snapshots_all_none() {
        let snap = make_snapshot();
        let fd = FieldDelta::compute(&snap, &snap);
        assert!(fd.cpu_regs.is_none());
        assert!(fd.ppu_ctrl.is_none());
        assert!(fd.ppu_timing.is_none());
        assert!(fd.ppu_scroll.is_none());
        assert!(fd.mapper.is_none());
    }

    #[test]
    fn mapper_irq_state_roundtrip_requires_mapper_delta() {
        let before = make_mmc3_snapshot();

        let mut core = NesCore::new();
        core.load_state(&before);
        core.write_cpu_bus(0xC000, 0x03);
        core.write_cpu_bus(0xC001, 0x00);
        core.write_cpu_bus(0xE001, 0x00);
        let after = core.save_state();

        let fd = FrameDelta::compute(&before, &after);
        let mut target = before.clone();
        fd.apply(&mut target);

        assert_eq!(target, after);
    }

    #[test]
    fn chr_ram_writes_preserve_mapper_backing_without_register_change() {
        let before = make_cnrom_chr_ram_snapshot();

        let mut core = NesCore::new();
        core.load_state(&before);
        write_ppu_data(&mut core, 0x0000, &[0x5A]);
        let after = core.save_state();

        let fd = FrameDelta::compute(&before, &after);
        assert!(fd.fields.mapper.is_some());

        let mut target = before.clone();
        fd.apply(&mut target);

        let mut restored = NesCore::new();
        restored.load_state(&target);
        assert_eq!(restored.save_state().ppu.chr[0], 0x5A);
    }

    #[test]
    fn ppu_scroll_change_detected() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.vram_addr = 0x1234;
        after.ppu.fine_x = 5;
        after.ppu.scroll_x = 80;

        let fd = FieldDelta::compute(&before, &after);
        assert!(fd.ppu_scroll.is_some());
        let scroll = fd.ppu_scroll.unwrap();
        assert_eq!(scroll.vram_addr, 0x1234);
        assert_eq!(scroll.fine_x, 5);
        assert_eq!(scroll.scroll_x, 80);
    }

    #[test]
    fn ppu_scroll_delta_apply_restores_scroll() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.vram_addr = 0x2000;
        after.ppu.temp_addr = 0x3FFF;
        after.ppu.fine_x = 7;
        after.ppu.write_toggle = true;
        after.ppu.scroll_x = 120;
        after.ppu.scroll_y = 30;
        after.ppu.read_buffer = 0xAB;

        let fd = FieldDelta::compute(&before, &after);
        let mut target = before.clone();
        fd.apply(&mut target);

        assert_eq!(target.ppu.vram_addr, 0x2000);
        assert_eq!(target.ppu.temp_addr, 0x3FFF);
        assert_eq!(target.ppu.fine_x, 7);
        assert!(target.ppu.write_toggle);
        assert_eq!(target.ppu.scroll_x, 120);
        assert_eq!(target.ppu.scroll_y, 30);
        assert_eq!(target.ppu.read_buffer, 0xAB);
    }

    #[test]
    fn cpu_register_change_detected() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.cpu.a = 0x42;
        after.cpu.pc = 0xBEEF;

        let fd = FieldDelta::compute(&before, &after);
        assert!(fd.cpu_regs.is_some());
        let cpu = fd.cpu_regs.unwrap();
        assert_eq!(cpu.a, 0x42);
        assert_eq!(cpu.pc, 0xBEEF);
    }

    #[test]
    fn ppu_ctrl_change_detected() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.ctrl = 0x80;

        let fd = FieldDelta::compute(&before, &after);
        assert!(fd.ppu_ctrl.is_some());
        assert_eq!(fd.ppu_ctrl.unwrap()[0], 0x80);
    }

    #[test]
    fn ppu_timing_change_detected() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.frame_counter = 999;
        after.ppu.scanline = 120;

        let fd = FieldDelta::compute(&before, &after);
        assert!(fd.ppu_timing.is_some());
        let t = fd.ppu_timing.unwrap();
        assert_eq!(t.frame_counter, 999);
        assert_eq!(t.scanline, 120);
    }

    #[test]
    fn field_delta_apply_restores_changes() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.cpu.a = 0x77;
        after.ppu.ctrl = 0x88;
        after.ppu.frame_counter = 42;

        let fd = FieldDelta::compute(&before, &after);

        let mut target = before.clone();
        fd.apply(&mut target);
        assert_eq!(target.cpu.a, 0x77);
        assert_eq!(target.ppu.ctrl, 0x88);
        assert_eq!(target.ppu.frame_counter, 42);
    }

    // -----------------------------------------------------------------------
    // FrameDelta tests
    // -----------------------------------------------------------------------

    #[test]
    fn identical_snapshots_zero_compressed_size() {
        let snap = make_snapshot();
        let fd = FrameDelta::compute(&snap, &snap);
        assert_eq!(fd.compressed_size(), 0);
    }

    #[test]
    fn palette_change_tracked_in_frame_delta() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.palette_ram[0] = 0x0F; // black background
        after.ppu.palette_ram[1] = 0x16; // red

        let fd = FrameDelta::compute(&before, &after);
        assert!(!fd.palette_deltas.is_empty());
    }

    #[test]
    fn palette_delta_roundtrip() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.palette_ram[0] = 0x0F;
        after.ppu.palette_ram[16] = 0x30; // white sprite palette

        let fd = FrameDelta::compute(&before, &after);
        let mut target = before.clone();
        fd.apply(&mut target);

        assert_eq!(target.ppu.palette_ram[0], 0x0F);
        assert_eq!(target.ppu.palette_ram[16], 0x30);
    }

    #[test]
    fn frame_delta_roundtrip() {
        let before = make_snapshot();
        let mut after = before.clone();

        // Mutate several areas
        after.cpu.work_ram[0] = 0xAA;
        after.cpu.work_ram[1] = 0xBB;
        after.ppu.oam[10] = 0xCC;
        after.ppu.chr[500] = 0xDD;
        after.ppu.nametable_ram[100] = 0xEE;
        after.ppu.palette_ram[4] = 0x16;
        after.ppu.vram_addr = 0x1234;
        after.ppu.scroll_x = 100;
        after.cpu.a = 0x42;
        after.ppu.frame_counter = 100;

        let fd = FrameDelta::compute(&before, &after);
        assert!(fd.compressed_size() > 0);

        let mut target = before.clone();
        fd.apply(&mut target);

        assert_eq!(target.cpu.work_ram[0], 0xAA);
        assert_eq!(target.cpu.work_ram[1], 0xBB);
        assert_eq!(target.ppu.oam[10], 0xCC);
        assert_eq!(target.ppu.chr[500], 0xDD);
        assert_eq!(target.ppu.nametable_ram[100], 0xEE);
        assert_eq!(target.ppu.palette_ram[4], 0x16);
        assert_eq!(target.ppu.vram_addr, 0x1234);
        assert_eq!(target.ppu.scroll_x, 100);
        assert_eq!(target.cpu.a, 0x42);
        assert_eq!(target.ppu.frame_counter, 100);
    }

    #[test]
    fn frame_delta_frame_id_is_after_frame_counter() {
        let before = make_snapshot();
        let mut after = before.clone();
        after.ppu.frame_counter = 7;

        let fd = FrameDelta::compute(&before, &after);
        assert_eq!(fd.frame_id, 7);
    }
}
