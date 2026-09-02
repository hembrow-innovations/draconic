//! ROADMAP F05 / F05.01–F05.02: load dynamic lib; resolve and call one C symbol.
//! F05 parent locks the combined link-dynamic / call-one-symbol surface in one Program.

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

#[test]
fn link_dynamic_call_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == "ffi/link_dynamic/call"),
        "missing ffi/link_dynamic/call fixture, got {ids:?}"
    );
}

#[test]
fn link_dynamic_call_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "ffi/link_dynamic/call")
        .expect("ffi/link_dynamic/call");
    assert!(
        fixture.targets.contains(&Target::Native),
        "ffi/link_dynamic/call must target native"
    );
    assert_eq!(
        fixture.expect_native.dylink.len(),
        1,
        "fixture must declare native.dylink"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("42\n7\n"),
        "F05.02 must observe C return values, not a local let"
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
fn surface_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == "ffi/link_dynamic/surface"),
        "missing ffi/link_dynamic/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "ffi/link_dynamic/surface")
        .expect("ffi/link_dynamic/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "ffi/link_dynamic/surface must target native"
    );
    assert_eq!(
        fixture.expect_native.dylink.len(),
        1,
        "surface must declare native.dylink"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("1\n42\n7\n"),
        "F05 surface must observe resolve side-effect print and C return values"
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
