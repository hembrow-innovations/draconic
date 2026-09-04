//! ROADMAP L07.01: parse long/short flags + leftover positionals from a string array.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn parse_long_short_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/flags/parse_long_short"),
        "missing stdlib/flags/parse_long_short fixture, got {ids:?}"
    );
}

#[test]
fn parse_long_short_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/flags/parse_long_short")
        .expect("stdlib/flags/parse_long_short");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L07.01 targets both js and native"
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
