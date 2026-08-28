//! ROADMAP L06.01: leveled logger + filter by level.
//! ROADMAP L06.02: sink to stderr/stdout (string format).

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn levels_filter_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/logging/levels_filter"),
        "missing stdlib/logging/levels_filter fixture, got {ids:?}"
    );
}

#[test]
fn levels_filter_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/logging/levels_filter")
        .expect("stdlib/logging/levels_filter");
    assert_eq!(fixture.targets.len(), 2, "L06.01 targets both js and native");
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
fn stdio_sink_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/logging/stdio_sink"),
        "missing stdlib/logging/stdio_sink fixture, got {ids:?}"
    );
}

#[test]
fn stdio_sink_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/logging/stdio_sink")
        .expect("stdlib/logging/stdio_sink");
    assert_eq!(fixture.targets.len(), 2, "L06.02 targets both js and native");
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
