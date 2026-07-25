//! ROADMAP E18.01+: Annex B fixtures on js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn escape_unescape_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/annex-b/escape_unescape"),
        "missing es/annex-b/escape_unescape fixture, got {ids:?}"
    );
}

#[test]
fn escape_unescape_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/annex-b/escape_unescape")
        .expect("es/annex-b/escape_unescape");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
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
fn object_proto_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/annex-b/object_proto"),
        "missing es/annex-b/object_proto fixture, got {ids:?}"
    );
}

#[test]
fn object_proto_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/annex-b/object_proto")
        .expect("es/annex-b/object_proto");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
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
