//! ROADMAP E07: string fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn string_lit_access_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/strings/string_lit_access"),
        "missing es/strings/string_lit_access fixture, got {ids:?}"
    );
}

#[test]
fn string_lit_access_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/strings/string_lit_access")
        .expect("es/strings/string_lit_access");
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
fn template_lit_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/strings/template_lit"),
        "missing es/strings/template_lit fixture, got {ids:?}"
    );
}

#[test]
fn template_lit_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/strings/template_lit")
        .expect("es/strings/template_lit");
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
fn unicode_escapes_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/strings/unicode_escapes"),
        "missing es/strings/unicode_escapes fixture, got {ids:?}"
    );
}

#[test]
fn unicode_escapes_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/strings/unicode_escapes")
        .expect("es/strings/unicode_escapes");
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
fn tagged_template_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/strings/tagged_template"),
        "missing es/strings/tagged_template fixture, got {ids:?}"
    );
}

#[test]
fn tagged_template_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/strings/tagged_template")
        .expect("es/strings/tagged_template");
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
fn utf16_semantics_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/strings/utf16_semantics"),
        "missing es/strings/utf16_semantics fixture, got {ids:?}"
    );
}

#[test]
fn utf16_semantics_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/strings/utf16_semantics")
        .expect("es/strings/utf16_semantics");
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
