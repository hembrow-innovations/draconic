//! ROADMAP E00: conformance harness loads fixtures and runs js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_all, run_fixture, Target};

#[test]
fn discovers_fixtures_under_fixtures_dir() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    assert!(
        !fixtures.is_empty(),
        "expected at least one .drac fixture under {}",
        fixtures_dir().display()
    );
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| id.contains("empty") || *id == "smoke/empty"),
        "missing smoke empty fixture, got {ids:?}"
    );
    assert!(
        ids.iter()
            .any(|id| id.contains("let") || *id == "smoke/let-add"),
        "missing smoke let-add fixture, got {ids:?}"
    );
}

#[test]
fn each_fixture_declares_js_and_or_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    for f in &fixtures {
        assert!(!f.targets.is_empty(), "fixture {} has no targets", f.id);
        for t in &f.targets {
            assert!(matches!(t, Target::Js | Target::Native));
        }
    }
}

#[test]
fn run_all_fixtures_green_on_declared_targets() {
    // E00: `run_all` exercises js + native on the smoke slice. Per-area tests
    // already run the rest of the tree; re-running every fixture here (native
    // LLVM included) made `cargo test --workspace` exceed the Review window as
    // E17.02 remainder fixtures accumulated.
    let results = run_all(&fixtures_dir().join("smoke")).expect("run_all");
    assert!(!results.is_empty(), "run_all produced no results");

    let mut js_runs = 0;
    let mut native_runs = 0;
    let mut failures = Vec::new();

    for r in &results {
        match r.target {
            Target::Js => js_runs += 1,
            Target::Native => native_runs += 1,
        }
        if !r.ok {
            failures.push(format!(
                "{} @ {}: {}",
                r.fixture_id,
                r.target.as_str(),
                r.message
            ));
        }
    }

    assert!(js_runs > 0, "expected at least one js run");
    assert!(native_runs > 0, "expected at least one native run");
    assert!(
        failures.is_empty(),
        "conformance failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn smoke_empty_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "smoke/empty" || f.id == "empty")
        .expect("smoke/empty fixture");
    assert!(!fixture.targets.is_empty());
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}

#[test]
fn smoke_let_add_runs_js_only() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "smoke/let-add" || f.id == "let_add" || f.id == "let-add")
        .expect("smoke/let-add fixture");
    assert_eq!(fixture.targets, vec![Target::Js]);
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}
