#![no_main]

use libfuzzer_sys::fuzz_target;

fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    let mut valid_bytes = raw
        .bytes()
        .enumerate()
        .filter(|&(_, b)| !b.is_ascii_whitespace() && b != b'_');

    while let Some((i1, b1)) = valid_bytes.next() {
        let Some((i2, b2)) = valid_bytes.next() else {
            return Err("odd number".to_string());
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
        _ => Err("invalid".to_string()),
    }
}

fuzz_target!(|data: &str| {
    let _ = parse_hex_bytes(data);
});
