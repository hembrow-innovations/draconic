//! ROADMAP L02.01 / L02.02: `groupBy` / `chunk` and designed Deque.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn groupby_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/collections/groupby"),
        "missing stdlib/collections/groupby fixture, got {ids:?}"
    );
}

#[test]
fn groupby_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/collections/groupby")
        .expect("stdlib/collections/groupby");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L02.01 targets both js and native"
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
fn chunk_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/collections/chunk"),
        "missing stdlib/collections/chunk fixture, got {ids:?}"
    );
}

#[test]
fn chunk_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/collections/chunk")
        .expect("stdlib/collections/chunk");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L02.01 targets both js and native"
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
fn invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/collections/invalid"),
        "missing stdlib/collections/invalid fixture, got {ids:?}"
    );
}

#[test]
fn invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/collections/invalid")
        .expect("stdlib/collections/invalid");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L02.01 targets both js and native"
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
fn deque_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/collections/deque"),
        "missing stdlib/collections/deque fixture, got {ids:?}"
    );
}

#[test]
fn deque_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/collections/deque")
        .expect("stdlib/collections/deque");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L02.02 targets both js and native"
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
