//! ROADMAP L01.01: UTF-8 encode/decode stdlib fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn utf8_roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/encoding/utf8_roundtrip"),
        "missing stdlib/encoding/utf8_roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn utf8_roundtrip_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/encoding/utf8_roundtrip")
        .expect("stdlib/encoding/utf8_roundtrip");
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
fn utf8_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/encoding/utf8_invalid"),
        "missing stdlib/encoding/utf8_invalid fixture, got {ids:?}"
    );
}

#[test]
fn utf8_invalid_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/encoding/utf8_invalid")
        .expect("stdlib/encoding/utf8_invalid");
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
