//! ROADMAP E10: exception fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn throw_try_catch_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/exceptions/throw_try_catch"),
        "missing es/exceptions/throw_try_catch fixture, got {ids:?}"
    );
}

#[test]
fn throw_try_catch_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/exceptions/throw_try_catch")
        .expect("es/exceptions/throw_try_catch");
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
fn try_finally_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/exceptions/try_finally"),
        "missing es/exceptions/try_finally fixture, got {ids:?}"
    );
}

#[test]
fn try_finally_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/exceptions/try_finally")
        .expect("es/exceptions/try_finally");
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
fn optional_catch_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/exceptions/optional_catch"),
        "missing es/exceptions/optional_catch fixture, got {ids:?}"
    );
}

#[test]
fn optional_catch_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/exceptions/optional_catch")
        .expect("es/exceptions/optional_catch");
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
