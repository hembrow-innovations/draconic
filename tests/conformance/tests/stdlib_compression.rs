//! ROADMAP L04: gzip/deflate compress and decompress of byte buffers.
//! Invalid or truncated input errors rather than silently corrupting.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn gzip_roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/compression/gzip_roundtrip"),
        "missing stdlib/compression/gzip_roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn gzip_roundtrip_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/compression/gzip_roundtrip")
        .expect("stdlib/compression/gzip_roundtrip");
    assert_eq!(fixture.targets.len(), 2, "L04 targets both js and native");
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
fn deflate_roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/compression/deflate_roundtrip"),
        "missing stdlib/compression/deflate_roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn deflate_roundtrip_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/compression/deflate_roundtrip")
        .expect("stdlib/compression/deflate_roundtrip");
    assert_eq!(fixture.targets.len(), 2, "L04 targets both js and native");
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
        ids.iter().any(|id| *id == "stdlib/compression/invalid"),
        "missing stdlib/compression/invalid fixture, got {ids:?}"
    );
}

#[test]
fn invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/compression/invalid")
        .expect("stdlib/compression/invalid");
    assert_eq!(fixture.targets.len(), 2, "L04 targets both js and native");
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
        ids.iter().any(|id| *id == "stdlib/compression/surface"),
        "missing stdlib/compression/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/compression/surface")
        .expect("stdlib/compression/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/compression/surface must target js and native"
    );
    for name in ["gzip", "gunzip", "deflate", "inflate", "TypeError"] {
        assert!(
            fixture.source.contains(name),
            "L04 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\ntrue\n1\n1\n1\n1\n"),
        "L04 surface must observe gzip/deflate round-trips and invalid-input errors"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L04 surface must terminate with exit 0"
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
