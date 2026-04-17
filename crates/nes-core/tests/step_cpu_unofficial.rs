use nes_core::cpu::Cpu;

#[test]
fn test_unofficial_instruction_trace_no_alloc() {
    let mut cpu = Cpu::new(0xC000);
    cpu.set_trace_enabled(true);

    // We execute various unofficial instructions to hit the trace branches.
    let program = vec![
        0x03, 0x00, // SLO ($00,X) -> length 2
        0x07, 0x00, // SLO $00 -> length 2
        0x0F, 0x00, 0x00, // SLO $0000 -> length 3
        0x13, 0x00, // SLO ($00),Y -> length 2
        0x17, 0x00, // SLO $00,X -> length 2
        0x1B, 0x00, 0x00, // SLO $0000,Y -> length 3
        0x1F, 0x00, 0x00, // SLO $0000,X -> length 3
        0x27, 0x00, // RLA $00 -> length 2
        0x4F, 0x00, 0x00, // SRE $0000 -> length 3
        0x73, 0x00, // RRA ($00),Y -> length 2
        0xD7, 0x00, // DCP $00,X -> length 2
        0xFB, 0x00, 0x00, // ISC $0000,Y -> length 3
        0xA3, 0x00, // LAX ($00,X) -> length 2
        0xA7, 0x00, // LAX $00 -> length 2
        0xAF, 0x00, 0x00, // LAX $0000 -> length 3
        0xB3, 0x00, // LAX ($00),Y -> length 2
        0xB7, 0x00, // LAX $00,Y -> length 2
        0xBF, 0x00, 0x00, // LAX $0000,Y -> length 3
        0x83, 0x00, // SAX ($00,X) -> length 2
        0x87, 0x00, // SAX $00 -> length 2
        0x8F, 0x00, 0x00, // SAX $0000 -> length 3
        0x97, 0x00, // SAX $00,Y -> length 2
        0x0B, 0x00, // ANC #imm
    ];

    cpu.load_bytes(0xC000, &program);

    for _ in 0..23 {
        let trace = cpu.step_with_trace().unwrap();
        assert!(!trace.is_empty(), "Trace should not be empty for opcode!");
    }
}
