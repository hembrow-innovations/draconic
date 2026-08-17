//! ROADMAP U11: line coverage via `draconic test --coverage` (js).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_conformance::{load_path, run_fixture_cov, CoverageReport};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-cov-{}-{}-{}",
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
fn e2e_js_line_coverage_hits_executed_lines() {
    let dir = temp_dir();
    let src = dir.join("prog.drac");
    fs::write(&src, "let x = 10;\nlet y = 20;\nlet z = x + y;\n").unwrap();
    fs::write(
        dir.join("prog.meta"),
        "\
id: prog
targets: js
js.exit: 0
js.check: if (z !== 30) process.exit(1);
",
    )
    .unwrap();

    let fixtures = load_path(&dir).expect("load");
    let mut report = CoverageReport::new();
    for f in &fixtures {
        let results = run_fixture_cov(f, Some(&mut report));
        for r in results {
            assert!(r.ok, "{}: {}", r.fixture_id, r.message);
        }
    }

    assert!(report.total_executable() > 0, "expected mapped lines");
    assert!(report.total_hit() > 0, "expected hit lines");
    assert!(
        report.total_hit() <= report.total_executable(),
        "hit <= executable"
    );
    let summary = report.format_summary();
    assert!(summary.contains("coverage"));
    assert!(summary.contains("lines"));
}

#[test]
fn coverage_report_merge_is_monotonic() {
    let mut r = CoverageReport::new();
    let mut exec = BTreeSet::new();
    exec.insert(1);
    exec.insert(2);
    exec.insert(3);
    let mut hit = BTreeSet::new();
    hit.insert(1);
    r.merge_file("a.drac", exec.clone(), hit);
    let mut hit2 = BTreeSet::new();
    hit2.insert(2);
    r.merge_file("a.drac", exec, hit2);
    assert_eq!(r.total_executable(), 3);
    assert_eq!(r.total_hit(), 2);
}
