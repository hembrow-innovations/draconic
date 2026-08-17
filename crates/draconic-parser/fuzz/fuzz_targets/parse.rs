//! cargo-fuzz target for the parser (optional; needs libFuzzer).
//!
//! ```text
//! cargo fuzz run parse --fuzz-dir crates/draconic-parser/fuzz
//! # or:
//! cargo run --manifest-path crates/draconic-parser/fuzz/Cargo.toml \
//!   --features libfuzzer --bin parse
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    draconic_parser::fuzz_parse(data);
});
