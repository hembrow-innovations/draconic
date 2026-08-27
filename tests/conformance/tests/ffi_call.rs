//! ROADMAP F01.01: call extern C fn with i32 args/return.

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
fn call_i32_abs_fixture_present() {
    assert_fixture_present("ffi/call/call_i32_abs");
}

#[test]
fn call_i32_abs_runs_native() {
    assert_fixture_runs_native("ffi/call/call_i32_abs");
}

#[test]
fn call_i32_multi_fixture_present() {
    assert_fixture_present("ffi/call/call_i32_multi");
}

#[test]
fn call_i32_multi_runs_native() {
    assert_fixture_runs_native("ffi/call/call_i32_multi");
}
