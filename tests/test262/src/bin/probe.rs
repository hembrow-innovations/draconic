//! One-shot Test262 probe: paths on stdin → pass/fail lines on stdout.
//! Usage: cargo run -p draconic-test262 --bin probe < paths.txt

use std::io::{self, BufRead};

use draconic_test262::{resolve_suite_root, run_case, Status};

fn main() {
    let root = resolve_suite_root();
    let stdin = io::stdin();
    let mut pass = 0usize;
    let mut fail = 0usize;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { continue };
        let rel = line.trim();
        if rel.is_empty() || rel.starts_with('#') {
            continue;
        }
        let r = run_case(&root, rel);
        match r.status {
            Status::Pass => {
                pass += 1;
                println!("PASS\t{rel}");
            }
            Status::Fail => {
                fail += 1;
                let msg = r.message.replace('\n', " ").chars().take(160).collect::<String>();
                println!("FAIL\t{rel}\t{msg}");
            }
            Status::Skip => {
                println!("SKIP\t{rel}\t{}", r.message);
            }
        }
    }
    eprintln!("# probe totals pass={pass} fail={fail}");
}
