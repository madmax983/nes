use nes_core::cpu::status::Status;

#[test]
fn status_flag_accessors_strict() {
    let s = Status::default();
    assert_eq!(s.bits(), 0x00);

    // Explicitly construct with exact bits to test getters
    let s = Status::with_bits(0b1100_0001); // N, V, C
    assert!(s.negative());
    assert!(s.overflow());
    assert!(s.carry());
    assert!(!s.zero());
    assert!(!s.interrupt_disable());

    let s = Status::with_bits(0b0000_0110); // I, Z
    assert!(!s.negative());
    assert!(!s.overflow());
    assert!(!s.carry());
    assert!(s.zero());
    assert!(s.interrupt_disable());

    let mut s = Status::default();
    s.set_carry(true);
    assert!(s.carry());
    s.set_carry(false);
    assert!(!s.carry());

    s.set_interrupt_disable(true);
    assert!(s.interrupt_disable());
    s.set_interrupt_disable(false);
    assert!(!s.interrupt_disable());

    s.set_decimal(true);
    assert_eq!(s.bits() & 0x08, 0x08);
    s.set_decimal(false);
    assert_eq!(s.bits() & 0x08, 0x00);

    s.set_break(true);
    assert_eq!(s.bits() & 0x10, 0x10);
    s.set_break(false);
    assert_eq!(s.bits() & 0x10, 0x00);

    s.set_overflow(true);
    assert!(s.overflow());
    s.set_overflow(false);
    assert!(!s.overflow());

    s.set_negative(true);
    assert!(s.negative());
    s.set_negative(false);
    assert!(!s.negative());

    s.update_zn(0x00);
    assert!(s.zero());
    assert!(!s.negative());

    s.update_zn(0x80);
    assert!(!s.zero());
    assert!(s.negative());

    s.update_zn(0x01);
    assert!(!s.zero());
    assert!(!s.negative());

    s.update_compare(0x05, 0x05); // equal -> carry set
    assert!(s.carry());

    s.update_compare(0x05, 0x04); // a >= b -> carry set
    assert!(s.carry());

    s.update_compare(0x04, 0x05); // a < b -> carry clear
    assert!(!s.carry());

    // update_bit_test
    s.update_bit_test(0b1100_0000, 0b1100_0000); // non-zero intersection -> zero flag clear
    assert!(!s.zero());
    // update_bit_test copies bit 7 and 6 from rhs
    assert!(s.negative());
    assert!(s.overflow());

    s.update_bit_test(0b0000_1111, 0b0000_1111); // zero intersection (bit 6 and 7 of rhs are 0)
    assert!(!s.zero());

    s.update_bit_test(0b0000_1111, 0b1111_0000); // zero intersection
    assert!(s.zero());
    // update_bit_test copies N and V from rhs (rhs is 0b1111_0000 -> N=1, V=1)
    assert!(s.negative());
    assert!(s.overflow());

    s.update_bit_test(0b1111_0000, 0b0000_1111);
    assert!(s.zero());
    assert!(!s.negative());
    assert!(!s.overflow());

    // bits_for_stack_push
    let mut s = Status::with_bits(0b1100_0001); // N, V, C
    s.set_break(true);
    assert_eq!(s.bits_for_stack_push(), 225); // should have B and U bits set (0x10 | 0x20)

    s.set_break(false);
    assert_eq!(s.bits_for_stack_push(), 225); // should have U bit set (0x20)

    // bits_for_php
    assert_eq!(s.bits_for_php(), 241); // PHP always sets B and U

    // restore_from_stack
    s.restore_from_stack(0b1100_1111);
    assert_eq!(s.bits(), 239);
}

#[test]
fn restore_from_stack_ignores_b_and_u() {
    let mut s = Status::with_bits(0x00);
    s.restore_from_stack(0xFF);
    // B (0x10) and U (0x20) from the stack are ignored.
    // The CPU always keeps U=1 and B=0 internally.
    assert_eq!(s.bits(), 0xEF); // 0xFF & ~0x10 | 0x20 == 0xEF
}
