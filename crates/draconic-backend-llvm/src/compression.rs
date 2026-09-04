//! gzip / zlib-deflate byte buffers for L04
//! (`gzip` / `gunzip` / `deflate` / `inflate` on `Uint8Array`).

use std::io::Read;

use flate2::read::{GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder};
use flate2::Compression;

pub(crate) fn gzip(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut enc = GzEncoder::new(bytes, Compression::default());
    let mut out = Vec::new();
    enc.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

pub(crate) fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut dec = GzDecoder::new(bytes);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

pub(crate) fn deflate(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut enc = ZlibEncoder::new(bytes, Compression::default());
    let mut out = Vec::new();
    enc.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

pub(crate) fn inflate(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut dec = ZlibDecoder::new(bytes);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Node `zlib.gzipSync(Buffer.from("hello"), { level: 9, mtime: 0 })`.
    const GZIP_HELLO: &[u8] = &[
        31, 139, 8, 0, 0, 0, 0, 0, 2, 19, 203, 72, 205, 201, 201, 7, 0, 134, 166, 16, 54, 5, 0, 0,
        0,
    ];

    /// Node `zlib.deflateSync(Buffer.from("hello"), { level: 9 })`.
    const DEFLATE_HELLO: &[u8] = &[120, 218, 203, 72, 205, 201, 201, 7, 0, 6, 44, 2, 21];

    #[test]
    fn gzip_roundtrip_hello() {
        let gz = gzip(b"hello").expect("gzip");
        assert_eq!(&gz[..2], &[0x1f, 0x8b]);
        assert_eq!(gunzip(&gz).expect("gunzip"), b"hello");
    }

    #[test]
    fn gunzip_known_hello_vector() {
        assert_eq!(gunzip(GZIP_HELLO).expect("gunzip known"), b"hello");
    }

    #[test]
    fn gzip_roundtrip_empty() {
        let gz = gzip(b"").expect("gzip empty");
        assert_eq!(gunzip(&gz).expect("gunzip empty"), b"");
    }

    #[test]
    fn gunzip_truncated_errors() {
        assert!(gunzip(&[31, 139, 8]).is_err());
        assert!(gunzip(&[0, 1, 2, 3, 4, 5]).is_err());
    }

    #[test]
    fn deflate_roundtrip_hello() {
        let d = deflate(b"hello").expect("deflate");
        assert_eq!(d[0], 0x78);
        assert_eq!(inflate(&d).expect("inflate"), b"hello");
    }

    #[test]
    fn inflate_known_hello_vector() {
        assert_eq!(inflate(DEFLATE_HELLO).expect("inflate known"), b"hello");
    }

    #[test]
    fn deflate_roundtrip_empty() {
        let d = deflate(b"").expect("deflate empty");
        assert_eq!(inflate(&d).expect("inflate empty"), b"");
    }

    #[test]
    fn inflate_truncated_errors() {
        assert!(inflate(&[120, 218]).is_err());
        assert!(inflate(&[0, 1, 2, 3]).is_err());
    }
}
