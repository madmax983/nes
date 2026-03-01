use nes_core::Status;

#[test]
fn zero_and_negative_flags_follow_value_written() {
    let mut s = Status::default();
    s.update_zn(0x00);
    assert!(s.zero());
    assert!(!s.negative());

    s.update_zn(0x80);
    assert!(!s.zero());
    assert!(s.negative());
}
