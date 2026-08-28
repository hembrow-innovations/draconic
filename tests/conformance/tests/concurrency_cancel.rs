//! ROADMAP C05.01: cancel token / Abort-like signal; abort propagates to linked tokens.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_js_and_native(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target js and native"
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
fn cancel_typeof_fixture_present() {
    assert_fixture_present("concurrency/cancel/cancel_typeof");
}

#[test]
fn cancel_typeof_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/cancel/cancel_typeof");
}

#[test]
fn cancel_abort_fixture_present() {
    assert_fixture_present("concurrency/cancel/cancel_abort");
}

#[test]
fn cancel_abort_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/cancel/cancel_abort");
}

#[test]
fn cancel_link_fixture_present() {
    assert_fixture_present("concurrency/cancel/cancel_link");
}

#[test]
fn cancel_link_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/cancel/cancel_link");
}
