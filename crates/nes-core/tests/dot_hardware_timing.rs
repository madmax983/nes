use nes_core::{Command, NesCore};

const LOOP_STREAM: [u8; 4] = [0xEA, 0x4C, 0x00, 0xC0]; // NOP; JMP $C000

#[test]
fn ppu_vblank_edges_are_dot_exact() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);
    core.write_cpu_bus(0x2000, 0x80); // NMI on vblank

    let mut saw_set = false;
    let mut saw_clear = false;
    let mut prev_vblank = false;

    for _ in 0..80_000 {
        core.execute(Command::StepCpu).unwrap();
        let vblank = core.read_memory(0x2002) & 0x80 != 0;

        if !prev_vblank && vblank {
            assert_eq!(core.ppu_scanline(), 241);
            assert!(core.ppu_dot() >= 1);
            assert!(core.ppu_dot() <= 32);
            saw_set = true;
        }

        if prev_vblank && !vblank {
            assert_eq!(core.ppu_scanline(), 261);
            assert!(core.ppu_dot() >= 1);
            assert!(core.ppu_dot() <= 32);
            saw_clear = true;
            break;
        }

        prev_vblank = vblank;
    }

    assert!(saw_set, "expected exact vblank set edge");
    assert!(saw_clear, "expected exact vblank clear edge");
}

#[test]
fn step_frame_advances_one_ppu_frame_with_odd_frame_shortening() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);
    core.write_cpu_bus(0x2001, 0x18); // enable rendering for odd-frame skip behavior

    let ppu0 = core.ppu_total_cycles();
    let f0 = core.ppu_frame_counter();
    core.execute(Command::StepFrame).unwrap();
    let ppu1 = core.ppu_total_cycles();
    let f1 = core.ppu_frame_counter();
    assert_eq!(f1, f0 + 1);

    core.execute(Command::StepFrame).unwrap();
    let ppu2 = core.ppu_total_cycles();
    let f2 = core.ppu_frame_counter();
    assert_eq!(f2, f1 + 1);

    let frame0 = ppu1 - ppu0;
    let frame1 = ppu2 - ppu1;
    assert!(frame0 >= 341 * 262);
    assert!(frame0 <= 341 * 262 + 9);
    assert!(frame1 <= frame0);
    assert!(frame0.saturating_sub(frame1) <= 9);
}

#[test]
fn apu_frame_sequencer_ticks_on_hardware_cycle_boundaries() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);

    while core.apu_total_cycles() < 14_916 {
        core.execute(Command::StepCpu).unwrap();
    }

    assert_eq!(core.apu_quarter_frame_ticks(), 4);
    assert_eq!(core.apu_half_frame_ticks(), 2);
    assert!(core.apu_irq_pending());
}
