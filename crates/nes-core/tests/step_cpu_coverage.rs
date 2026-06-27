use nes_core::cpu::Cpu;

#[derive(Default)]
struct TestCase {
    name: &'static str,
    setup_code: &'static [u8],
    setup_memory: Vec<(u16, u8)>,
    setup_a: u8,
    setup_x: u8,
    setup_y: u8,
    expected_a: u8,
    expected_x: u8,
    expected_y: u8,
    expected_memory: Vec<(u16, u8)>,
    expected_trace_contains: &'static str,
}

fn run_test_case(tc: &TestCase) {
    let mut cpu = Cpu::new(0xC000);

    // Set initial memory
    for &(addr, val) in &tc.setup_memory {
        cpu.write_byte(addr, val);
    }

    // Set up registers by pushing prep instructions
    // To set A, X, Y we can use immediate loads:
    // LDA #imm = A9 imm
    // LDX #imm = A2 imm
    // LDY #imm = A0 imm
    let mut code = vec![
        0xA9, tc.setup_a, // LDA #setup_a
        0xA2, tc.setup_x, // LDX #setup_x
        0xA0, tc.setup_y, // LDY #setup_y
    ];
    code.extend_from_slice(tc.setup_code);
    cpu.load_bytes(0xC000, &code);

    // Run prep instructions
    cpu.step_with_trace().unwrap(); // LDA
    cpu.step_with_trace().unwrap(); // LDX
    cpu.step_with_trace().unwrap(); // LDY

    // Run target instruction
    let trace = cpu.step_with_trace().unwrap();

    assert!(
        trace.contains(tc.expected_trace_contains),
        "Test '{}' failed: trace `{}` did not contain `{}`",
        tc.name,
        trace,
        tc.expected_trace_contains
    );

    assert_eq!(
        cpu.a(),
        tc.expected_a,
        "Test '{}' failed on A register",
        tc.name
    );
    assert_eq!(
        cpu.x(),
        tc.expected_x,
        "Test '{}' failed on X register",
        tc.name
    );
    assert_eq!(
        cpu.y(),
        tc.expected_y,
        "Test '{}' failed on Y register",
        tc.name
    );

    for &(addr, expected_val) in &tc.expected_memory {
        let actual_val = cpu.read_byte(addr);
        assert_eq!(
            actual_val, expected_val,
            "Test '{}' failed on memory at ${:04X}",
            tc.name, addr
        );
    }
}

