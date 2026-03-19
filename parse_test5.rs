fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    let mut valid_bytes = raw
        .bytes()
        .enumerate()
        .filter(|&(_, b)| !b.is_ascii_whitespace() && b != b'_');

    while let Some((i1, b1)) = valid_bytes.next() {
        let Some((i2, b2)) = valid_bytes.next() else {
            return Err("invalid params: rom_hex must have an even number of hex digits".to_owned());
        };

        let hi = decode_hex_nibble(b1, i1)?;
        let lo = decode_hex_nibble(b2, i2)?;
        bytes.push((hi << 4) + lo);
    }
    Ok(bytes)
}

fn decode_hex_nibble(ch: u8, index: usize) -> Result<u8, String> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        b'A'..=b'F' => Ok(ch - b'A' + 10),
        _ => Err(format!(
            "invalid params: rom_hex has invalid hex digit '{}' at index {}",
            ch as char, index
        )),
    }
}

fn main() {
    let odd = parse_hex_bytes("ABC").unwrap_err();
    assert_eq!(odd.to_string(), "invalid params: rom_hex must have an even number of hex digits");

    let bad = parse_hex_bytes("AAx0").unwrap_err();
    assert_eq!(bad.to_string(), "invalid params: rom_hex has invalid hex digit 'x' at index 2");

    let bad_low_nibble = parse_hex_bytes("AA0x").unwrap_err();
    assert_eq!(bad_low_nibble.to_string(), "invalid params: rom_hex has invalid hex digit 'x' at index 3");

    let parsed = parse_hex_bytes("de ad_BE ef").unwrap();
    assert_eq!(parsed, vec![0xDE, 0xAD, 0xBE, 0xEF]);

    println!("All tests passed!");
}
