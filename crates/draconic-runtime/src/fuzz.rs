//! Designed Runtime fuzz/stress entry (Roadmap **R05.02**).
//!
//! [`fuzz_runtime`] is the public Runtime-entry hook. Call it from a fuzzer
//! (cargo-fuzz, AFL, honggfuzz, or a designed stdin harness).
//! Ok and Err are both success for the harness; panics / aborts are failures.

/// Fuzz entry: treat `data` as untrusted input at Runtime byte/string entries.
///
/// Invalid UTF-8 is decoded lossily so byte-oriented fuzzers still exercise
/// URL, query, flags, and MIME parse. Ok and Err are discarded; the contract
/// is **no panic**.
pub fn fuzz_runtime(data: &[u8]) {
    let source = String::from_utf8_lossy(data);
    let _ = crate::parse_url(source.as_ref());
    let _ = crate::parse_query(source.as_ref());
    let argv: Vec<String> = source.split_whitespace().map(str::to_string).collect();
    let _ = crate::parse_flags(&argv);
    let boundary = source.lines().next().unwrap_or("b");
    let _ = crate::parse_multipart(source.as_ref(), boundary);
}

#[cfg(test)]
mod tests {
    use super::fuzz_runtime;

    #[test]
    fn fuzz_runtime_empty_and_valid_smoke() {
        fuzz_runtime(b"");
        fuzz_runtime(b"https://example.com/path?q=1#frag");
        fuzz_runtime(b"a=1&b=2");
        fuzz_runtime(b"--verbose file.txt");
    }

    #[test]
    fn fuzz_runtime_invalid_input_no_panic() {
        fuzz_runtime(b"{");
        fuzz_runtime(b"/relative");
        fuzz_runtime(b"---");
        fuzz_runtime(b"--\nbad");
    }

    #[test]
    fn fuzz_runtime_binary_and_non_utf8_no_panic() {
        fuzz_runtime(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
        fuzz_runtime(&[0xc0, 0x80, b'?', b'=', b'a']);
        let mut buf = Vec::new();
        for b in 0u8..=0xff {
            buf.push(b);
            if buf.len() % 17 == 0 {
                buf.extend_from_slice(b" https://x.test ");
            }
        }
        fuzz_runtime(&buf);
    }

    /// R05.02: the designed Runtime entry is the crate-root `fuzz_runtime` hook.
    #[test]
    fn r05_02_designed_runtime_entry_does_not_panic() {
        crate::fuzz_runtime(b"");
        crate::fuzz_runtime(b"https://example.com/");
        crate::fuzz_runtime(b"{");
        crate::fuzz_runtime(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
    }
}
