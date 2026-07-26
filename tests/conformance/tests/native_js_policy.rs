//! ROADMAP N04: JS lowering/polyfill or hard-error policy per native feature.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js),
        "{id} must target js"
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
fn ptr_hard_error_fixture_present() {
    assert_fixture_present("native/js-policy/ptr_hard_error");
}

#[test]
fn ptr_hard_error_on_js() {
    assert_fixture_runs_js("native/js-policy/ptr_hard_error");
}

#[test]
fn ptr_store_hard_error_fixture_present() {
    assert_fixture_present("native/js-policy/ptr_store_hard_error");
}

#[test]
fn ptr_store_hard_error_on_js() {
    assert_fixture_runs_js("native/js-policy/ptr_store_hard_error");
}

#[test]
fn scalar_polyfill_fixture_present() {
    assert_fixture_present("native/js-policy/scalar_polyfill");
}

#[test]
fn scalar_polyfill_on_js() {
    assert_fixture_runs_js("native/js-policy/scalar_polyfill");
}

#[test]
fn struct_polyfill_fixture_present() {
    assert_fixture_present("native/js-policy/struct_polyfill");
}

#[test]
fn struct_polyfill_on_js() {
    assert_fixture_runs_js("native/js-policy/struct_polyfill");
}

#[test]
fn array_polyfill_fixture_present() {
    assert_fixture_present("native/js-policy/array_polyfill");
}

#[test]
fn array_polyfill_on_js() {
    assert_fixture_runs_js("native/js-policy/array_polyfill");
}
