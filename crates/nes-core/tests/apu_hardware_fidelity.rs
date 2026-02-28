use nes_core::{Command, NesCore};

const LOOP_STREAM: [u8; 4] = [0xEA, 0x4C, 0x00, 0xC0]; // NOP ; JMP $C000

#[test]
fn frame_irq_clears_on_status_read() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);

    while !core.apu_irq_pending() && core.apu_total_cycles() < 20_000 {
        core.execute(Command::StepCpu).unwrap();
    }

    assert!(core.apu_irq_pending(), "expected frame IRQ to assert");
    let status = core.read_apu_status();
    assert_ne!(status & 0x40, 0, "status should expose frame IRQ flag");
    assert!(
        !core.apu_irq_pending(),
        "reading APU status must clear frame IRQ"
    );
}

#[test]
fn mode5_write_clocks_quarter_and_half_immediately() {
    let mut core = NesCore::new();
    core.write_cpu_bus(0x4017, 0x80); // mode 5, IRQ enabled flag irrelevant in mode 5

    assert_eq!(core.apu_quarter_frame_ticks(), 1);
    assert_eq!(core.apu_half_frame_ticks(), 1);
    assert!(!core.apu_irq_pending());
}

#[test]
fn pulse2_triangle_noise_writes_change_mixed_output() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);

    for _ in 0..8_000 {
        core.execute(Command::StepCpu).unwrap();
    }
    let baseline = core.audio_chunk_i16();

    core.write_cpu_bus(0x4015, 0x0F); // enable pulse1/pulse2/triangle/noise

    // Pulse 2
    core.write_cpu_bus(0x4004, 0x9F); // duty + envelope/volume
    core.write_cpu_bus(0x4006, 0x30); // timer low
    core.write_cpu_bus(0x4007, 0x08); // length load + timer high

    // Triangle
    core.write_cpu_bus(0x4008, 0x8F); // linear counter control + reload
    core.write_cpu_bus(0x400A, 0x40); // timer low
    core.write_cpu_bus(0x400B, 0x08); // length load + timer high

    // Noise
    core.write_cpu_bus(0x400C, 0x1F); // envelope/volume
    core.write_cpu_bus(0x400E, 0x03); // short period
    core.write_cpu_bus(0x400F, 0x08); // length load

    for _ in 0..8_000 {
        core.execute(Command::StepCpu).unwrap();
    }
    let mixed = core.audio_chunk_i16();

    assert_ne!(baseline, mixed, "channel writes should alter mixed output");
}

#[test]
fn pulse_sweep_updates_timer_reload_on_half_frame_tick() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);
    core.write_cpu_bus(0x4015, 0x01); // enable pulse1
    core.write_cpu_bus(0x4002, 0x00);
    core.write_cpu_bus(0x4003, 0x02); // timer=0x200
    core.write_cpu_bus(0x4001, 0x81); // enable, period=0, negate=0, shift=1

    while core.apu_half_frame_ticks() < 1 {
        core.execute(Command::StepCpu).unwrap();
    }

    let (p1, _) = core.apu_pulse_timer_reloads();
    assert_eq!(p1, 0x300, "positive sweep should increase pulse1 period");
}

#[test]
fn pulse1_negate_sweep_uses_extra_subtract_step() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &LOOP_STREAM);
    core.write_cpu_bus(0x4015, 0x03); // enable pulse1 + pulse2

    core.write_cpu_bus(0x4002, 0x00);
    core.write_cpu_bus(0x4003, 0x03); // pulse1 timer=0x300
    core.write_cpu_bus(0x4006, 0x00);
    core.write_cpu_bus(0x4007, 0x03); // pulse2 timer=0x300

    core.write_cpu_bus(0x4001, 0x89); // pulse1: enable, negate, shift=1
    core.write_cpu_bus(0x4005, 0x89); // pulse2: enable, negate, shift=1

    while core.apu_half_frame_ticks() < 1 {
        core.execute(Command::StepCpu).unwrap();
    }

    let (p1, p2) = core.apu_pulse_timer_reloads();
    assert_eq!(p1, 0x17F);
    assert_eq!(p2, 0x180);
}
