pub fn encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0usize;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        i += 3;

        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) + (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from(((b1 & 0x0F) << 2) + (b2 >> 6))] as char);
        out.push(ALPHABET[usize::from(b2 & 0x3F)] as char);
    }

    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i];
        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from((b0 & 0x03) << 4)] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) + (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from((b1 & 0x0F) << 2)] as char);
        out.push('=');
    }

    out
}

pub fn decode(raw: &str) -> Result<Vec<u8>, String> {
    fn decode_char(ch: u8) -> Result<u8, String> {
        match ch {
            b'A'..=b'Z' => Ok(ch - b'A'),
            b'a'..=b'z' => Ok(ch - b'a' + 26),
            b'0'..=b'9' => Ok(ch - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("invalid base64 character '{}'", ch as char)),
        }
    }

    if !raw.len().is_multiple_of(4) {
        return Err("base64 length must be multiple of 4".to_owned());
    }

    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    let mut i = 0usize;
    while i < bytes.len() {
        let c0 = bytes[i];
        let c1 = bytes[i + 1];
        let c2 = bytes[i + 2];
        let c3 = bytes[i + 3];
        i += 4;

        let v0 = decode_char(c0)?;
        let v1 = decode_char(c1)?;
        let pad2 = c2 == b'=';
        let pad3 = c3 == b'=';
        let v2 = if pad2 { 0 } else { decode_char(c2)? };
        let v3 = if pad3 { 0 } else { decode_char(c3)? };

        out.push((v0 << 2) | (v1 >> 4));
        if !pad2 {
            out.push((v1 << 4) | (v2 >> 2));
        }
        if !pad3 {
            out.push((v2 << 6) | v3);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_matches_known_vectors() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn decode_matches_known_vectors() {
        assert_eq!(decode("").unwrap(), b"");
        assert_eq!(decode("Zg==").unwrap(), b"f");
        assert_eq!(decode("Zm8=").unwrap(), b"fo");
        assert_eq!(decode("Zm9v").unwrap(), b"foo");
        assert_eq!(decode("Zm9vYg==").unwrap(), b"foob");
        assert_eq!(decode("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn decode_rejects_invalid_length() {
        assert!(decode("Zg").is_err());
        assert!(decode("Zg=").is_err());
        assert!(decode("Zg===").is_err());
    }

    #[test]
    fn decode_rejects_invalid_chars() {
        assert!(decode("Zg=^").is_err());
    }
}
