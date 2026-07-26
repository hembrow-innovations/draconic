//! ROADMAP E04: object fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn object_lit_access_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/object_lit_access"),
        "missing es/objects/object_lit_access fixture, got {ids:?}"
    );
}

#[test]
fn object_lit_access_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/object_lit_access")
        .expect("es/objects/object_lit_access");
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
fn property_assign_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/property_assign"),
        "missing es/objects/property_assign fixture, got {ids:?}"
    );
}

#[test]
fn property_assign_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/property_assign")
        .expect("es/objects/property_assign");
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
fn property_compound_assign_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/objects/property_compound_assign"),
        "missing es/objects/property_compound_assign fixture, got {ids:?}"
    );
}

#[test]
fn property_compound_assign_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/property_compound_assign")
        .expect("es/objects/property_compound_assign");
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
fn this_method_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/this_method"),
        "missing es/objects/this_method fixture, got {ids:?}"
    );
}

#[test]
fn this_method_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/this_method")
        .expect("es/objects/this_method");
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
fn new_ctor_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/new_ctor"),
        "missing es/objects/new_ctor fixture, got {ids:?}"
    );
}

#[test]
fn new_ctor_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/new_ctor")
        .expect("es/objects/new_ctor");
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
fn object_lit_sugar_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/object_lit_sugar"),
        "missing es/objects/object_lit_sugar fixture, got {ids:?}"
    );
}

#[test]
fn object_lit_sugar_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/object_lit_sugar")
        .expect("es/objects/object_lit_sugar");
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
fn prototype_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/objects/prototype"),
        "missing es/objects/prototype fixture, got {ids:?}"
    );
}

#[test]
fn prototype_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/objects/prototype")
        .expect("es/objects/prototype");
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
