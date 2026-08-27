//! ROADMAP F04.01 / F04.02: build links `.a`; resolve and call one C symbol.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn link_static_resolve_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == "ffi/link_static/resolve"),
        "missing ffi/link_static/resolve fixture, got {ids:?}"
    );
}

#[test]
fn link_static_resolve_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "ffi/link_static/resolve")
        .expect("ffi/link_static/resolve");
    assert!(
        fixture.targets.contains(&Target::Native),
        "ffi/link_static/resolve must target native"
    );
    assert_eq!(
        fixture.expect_native.link.len(),
        1,
        "fixture must declare native.link"
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
fn link_static_call_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == "ffi/link_static/call"),
        "missing ffi/link_static/call fixture, got {ids:?}"
    );
}

#[test]
fn link_static_call_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "ffi/link_static/call")
        .expect("ffi/link_static/call");
    assert!(
        fixture.targets.contains(&Target::Native),
        "ffi/link_static/call must target native"
    );
    assert_eq!(
        fixture.expect_native.link.len(),
        1,
        "fixture must declare native.link"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("42\n7\n"),
        "F04.02 must observe C return values, not a local let"
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
