//! ROADMAP L09: MIME multipart parse and serialize for HTTP-shaped programs.
//! Invalid or truncated input errors rather than silently dropping parts.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn parse_parts_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/mime/parse_parts"),
        "missing stdlib/mime/parse_parts fixture, got {ids:?}"
    );
}

#[test]
fn parse_parts_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/mime/parse_parts")
        .expect("stdlib/mime/parse_parts");
    assert_eq!(fixture.targets.len(), 2, "L09 targets both js and native");
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
fn roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/mime/roundtrip"),
        "missing stdlib/mime/roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn roundtrip_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/mime/roundtrip")
        .expect("stdlib/mime/roundtrip");
    assert_eq!(fixture.targets.len(), 2, "L09 targets both js and native");
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
        ids.iter().any(|id| *id == "stdlib/mime/invalid"),
        "missing stdlib/mime/invalid fixture, got {ids:?}"
    );
}

#[test]
fn invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/mime/invalid")
        .expect("stdlib/mime/invalid");
    assert_eq!(fixture.targets.len(), 2, "L09 targets both js and native");
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
        ids.iter().any(|id| *id == "stdlib/mime/surface"),
        "missing stdlib/mime/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/mime/surface")
        .expect("stdlib/mime/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/mime/surface must target js and native"
    );
    for name in ["parseMultipart", "serializeMultipart", "TypeError"] {
        assert!(
            fixture.source.contains(name),
            "L09 surface must use {name} in one Program"
        );
    }
    assert!(
        fixture.source.contains("parseMultipart(")
            && fixture.source.contains("serializeMultipart("),
        "L09 surface must parse and serialize multipart text in one Program"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\n1\n1\n"),
        "L09 surface must observe parse/serialize round-trips and invalid-input errors"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L09 surface must terminate with exit 0"
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
