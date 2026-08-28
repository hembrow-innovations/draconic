//! ROADMAP L05.01 / L05.02 / L05.03: `describe` / `it` / `expect` + nested hooks.

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

#[test]
fn expect_matchers_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/testing/expect_matchers"),
        "missing stdlib/testing/expect_matchers fixture, got {ids:?}"
    );
}

#[test]
fn expect_matchers_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/testing/expect_matchers")
        .expect("stdlib/testing/expect_matchers");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L05.02 targets both js and native"
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
fn expect_fail_messages_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/testing/expect_fail_messages"),
        "missing stdlib/testing/expect_fail_messages fixture, got {ids:?}"
    );
}

#[test]
fn expect_fail_messages_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/testing/expect_fail_messages")
        .expect("stdlib/testing/expect_fail_messages");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L05.02 targets both js and native"
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
fn nested_hooks_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/testing/nested_hooks"),
        "missing stdlib/testing/nested_hooks fixture, got {ids:?}"
    );
}

#[test]
fn nested_hooks_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/testing/nested_hooks")
        .expect("stdlib/testing/nested_hooks");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L05.03 targets both js and native"
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
