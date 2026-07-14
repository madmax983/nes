use nes_core::CheatCode;

#[test]
fn cheat_code_parsing_is_strict_about_bits() {
    let test_cases = [
        ("GOSSIP", 0xD1DD, 0x14, None),
        ("ZEXPYGLA", 0x94A7, 0x02, Some(0x03)),
        ("SXTPOU", 0x9BE1, 0xAD, None),
        ("AEXSAX", 53928, 8, None),
        ("PZLZIG", 42037, 33, None),
        ("VNETKY", 61316, 246, None),
        ("XYZAEXSA", 35360, 250, Some(141)),
    ];

    for (raw, exp_addr, exp_val, exp_comp) in test_cases {
        let code: CheatCode = raw.parse().unwrap();
        assert_eq!(code.address(), exp_addr, "Address mismatch for {}", raw);
        assert_eq!(code.value(), exp_val, "Value mismatch for {}", raw);
        assert_eq!(code.compare(), exp_comp, "Compare mismatch for {}", raw);
    }
}
