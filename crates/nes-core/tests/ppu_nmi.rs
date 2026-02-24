use nes_core::{Command, NesCore};

#[test]
fn nmi_triggers_when_ppuctrl_nmi_enabled() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.load_cpu_bytes(0xFFFA, &[0x10, 0xC0]); // NMI -> $C010
    core.load_cpu_bytes(0xC010, &[0xEA]);
    core.write_cpu_bus(0x2000, 0x80); // enable NMI-on-vblank

    let mut hit_nmi = false;
    for _ in 0..30_000 {
        core.execute(Command::StepCpu).unwrap();
        if core.cpu_pc() == 0xC010 {
            hit_nmi = true;
            break;
        }
    }

    assert!(hit_nmi, "expected NMI vector at $C010 to be taken");
}

#[test]
fn ppu_status_register_reflects_vblank_progression() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // stable instruction stream

    assert_eq!(core.read_memory(0x2002) & 0x80, 0);
    let mut vblank_set = false;
    for _ in 0..300 {
        core.execute(Command::StepScanline).unwrap();
        if core.read_memory(0x2002) & 0x80 != 0 {
            vblank_set = true;
            break;
        }
    }
    assert!(vblank_set, "expected vblank status bit to become set");

    let mut vblank_cleared = false;
    for _ in 0..100 {
        core.execute(Command::StepScanline).unwrap();
        if core.read_memory(0x2002) & 0x80 == 0 {
            vblank_cleared = true;
            break;
        }
    }
    assert!(vblank_cleared, "expected vblank status bit to clear");
}
