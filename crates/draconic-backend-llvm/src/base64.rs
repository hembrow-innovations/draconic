//! RFC 4648 Base64 for L01.02 (`Uint8Array.toBase64` / `fromBase64`).

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(B64[((n >> 6) & 63) as usize] as char);
        out.push(B64[(n & 63) as usize] as char);
        i += 3;
    }
    match bytes.len() - i {
        1 => {
            let n = (bytes[i] as u32) << 16;
            out.push(B64[((n >> 18) & 63) as usize] as char);
            out.push(B64[((n >> 12) & 63) as usize] as char);
            out.push_str("==");
        }
        2 => {
            let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
            out.push(B64[((n >> 18) & 63) as usize] as char);
            out.push(B64[((n >> 12) & 63) as usize] as char);
            out.push(B64[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

fn val(c: u8) -> Result<u8, ()> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(()),
    }
}

pub(crate) fn decode(s: &str) -> Result<Vec<u8>, ()> {
    let raw = s.as_bytes();
    let mut pad = 0usize;
    let mut end = raw.len();
    while end > 0 && raw[end - 1] == b'=' {
        pad += 1;
        end -= 1;
        if pad > 2 {
            return Err(());
        }
    }
    let body = &raw[..end];
    if body.iter().any(|&c| val(c).is_err()) {
        return Err(());
    }
    let required = match body.len() % 4 {
        0 => 0,
        2 => 2,
        3 => 1,
        _ => return Err(()),
    };
    if pad != 0 && pad != required {
        return Err(());
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let remain = body.len() - i;
        let a = val(body[i])? as u32;
        let b = val(body[i + 1])? as u32;
        if remain == 2 {
            out.push(((a << 2) | (b >> 4)) as u8);
            break;
        }
        let c = val(body[i + 2])? as u32;
        if remain == 3 {
            out.push(((a << 2) | (b >> 4)) as u8);
            out.push(((b << 4) | (c >> 2)) as u8);
            break;
        }
        let d = val(body[i + 3])? as u32;
        out.push(((a << 2) | (b >> 4)) as u8);
        out.push(((b << 4) | (c >> 2)) as u8);
        out.push(((c << 6) | d) as u8);
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_vectors() {
        assert_eq!(encode(b"hi"), "aGk=");
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&[0]), "AA==");
        assert_eq!(encode(&[255, 255, 255]), "////");
        assert_eq!(decode("aGk=").unwrap(), b"hi");
        assert_eq!(decode("aGk").unwrap(), b"hi");
        assert_eq!(decode("").unwrap(), b"");
        assert!(decode("!!!").is_err());
        assert!(decode("a").is_err());
        assert!(decode("aGk==").is_err());
    }
}
