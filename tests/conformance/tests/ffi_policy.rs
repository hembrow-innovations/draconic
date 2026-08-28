//! ROADMAP F08.01: FFI/extern on js → hard diagnostic (N04 spirit).
//! ROADMAP F08.02: clear spans + codes for bad extern signatures / unsupported types.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
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

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js),
        "{id} must target js"
    );
    assert_fixture_runs(id);
}

fn assert_fixture_runs_both(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target both js and native, got {:?}",
        fixture.targets
    );
    assert_fixture_runs(id);
}

#[test]
fn extern_js_hard_error_fixture_present() {
    assert_fixture_present("ffi/policy/extern_js_hard_error");
}

#[test]
fn extern_js_hard_error_on_js() {
    assert_fixture_runs_js("ffi/policy/extern_js_hard_error");
}

#[test]
fn bad_param_string_fixture_present() {
    assert_fixture_present("ffi/policy/bad_param_string");
}

#[test]
fn bad_param_string_both_targets() {
    assert_fixture_runs_both("ffi/policy/bad_param_string");
}

#[test]
fn bad_return_string_fixture_present() {
    assert_fixture_present("ffi/policy/bad_return_string");
}

#[test]
fn bad_return_string_both_targets() {
    assert_fixture_runs_both("ffi/policy/bad_return_string");
}

#[test]
fn rest_param_fixture_present() {
    assert_fixture_present("ffi/policy/rest_param");
}

#[test]
fn rest_param_both_targets() {
    assert_fixture_runs_both("ffi/policy/rest_param");
}
