//! ROADMAP F05.01: load dynamic lib at link time; resolve one C symbol.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn link_dynamic_resolve_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == "ffi/link_dynamic/resolve"),
        "missing ffi/link_dynamic/resolve fixture, got {ids:?}"
    );
}

#[test]
fn link_dynamic_resolve_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "ffi/link_dynamic/resolve")
        .expect("ffi/link_dynamic/resolve");
    assert!(
        fixture.targets.contains(&Target::Native),
        "ffi/link_dynamic/resolve must target native"
    );
    assert_eq!(
        fixture.expect_native.dylink.len(),
        1,
        "fixture must declare native.dylink"
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
