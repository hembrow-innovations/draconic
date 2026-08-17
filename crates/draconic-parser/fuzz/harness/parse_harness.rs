//! Designed parser fuzz harness (Roadmap **R05.01**).
//!
//! Reads one input from stdin (or a file path argv[1]) and runs
//! [`draconic_parser::fuzz_parse`]. Suitable for AFL/honggfuzz-style drivers
//! or a one-shot smoke:
//!
//! ```text
//! cargo run --manifest-path crates/draconic-parser/fuzz/Cargo.toml --bin parse_harness
//! echo 'let x = 1;' | cargo run --manifest-path crates/draconic-parser/fuzz/Cargo.toml --bin parse_harness
//! ```

use std::env;
use std::io::{self, Read};
use std::process;

fn main() {
    let data = match env::args().nth(1) {
        Some(path) => match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("parse_harness: failed to read {path}: {err}");
                process::exit(2);
            }
        },
        None => {
            let mut buf = Vec::new();
            if let Err(err) = io::stdin().read_to_end(&mut buf) {
                eprintln!("parse_harness: stdin read failed: {err}");
                process::exit(2);
            }
            buf
        }
    };

    draconic_parser::fuzz_parse(&data);
}