#[test]
fn cpu_opcode_coverage() {
    let cases = vec![
        TestCase {
            name: "ORA ($zp,X)",
            setup_code: &[0x01, 0x10],
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0xAA)],
            setup_a: 0x55,
            setup_x: 0x05,
            expected_a: 0xFF,
            expected_x: 0x05,
            expected_trace_contains: "ORA ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "ORA $zp",
            setup_code: &[0x05, 0x10],
            setup_memory: vec![(0x10, 0xAA)],
            setup_a: 0x55,
            expected_a: 0xFF,
            expected_trace_contains: "ORA $10",
            ..Default::default()
        },
        TestCase {
            name: "ASL $zp",
            setup_code: &[0x06, 0x10],
            setup_memory: vec![(0x10, 0x81)],
            expected_memory: vec![(0x10, 0x02)],
            expected_trace_contains: "ASL $10",
            ..Default::default()
        },
        TestCase {
            name: "ORA #$imm",
            setup_code: &[0x09, 0xAA],
            setup_a: 0x55,
            expected_a: 0xFF,
            expected_trace_contains: "ORA #$AA",
            ..Default::default()
        },
        TestCase {
            name: "ASL A",
            setup_code: &[0x0A],
            setup_a: 0x81,
            expected_a: 0x02,
            expected_trace_contains: "ASL A",
            ..Default::default()
        },
        TestCase {
            name: "ORA $addr",
            setup_code: &[0x0D, 0x34, 0x12],
            setup_memory: vec![(0x1234, 0xAA)],
            setup_a: 0x55,
            expected_a: 0xFF,
            expected_trace_contains: "ORA $1234",
            ..Default::default()
        },
        TestCase {
            name: "ASL $addr",
            setup_code: &[0x0E, 0x34, 0x12],
            setup_memory: vec![(0x1234, 0x81)],
            expected_memory: vec![(0x1234, 0x02)],
            expected_trace_contains: "ASL $1234",
            ..Default::default()
        },
        TestCase {
            name: "ORA ($zp),Y",
            setup_code: &[0x11, 0x10],
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0xAA)],
            setup_a: 0x55,
            setup_y: 0x06,
            expected_a: 0xFF,
            expected_y: 0x06,
            expected_trace_contains: "ORA ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "ORA $zp,X",
            setup_code: &[0x15, 0x10],
            setup_memory: vec![(0x15, 0xAA)],
            setup_a: 0x55,
            setup_x: 0x05,
            expected_a: 0xFF,
            expected_x: 0x05,
            expected_trace_contains: "ORA $10,X",
            ..Default::default()
        },
        TestCase {
            name: "ASL $zp,X",
            setup_code: &[0x16, 0x10],
            setup_memory: vec![(0x15, 0x81)],
            setup_x: 0x05,
            expected_x: 0x05,
            expected_memory: vec![(0x15, 0x02)],
            expected_trace_contains: "ASL $10,X",
            ..Default::default()
        },
        TestCase {
            name: "ORA $addr,Y",
            setup_code: &[0x19, 0x34, 0x12],
            setup_memory: vec![(0x123A, 0xAA)],
            setup_a: 0x55,
            setup_y: 0x06,
            expected_a: 0xFF,
            expected_y: 0x06,
            expected_trace_contains: "ORA $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "ORA $addr,X",
            setup_code: &[0x1D, 0x34, 0x12],
            setup_memory: vec![(0x1239, 0xAA)],
            setup_a: 0x55,
            setup_x: 0x05,
            expected_a: 0xFF,
            expected_x: 0x05,
            expected_trace_contains: "ORA $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "ASL $addr,X",
            setup_code: &[0x1E, 0x34, 0x12],
            setup_memory: vec![(0x1239, 0x81)],
            setup_x: 0x05,
            expected_x: 0x05,
            expected_memory: vec![(0x1239, 0x02)],
            expected_trace_contains: "ASL $1234,X",
            ..Default::default()
        },
        // AND operations
        TestCase {
            name: "AND ($zp,X)",
            setup_code: &[0x21, 0x10],
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0x0F)],
            setup_a: 0x33,
            setup_x: 0x05,
            expected_a: 0x03,
            expected_x: 0x05,
            expected_trace_contains: "AND ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "AND $zp",
            setup_code: &[0x25, 0x10],
            setup_memory: vec![(0x10, 0x0F)],
            setup_a: 0x33,
            expected_a: 0x03,
            expected_trace_contains: "AND $10",
            ..Default::default()
        },
        TestCase {
            name: "ROL $zp",
            setup_code: &[0x26, 0x10],
            setup_memory: vec![(0x10, 0x81)], // carry flag comes from something, let's assume it's 0 because we didn't set it
            expected_memory: vec![(0x10, 0x02)], // 0x81 << 1 | C(0)
            expected_trace_contains: "ROL $10",
            ..Default::default()
        },
        TestCase {
            name: "AND #$imm",
            setup_code: &[0x29, 0x0F],
            setup_a: 0x33,
            expected_a: 0x03,
            expected_trace_contains: "AND #$0F",
            ..Default::default()
        },
        TestCase {
            name: "ROL A",
            setup_code: &[0x2A],
            setup_a: 0x81,
            expected_a: 0x02,
            expected_trace_contains: "ROL A",
            ..Default::default()
        },
        TestCase {
            name: "AND $addr",
            setup_code: &[0x2D, 0x34, 0x12],
            setup_memory: vec![(0x1234, 0x0F)],
            setup_a: 0x33,
            expected_a: 0x03,
            expected_trace_contains: "AND $1234",
            ..Default::default()
        },
        TestCase {
            name: "ROL $addr",
            setup_code: &[0x2E, 0x34, 0x12],
            setup_memory: vec![(0x1234, 0x81)],
            expected_memory: vec![(0x1234, 0x02)],
            expected_trace_contains: "ROL $1234",
            ..Default::default()
        },
        TestCase {
            name: "AND ($zp),Y",
            setup_code: &[0x31, 0x10],
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0x0F)],
            setup_a: 0x33,
            setup_y: 0x06,
            expected_a: 0x03,
            expected_y: 0x06,
            expected_trace_contains: "AND ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "AND $zp,X",
            setup_code: &[0x35, 0x10],
            setup_memory: vec![(0x15, 0x0F)],
            setup_a: 0x33,
            setup_x: 0x05,
            expected_a: 0x03,
            expected_x: 0x05,
            expected_trace_contains: "AND $10,X",
            ..Default::default()
        },
        TestCase {
            name: "ROL $zp,X",
            setup_code: &[0x36, 0x10],
            setup_memory: vec![(0x15, 0x81)],
            setup_x: 0x05,
            expected_x: 0x05,
            expected_memory: vec![(0x15, 0x02)],
            expected_trace_contains: "ROL $10,X",
            ..Default::default()
        },
        TestCase {
            name: "AND $addr,Y",
            setup_code: &[0x39, 0x34, 0x12],
            setup_memory: vec![(0x123A, 0x0F)],
            setup_a: 0x33,
            setup_y: 0x06,
            expected_a: 0x03,
            expected_y: 0x06,
            expected_trace_contains: "AND $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "AND $addr,X",
            setup_code: &[0x3D, 0x34, 0x12],
            setup_memory: vec![(0x1239, 0x0F)],
            setup_a: 0x33,
            setup_x: 0x05,
            expected_a: 0x03,
            expected_x: 0x05,
            expected_trace_contains: "AND $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "ROL $addr,X",
            setup_code: &[0x3E, 0x34, 0x12],
            setup_memory: vec![(0x1239, 0x81)],
            setup_x: 0x05,
            expected_x: 0x05,
            expected_memory: vec![(0x1239, 0x02)],
            expected_trace_contains: "ROL $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "EOR ($zp,X)",
            setup_code: &[0x41, 0x10],
            expected_a: 0x66,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0x33)],
            expected_trace_contains: "EOR ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "EOR $zp",
            setup_code: &[0x45, 0x10],
            expected_a: 0x66,
            setup_a: 0x55,
            setup_memory: vec![(0x10, 0x33)],
            expected_trace_contains: "EOR $10",
            ..Default::default()
        },
        TestCase {
            name: "LSR $zp",
            setup_code: &[0x46, 0x10],
            expected_memory: vec![(0x10, 0x40)],
            setup_memory: vec![(0x10, 0x81)],
            expected_trace_contains: "LSR $10",
            ..Default::default()
        },
        TestCase {
            name: "EOR #$imm",
            setup_code: &[0x49, 0x33],
            expected_a: 0x66,
            setup_a: 0x55,
            expected_trace_contains: "EOR #$33",
            ..Default::default()
        },
        TestCase {
            name: "LSR A",
            setup_code: &[0x4A],
            expected_a: 0x40,
            setup_a: 0x81,
            expected_trace_contains: "LSR A",
            ..Default::default()
        },
        TestCase {
            name: "EOR $addr",
            setup_code: &[0x4D, 0x34, 0x12],
            expected_a: 0x66,
            setup_a: 0x55,
            setup_memory: vec![(0x1234, 0x33)],
            expected_trace_contains: "EOR $1234",
            ..Default::default()
        },
        TestCase {
            name: "LSR $addr",
            setup_code: &[0x4E, 0x34, 0x12],
            expected_memory: vec![(0x1234, 0x40)],
            setup_memory: vec![(0x1234, 0x81)],
            expected_trace_contains: "LSR $1234",
            ..Default::default()
        },
        TestCase {
            name: "EOR ($zp),Y",
            setup_code: &[0x51, 0x10],
            expected_a: 0x66,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0x33)],
            expected_trace_contains: "EOR ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "EOR $zp,X",
            setup_code: &[0x55, 0x10],
            expected_a: 0x66,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x33)],
            expected_trace_contains: "EOR $10,X",
            ..Default::default()
        },
        TestCase {
            name: "LSR $zp,X",
            setup_code: &[0x56, 0x10],
            expected_memory: vec![(0x15, 0x40)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x81)],
            expected_trace_contains: "LSR $10,X",
            ..Default::default()
        },
        TestCase {
            name: "EOR $addr,Y",
            setup_code: &[0x59, 0x34, 0x12],
            expected_a: 0x66,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x123A, 0x33)],
            expected_trace_contains: "EOR $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "EOR $addr,X",
            setup_code: &[0x5D, 0x34, 0x12],
            expected_a: 0x66,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x33)],
            expected_trace_contains: "EOR $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "LSR $addr,X",
            setup_code: &[0x5E, 0x34, 0x12],
            expected_memory: vec![(0x1239, 0x40)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x81)],
            expected_trace_contains: "LSR $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "ADC ($zp,X)",
            setup_code: &[0x61, 0x10],
            expected_a: 0x88,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0x33)],
            expected_trace_contains: "ADC ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "ADC $zp",
            setup_code: &[0x65, 0x10],
            expected_a: 0x88,
            setup_a: 0x55,
            setup_memory: vec![(0x10, 0x33)],
            expected_trace_contains: "ADC $10",
            ..Default::default()
        },
        TestCase {
            name: "ROR $zp",
            setup_code: &[0x66, 0x10],
            expected_memory: vec![(0x10, 0x40)],
            setup_memory: vec![(0x10, 0x81)],
            expected_trace_contains: "ROR $10",
            ..Default::default()
        },
        TestCase {
            name: "ADC #$imm",
            setup_code: &[0x69, 0x33],
            expected_a: 0x88,
            setup_a: 0x55,
            expected_trace_contains: "ADC #$33",
            ..Default::default()
        },
        TestCase {
            name: "ROR A",
            setup_code: &[0x6A],
            expected_a: 0x40,
            setup_a: 0x81,
            expected_trace_contains: "ROR A",
            ..Default::default()
        },
        TestCase {
            name: "ADC $addr",
            setup_code: &[0x6D, 0x34, 0x12],
            expected_a: 0x88,
            setup_a: 0x55,
            setup_memory: vec![(0x1234, 0x33)],
            expected_trace_contains: "ADC $1234",
            ..Default::default()
        },
        TestCase {
            name: "ROR $addr",
            setup_code: &[0x6E, 0x34, 0x12],
            expected_memory: vec![(0x1234, 0x40)],
            setup_memory: vec![(0x1234, 0x81)],
            expected_trace_contains: "ROR $1234",
            ..Default::default()
        },
        TestCase {
            name: "ADC ($zp),Y",
            setup_code: &[0x71, 0x10],
            expected_a: 0x88,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0x33)],
            expected_trace_contains: "ADC ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "ADC $zp,X",
            setup_code: &[0x75, 0x10],
            expected_a: 0x88,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x33)],
            expected_trace_contains: "ADC $10,X",
            ..Default::default()
        },
        TestCase {
            name: "ROR $zp,X",
            setup_code: &[0x76, 0x10],
            expected_memory: vec![(0x15, 0x40)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x81)],
            expected_trace_contains: "ROR $10,X",
            ..Default::default()
        },
        TestCase {
            name: "ADC $addr,Y",
            setup_code: &[0x79, 0x34, 0x12],
            expected_a: 0x88,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x123A, 0x33)],
            expected_trace_contains: "ADC $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "ADC $addr,X",
            setup_code: &[0x7D, 0x34, 0x12],
            expected_a: 0x88,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x33)],
            expected_trace_contains: "ADC $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "ROR $addr,X",
            setup_code: &[0x7E, 0x34, 0x12],
            expected_memory: vec![(0x1239, 0x40)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x81)],
            expected_trace_contains: "ROR $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "CMP ($zp,X)",
            setup_code: &[0xC1, 0x10],
            expected_a: 0x55,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0x33)],
            expected_trace_contains: "CMP ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "CMP $zp",
            setup_code: &[0xC5, 0x10],
            expected_a: 0x55,
            setup_a: 0x55,
            setup_memory: vec![(0x10, 0x33)],
            expected_trace_contains: "CMP $10",
            ..Default::default()
        },
        TestCase {
            name: "DEC $zp",
            setup_code: &[0xC6, 0x10],
            expected_memory: vec![(0x10, 0x80)],
            setup_memory: vec![(0x10, 0x81)],
            expected_trace_contains: "DEC $10",
            ..Default::default()
        },
        TestCase {
            name: "CMP #$imm",
            setup_code: &[0xC9, 0x33],
            expected_a: 0x55,
            setup_a: 0x55,
            expected_trace_contains: "CMP #$33",
            ..Default::default()
        },
        TestCase {
            name: "CMP $addr",
            setup_code: &[0xCD, 0x34, 0x12],
            expected_a: 0x55,
            setup_a: 0x55,
            setup_memory: vec![(0x1234, 0x33)],
            expected_trace_contains: "CMP $1234",
            ..Default::default()
        },
        TestCase {
            name: "DEC $addr",
            setup_code: &[0xCE, 0x34, 0x12],
            expected_memory: vec![(0x1234, 0x80)],
            setup_memory: vec![(0x1234, 0x81)],
            expected_trace_contains: "DEC $1234",
            ..Default::default()
        },
        TestCase {
            name: "CMP ($zp),Y",
            setup_code: &[0xD1, 0x10],
            expected_a: 0x55,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0x33)],
            expected_trace_contains: "CMP ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "CMP $zp,X",
            setup_code: &[0xD5, 0x10],
            expected_a: 0x55,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x33)],
            expected_trace_contains: "CMP $10,X",
            ..Default::default()
        },
        TestCase {
            name: "DEC $zp,X",
            setup_code: &[0xD6, 0x10],
            expected_memory: vec![(0x15, 0x80)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x81)],
            expected_trace_contains: "DEC $10,X",
            ..Default::default()
        },
        TestCase {
            name: "CMP $addr,Y",
            setup_code: &[0xD9, 0x34, 0x12],
            expected_a: 0x55,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x123A, 0x33)],
            expected_trace_contains: "CMP $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "CMP $addr,X",
            setup_code: &[0xDD, 0x34, 0x12],
            expected_a: 0x55,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x33)],
            expected_trace_contains: "CMP $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "DEC $addr,X",
            setup_code: &[0xDE, 0x34, 0x12],
            expected_memory: vec![(0x1239, 0x80)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x81)],
            expected_trace_contains: "DEC $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "SBC ($zp,X)",
            setup_code: &[0xE1, 0x10],
            expected_a: 0x21,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x34), (0x16, 0x12), (0x1234, 0x33)],
            expected_trace_contains: "SBC ($10,X)",
            ..Default::default()
        },
        TestCase {
            name: "SBC $zp",
            setup_code: &[0xE5, 0x10],
            expected_a: 0x21,
            setup_a: 0x55,
            setup_memory: vec![(0x10, 0x33)],
            expected_trace_contains: "SBC $10",
            ..Default::default()
        },
        TestCase {
            name: "INC $zp",
            setup_code: &[0xE6, 0x10],
            expected_memory: vec![(0x10, 0x82)],
            setup_memory: vec![(0x10, 0x81)],
            expected_trace_contains: "INC $10",
            ..Default::default()
        },
        TestCase {
            name: "SBC #$imm",
            setup_code: &[0xE9, 0x33],
            expected_a: 0x21,
            setup_a: 0x55,
            expected_trace_contains: "SBC #$33",
            ..Default::default()
        },
        TestCase {
            name: "SBC $addr",
            setup_code: &[0xED, 0x34, 0x12],
            expected_a: 0x21,
            setup_a: 0x55,
            setup_memory: vec![(0x1234, 0x33)],
            expected_trace_contains: "SBC $1234",
            ..Default::default()
        },
        TestCase {
            name: "INC $addr",
            setup_code: &[0xEE, 0x34, 0x12],
            expected_memory: vec![(0x1234, 0x82)],
            setup_memory: vec![(0x1234, 0x81)],
            expected_trace_contains: "INC $1234",
            ..Default::default()
        },
        TestCase {
            name: "SBC ($zp),Y",
            setup_code: &[0xF1, 0x10],
            expected_a: 0x21,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x10, 0x34), (0x11, 0x12), (0x123A, 0x33)],
            expected_trace_contains: "SBC ($10),Y",
            ..Default::default()
        },
        TestCase {
            name: "SBC $zp,X",
            setup_code: &[0xF5, 0x10],
            expected_a: 0x21,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x33)],
            expected_trace_contains: "SBC $10,X",
            ..Default::default()
        },
        TestCase {
            name: "INC $zp,X",
            setup_code: &[0xF6, 0x10],
            expected_memory: vec![(0x15, 0x82)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x15, 0x81)],
            expected_trace_contains: "INC $10,X",
            ..Default::default()
        },
        TestCase {
            name: "SBC $addr,Y",
            setup_code: &[0xF9, 0x34, 0x12],
            expected_a: 0x21,
            expected_y: 0x06,
            setup_a: 0x55,
            setup_y: 0x06,
            setup_memory: vec![(0x123A, 0x33)],
            expected_trace_contains: "SBC $1234,Y",
            ..Default::default()
        },
        TestCase {
            name: "SBC $addr,X",
            setup_code: &[0xFD, 0x34, 0x12],
            expected_a: 0x21,
            expected_x: 0x05,
            setup_a: 0x55,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x33)],
            expected_trace_contains: "SBC $1234,X",
            ..Default::default()
        },
        TestCase {
            name: "INC $addr,X",
            setup_code: &[0xFE, 0x34, 0x12],
            expected_memory: vec![(0x1239, 0x82)],
            expected_x: 0x05,
            setup_x: 0x05,
            setup_memory: vec![(0x1239, 0x81)],
            expected_trace_contains: "INC $1234,X",
            ..Default::default()
        },
    ];

    for tc in cases {
        run_test_case(&tc);
    }
}
