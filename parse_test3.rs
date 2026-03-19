fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    let mut valid_chars = raw
        .char_indices()
        .filter(|&(_, ch)| !ch.is_ascii_whitespace() && ch != '_');

    while let Some((i1, ch1)) = valid_chars.next() {
        let Some((_, ch2)) = valid_chars.next() else {
            return Err("rom_hex must have an even number of hex digits".to_owned());
        };

        let hi = decode_hex_nibble(ch1, i1)?;
        // Wait, what's the index of the second one? We want to report the correct index.
        // The original code was:
        // let hi = decode_hex_nibble(as_bytes[index], index)?;
        // let lo = decode_hex_nibble(as_bytes[index + 1], index + 1)?;
        // But what if there are whitespaces?
        panic!("Wait, let's see how the original handled it.");
    }
    Ok(bytes)
}

fn decode_hex_nibble(ch: char, index: usize) -> Result<u8, String> {
    Ok(0)
}

fn main() {}
