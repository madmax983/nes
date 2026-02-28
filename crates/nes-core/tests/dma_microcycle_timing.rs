use nes_core::{Command, NesCore};

const LOOP_STREAM: [u8; 3] = [0xEA, 0x4C, 0x00]; // NOP ; JMP $00xx high byte patched per load address

fn load_loop(core: &mut NesCore, addr: u16) {
    let [lo, hi] = addr.to_le_bytes();
    core.load_cpu_bytes(addr, &[LOOP_STREAM[0], LOOP_STREAM[1], lo, hi]);
}

#[test]
fn oam_dma_copies_page_and_stalls_cpu() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xA9, 0x02, 0x8D, 0x14, 0x40]); // LDA #$02 ; STA $4014

    for i in 0..256_u16 {
        core.write_cpu_bus(0x0200 + i, (i as u8).wrapping_mul(3));
    }

    core.execute(Command::StepCpu).unwrap();
    let before = core.total_cycles();
    core.execute(Command::StepCpu).unwrap();
    let delta = core.total_cycles() - before;

    // STA abs is 4 cycles plus OAMDMA stall of 513/514 cycles.
    assert!(
        delta == 517 || delta == 518,
        "unexpected DMA timing delta: {delta}"
    );
    assert_eq!(core.ppu_oam_byte(0x00), 0x00);
    assert_eq!(core.ppu_oam_byte(0x01), 0x03);
    assert_eq!(core.ppu_oam_byte(0x02), 0x06);
    assert_eq!(core.ppu_oam_byte(0x7F), 0x7D);
}

#[test]
fn dmc_irq_latches_until_disabled_via_status_write() {
    let mut core = NesCore::new();
    load_loop(&mut core, 0xC100);

    core.write_cpu_bus(0xC000, 0xAA); // sample byte source
    core.write_cpu_bus(0x4010, 0x8F); // IRQ enabled, no loop, fastest rate
    core.write_cpu_bus(0x4012, 0x00); // sample addr = $C000
    core.write_cpu_bus(0x4013, 0x00); // sample length = 1 byte
    core.write_cpu_bus(0x4015, 0x10); // enable DMC

    let mut saw_irq = false;
    for _ in 0..60_000 {
        core.execute(Command::StepCpu).unwrap();
        if core.apu_dmc_irq_pending() {
            saw_irq = true;
            break;
        }
    }
    assert!(saw_irq, "expected DMC IRQ to assert");
    assert!(core.apu_dmc_fetch_count() > 0);

    let status = core.read_apu_status();
    assert_ne!(status & 0x80, 0, "APU status should report DMC IRQ");
    assert!(
        core.apu_dmc_irq_pending(),
        "reading status must not clear DMC IRQ"
    );

    core.write_cpu_bus(0x4015, 0x00);
    assert!(
        !core.apu_dmc_irq_pending(),
        "writing $4015 should clear DMC IRQ"
    );
}

#[test]
fn dmc_fetches_increase_cpu_cycle_count_via_dma_stalls() {
    let mut baseline = NesCore::new();
    load_loop(&mut baseline, 0xC000);

    for _ in 0..2_000 {
        baseline.execute(Command::StepCpu).unwrap();
    }
    let baseline_cycles = baseline.total_cycles();

    let mut with_dmc = NesCore::new();
    load_loop(&mut with_dmc, 0xC000);
    for i in 0..64_u16 {
        with_dmc.write_cpu_bus(0xC000 + i, 0x55);
    }
    with_dmc.write_cpu_bus(0x4010, 0x0F); // fastest rate, IRQ off
    with_dmc.write_cpu_bus(0x4012, 0x00);
    with_dmc.write_cpu_bus(0x4013, 0x20); // long sample
    with_dmc.write_cpu_bus(0x4015, 0x10); // enable DMC

    for _ in 0..2_000 {
        with_dmc.execute(Command::StepCpu).unwrap();
    }
    let with_dmc_cycles = with_dmc.total_cycles();

    assert!(
        with_dmc_cycles > baseline_cycles,
        "expected DMC DMA stalls to increase CPU cycle count"
    );
    assert!(with_dmc.apu_dmc_fetch_count() > 0);
}
