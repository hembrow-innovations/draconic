//! ROADMAP L08 / L08.01 / L08.02: URL parse and query parse/serialize.
//! L08 parent locks the combined URL library surface in one Program.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

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
    assert_eq!(
        fixture.targets.len(),
        2,
        "L08.01 targets both js and native"
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
fn query_roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/url/query_roundtrip"),
        "missing stdlib/url/query_roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn query_roundtrip_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/url/query_roundtrip")
        .expect("stdlib/url/query_roundtrip");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L08.02 targets both js and native"
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
        ids.iter().any(|id| *id == "stdlib/url/surface"),
        "missing stdlib/url/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/url/surface")
        .expect("stdlib/url/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/url/surface must target js and native"
    );
    for name in [
        "parseUrl",
        "parseQuery",
        "serializeQuery",
        "scheme",
        "host",
        "path",
        "query",
        "hash",
    ] {
        assert!(
            fixture.source.contains(name),
            "L08 surface must use {name} in one Program"
        );
    }
    assert!(
        fixture.source.contains("parseQuery(") && fixture.source.contains("serializeQuery("),
        "L08 surface must parse and serialize query text in one Program"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\ntrue\n"),
        "L08 surface must observe URL parts and query round-trips"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L08 surface must terminate with exit 0"
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
