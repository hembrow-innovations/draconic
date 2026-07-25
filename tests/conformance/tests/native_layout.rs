//! ROADMAP N03.01–N03.02: native structs and fixed arrays (layout types).

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_native(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Native),
        "{id} must target native"
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
fn struct_basic_fixture_present() {
    assert_fixture_present("native/layout/struct_basic");
}

#[test]
fn struct_basic_runs_native() {
    assert_fixture_runs_native("native/layout/struct_basic");
}

#[test]
fn struct_fields_fixture_present() {
    assert_fixture_present("native/layout/struct_fields");
}

#[test]
fn struct_fields_runs_native() {
    assert_fixture_runs_native("native/layout/struct_fields");
}

#[test]
fn array_basic_fixture_present() {
    assert_fixture_present("native/layout/array_basic");
}

#[test]
fn array_basic_runs_native() {
    assert_fixture_runs_native("native/layout/array_basic");
}

#[test]
fn array_mixed_fixture_present() {
    assert_fixture_present("native/layout/array_mixed");
}

#[test]
fn array_mixed_runs_native() {
    assert_fixture_runs_native("native/layout/array_mixed");
}
