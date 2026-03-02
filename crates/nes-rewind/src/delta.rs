//! Delta encoding for NES emulator state snapshots.
//!
//! Three layers of delta:
//! - [`ArrayDelta`]: byte-level runs of changes in raw arrays (RAM, CHR, etc.).
//! - [`FieldDelta`]: scalar register/timing changes between [`CoreSnapshot`]s.
//! - [`FrameDelta`]: full per-frame delta combining array and field deltas.

use nes_core::CoreSnapshot;
use nes_core::cpu::CpuSnapshot;
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

/// Scalar-field delta between two [`CoreSnapshot`]s.
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
    // TODO: mapper delta tracking -- keyframes capture full state for now.
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

        Self {
            cpu_regs,
            ppu_ctrl,
            ppu_timing,
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
        let fields = FieldDelta::compute(before, after);

        let compressed_size = [&ram_deltas, &chr_deltas, &nametable_deltas, &oam_deltas]
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

    #[test]
    fn identical_snapshots_all_none() {
        let snap = make_snapshot();
        let fd = FieldDelta::compute(&snap, &snap);
        assert!(fd.cpu_regs.is_none());
        assert!(fd.ppu_ctrl.is_none());
        assert!(fd.ppu_timing.is_none());
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
    fn frame_delta_roundtrip() {
        let before = make_snapshot();
        let mut after = before.clone();

        // Mutate several areas
        after.cpu.work_ram[0] = 0xAA;
        after.cpu.work_ram[1] = 0xBB;
        after.ppu.oam[10] = 0xCC;
        after.ppu.chr[500] = 0xDD;
        after.ppu.nametable_ram[100] = 0xEE;
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
