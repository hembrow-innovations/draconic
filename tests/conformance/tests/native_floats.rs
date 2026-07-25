//! ROADMAP N02: native floats f32/f64 and boolean.

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
fn arith_f64_fixture_present() {
    assert_fixture_present("native/floats/arith_f64");
}

#[test]
fn arith_f64_runs_native() {
    assert_fixture_runs_native("native/floats/arith_f64");
}

#[test]
fn widths_fixture_present() {
    assert_fixture_present("native/floats/widths");
}

#[test]
fn widths_runs_native() {
    assert_fixture_runs_native("native/floats/widths");
}

#[test]
fn bool_basic_fixture_present() {
    assert_fixture_present("native/floats/bool_basic");
}

#[test]
fn bool_basic_runs_native() {
    assert_fixture_runs_native("native/floats/bool_basic");
}

#[test]
fn compare_if_fixture_present() {
    assert_fixture_present("native/floats/compare_if");
}

#[test]
fn compare_if_runs_native() {
    assert_fixture_runs_native("native/floats/compare_if");
}

#[test]
fn fn_call_fixture_present() {
    assert_fixture_present("native/floats/fn_call");
}

#[test]
fn fn_call_runs_native() {
    assert_fixture_runs_native("native/floats/fn_call");
}
