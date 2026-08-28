//! ROADMAP L05.04: `draconic test` aggregates in-language suite exit with fixtures.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_conformance::{load_path, run_fixture};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-l0504-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mixed_dir_loads_fixture_and_in_language_suite() {
    let dir = temp_dir();
    fs::write(dir.join("smoke.drac"), "let x = 1 + 2;\n").unwrap();
    fs::write(
        dir.join("smoke.meta"),
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    )
    .unwrap();
    fs::write(
        dir.join("suite.drac"),
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    )
    .unwrap();

    let fixtures = load_path(&dir).expect("load");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "smoke"),
        "missing fixture id, got {ids:?}"
    );
    assert!(
        ids.iter().any(|id| *id == "suite"),
        "missing in-language suite id, got {ids:?}"
    );
}

#[test]
fn mixed_dir_aggregate_fails_when_suite_fails() {
    let dir = temp_dir();
    fs::write(dir.join("smoke.drac"), "let x = 1 + 2;\n").unwrap();
    fs::write(
        dir.join("smoke.meta"),
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    )
    .unwrap();
    fs::write(
        dir.join("suite.drac"),
        r#"
describe("math", () => {
  it("adds", () => {
    throw 1;
  });
});
"#,
    )
    .unwrap();

    let fixtures = load_path(&dir).expect("load");
    let mut any_fail = false;
    let mut any_pass = false;
    for f in &fixtures {
        for r in run_fixture(f) {
            if r.ok {
                any_pass = true;
            } else {
                any_fail = true;
            }
        }
    }
    assert!(any_pass, "expected passing fixture result");
    assert!(any_fail, "expected failing in-language suite result");
}
