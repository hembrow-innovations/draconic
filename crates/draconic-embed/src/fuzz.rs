//! Designed embed fuzz entry (Roadmap **R05.02**).
//!
//! [`fuzz_eval`] is the public embed-eval hook. Call it from a fuzzer
//! (cargo-fuzz, AFL, honggfuzz, or a designed stdin harness).
//! Ok and Err are both success for the harness; panics / aborts are failures.

/// Fuzz entry: treat `data` as source text and run embed `eval` / `Function`.
///
/// Invalid UTF-8 is decoded lossily so byte-oriented fuzzers still exercise
/// the embed path. Diagnostics are discarded; the contract is **no panic**.
pub fn fuzz_eval(data: &[u8]) {
    let source = String::from_utf8_lossy(data);
    let _ = crate::eval_source(source.as_ref());
    let _ = crate::eval_function_call(&[], source.as_ref(), &[]);
}

#[cfg(test)]
mod tests {
    use super::fuzz_eval;

    #[test]
    fn fuzz_eval_empty_and_valid_smoke() {
        fuzz_eval(b"");
        fuzz_eval(b"1 + 2");
        fuzz_eval(b"'hi'");
        fuzz_eval(b"typeof undefined");
    }

    #[test]
    fn fuzz_eval_invalid_syntax_no_panic() {
        fuzz_eval(b"{");
        fuzz_eval(b"function (");
        fuzz_eval(b"return");
        fuzz_eval(b"(((((;");
    }

    #[test]
    fn fuzz_eval_binary_and_non_utf8_no_panic() {
        fuzz_eval(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
        fuzz_eval(&[0xc0, 0x80, b'{', b'}']);
        let mut buf = Vec::new();
        for b in 0u8..=0xff {
            buf.push(b);
            if buf.len() % 17 == 0 {
                buf.extend_from_slice(b" 1+2; ");
            }
        }
        fuzz_eval(&buf);
    }

    /// R05.02: the designed embed entry is the crate-root `fuzz_eval` hook.
    #[test]
    fn r05_02_designed_embed_entry_does_not_panic() {
        crate::fuzz_eval(b"");
        crate::fuzz_eval(b"1 + 2");
        crate::fuzz_eval(b"{");
        crate::fuzz_eval(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
    }
}
