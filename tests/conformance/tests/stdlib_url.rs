//! ROADMAP L08.01: stdlib URL parse — scheme/host/path/query/hash.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn parse_basics_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/url/parse_basics"),
        "missing stdlib/url/parse_basics fixture, got {ids:?}"
    );
}

#[test]
fn parse_basics_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/url/parse_basics")
        .expect("stdlib/url/parse_basics");
    assert_eq!(fixture.targets.len(), 2, "L08.01 targets both js and native");
    for r in run_fixture(fixture) {
        assert!(r.ok, "{} @ {}: {}", r.fixture_id, r.target.as_str(), r.message);
    }
}
