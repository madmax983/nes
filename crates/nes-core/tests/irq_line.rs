use nes_core::{Command, NesCore};

#[test]
fn apu_frame_irq_vectors_cpu_when_interrupts_enabled() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0x58, 0xEA, 0x4C, 0x01, 0xC0]); // CLI ; NOP ; JMP $C001
    core.load_cpu_bytes(0xFFFE, &[0x00, 0xC1]); // IRQ -> $C100
    core.load_cpu_bytes(0xC100, &[0xEA]);

    let mut hit_irq = false;
    for _ in 0..50_000 {
        core.execute(Command::StepCpu).unwrap();
        if core.cpu_pc() == 0xC100 {
            hit_irq = true;
            break;
        }
    }

    assert!(hit_irq, "expected frame IRQ vector at $C100 to be taken");
}

#[test]
fn apu_frame_irq_is_masked_while_interrupt_disable_set() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.load_cpu_bytes(0xFFFE, &[0x00, 0xC1]); // IRQ -> $C100
    core.load_cpu_bytes(0xC100, &[0xEA]);

    let mut hit_irq = false;
    for _ in 0..50_000 {
        core.execute(Command::StepCpu).unwrap();
        if core.cpu_pc() == 0xC100 {
            hit_irq = true;
            break;
        }
    }

    assert!(!hit_irq, "IRQ should stay masked while I flag is set");
    assert!(
        core.apu_irq_pending(),
        "frame IRQ source should still be pending when masked"
    );
}
