//! Hex encode/decode for L01.03 (`Uint8Array.toHex` / `fromHex`).
//!
//! Encode is lowercase; decode accepts mixed case. Odd length or non-hex
//! characters fail (ECMA-262 Uint8Array hex; SyntaxError at the call site).

const HEX: &[u8] = b"0123456789abcdef";

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn nibble(c: u8) -> Result<u8, ()> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(()),
    }
}

pub(crate) fn decode(s: &str) -> Result<Vec<u8>, ()> {
    let raw = s.as_bytes();
    if !raw.len().is_multiple_of(2) {
        return Err(());
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let mut i = 0;
    while i < raw.len() {
        let hi = nibble(raw[i])?;
        let lo = nibble(raw[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_vectors() {
        assert_eq!(encode(b"hi"), "6869");
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&[0]), "00");
        assert_eq!(encode(&[255, 255, 255]), "ffffff");
        assert_eq!(encode(&[0, 255, 16]), "00ff10");
        assert_eq!(decode("6869").unwrap(), b"hi");
        assert_eq!(decode("00FF10").unwrap(), vec![0, 255, 16]);
        assert_eq!(decode("").unwrap(), b"");
        assert!(decode("zzz").is_err());
        assert!(decode("abc").is_err());
        assert!(decode("gg").is_err());
        assert!(decode("0x00").is_err());
    }
}
