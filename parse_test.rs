use std::fmt;

#[derive(Debug)]
pub enum DispatchError {
    InvalidParams(String),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DispatchError::InvalidParams(s) => write!(f, "invalid params: {}", s),
        }
    }
}

fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, DispatchError> {
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    let mut valid_chars = raw
        .char_indices()
        .filter(|&(_, ch)| !ch.is_ascii_whitespace() && ch != '_');

    while let Some((i1, ch1)) = valid_chars.next() {
        let Some((i2, ch2)) = valid_chars.next() else {
            return Err(DispatchError::InvalidParams(
                "rom_hex must have an even number of hex digits".to_owned(),
            ));
        };

        let hi = decode_hex_nibble(ch1, i1)?;
        let lo = decode_hex_nibble(ch2, i2)?;
        bytes.push((hi << 4) + lo);
    }
    Ok(bytes)
}

fn decode_hex_nibble(ch: char, index: usize) -> Result<u8, DispatchError> {
    match ch {
        '0'..='9' => Ok(ch as u8 - b'0'),
        'a'..='f' => Ok(ch as u8 - b'a' + 10),
        'A'..='F' => Ok(ch as u8 - b'A' + 10),
        _ => Err(DispatchError::InvalidParams(format!(
            "rom_hex has invalid hex digit '{}' at index {}",
            ch, index
        ))),
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
