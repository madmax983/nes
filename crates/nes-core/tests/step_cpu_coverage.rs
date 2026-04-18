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
    ];

    for tc in cases {
        run_test_case(&tc);
    }
}
