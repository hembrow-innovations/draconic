//! Designed parser fuzz entry (Roadmap **R05**; harness is **R05.01**).
//!
//! [`fuzz_parse`] is the public parse-entry hook. Call it from a fuzzer
//! (cargo-fuzz, AFL, honggfuzz, or the designed stdin harness under `fuzz/`).
//! Ok and Err are both success for the harness; panics / aborts are failures.
//! Embed/runtime fuzz stays **R05.02**.

use crate::{parse, parse_module};

/// Fuzz entry: treat `data` as source text and parse as script and module.
///
/// Invalid UTF-8 is decoded lossily so byte-oriented fuzzers still exercise
/// the lexer/parser. Parse diagnostics are discarded; the contract is **no panic**.
pub fn fuzz_parse(data: &[u8]) {
    let source = String::from_utf8_lossy(data);
    let _ = parse(source.as_ref());
    let _ = parse_module(source.as_ref());
}

#[cfg(test)]
mod tests {
    use super::fuzz_parse;

    #[test]
    fn fuzz_parse_empty_and_valid_smoke() {
        fuzz_parse(b"");
        fuzz_parse(b"let x = 1;\n");
        fuzz_parse(b"export default 1;\n");
    }

    #[test]
    fn fuzz_parse_invalid_syntax_no_panic() {
        fuzz_parse(b"{");
        fuzz_parse(b"function (");
        fuzz_parse(b"class {");
        fuzz_parse(b"import {");
        fuzz_parse(b"(((((;");
    }

    #[test]
    fn fuzz_parse_binary_and_non_utf8_no_panic() {
        fuzz_parse(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
        fuzz_parse(&[0xc0, 0x80, b'{', b'}']);
        // Mixed control bytes and plausible tokens.
        let mut buf = Vec::new();
        for b in 0u8..=0xff {
            buf.push(b);
            if buf.len() % 17 == 0 {
                buf.extend_from_slice(b" let x=1; ");
            }
        }
        fuzz_parse(&buf);
    }

    #[test]
    fn fuzz_parse_structured_garbage_no_panic() {
        // Nested-ish noise that often stresses recursive descent.
        let s = "{{{{{{{{ monsta }}}}}}}} ;;;; async function* #x() { await yield; }\n".repeat(8);
        fuzz_parse(s.as_bytes());
        fuzz_parse(b"`${`${`${1}`}`}`;\n");
        fuzz_parse(b"/a/g /b/;\n");
        fuzz_parse(b"<!--\n-->\n");
    }

    /// R05 parent: the designed parser entry is the crate-root `fuzz_parse` hook.
    #[test]
    fn r05_designed_parser_entry_does_not_panic() {
        crate::fuzz_parse(b"");
        crate::fuzz_parse(b"let x = 1;\n");
        crate::fuzz_parse(b"export default 1;\n");
        crate::fuzz_parse(b"{");
        crate::fuzz_parse(&[0xff, 0xfe, 0x00, 0x01, 0x7f]);
    }
}
