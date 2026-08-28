//! ROADMAP L05.01: `describe` / `it` register tests; run via `draconic test`.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn describe_it_run_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/testing/describe_it_run"),
        "missing stdlib/testing/describe_it_run fixture, got {ids:?}"
    );
}

#[test]
fn describe_it_run_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/testing/describe_it_run")
        .expect("stdlib/testing/describe_it_run");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L05.01 targets both js and native"
    );
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
fn describe_it_fail_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/testing/describe_it_fail"),
        "missing stdlib/testing/describe_it_fail fixture, got {ids:?}"
    );
}

#[test]
fn describe_it_fail_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/testing/describe_it_fail")
        .expect("stdlib/testing/describe_it_fail");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L05.01 targets both js and native"
    );
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
