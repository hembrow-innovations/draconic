//! ROADMAP C06: shared-memory atomics (native-only integer buffer).
//! Allocate a shared integer buffer visible to a worker isolate; atomic
//! load, store, add, compare-exchange, wait, and notify. JS hard-errors.

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
        fixture.targets.contains(&Target::Native) && !fixture.targets.contains(&Target::Js),
        "{id} must target native only"
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

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && !fixture.targets.contains(&Target::Native),
        "{id} must target js only"
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
fn shared_ops_fixture_present() {
    assert_fixture_present("concurrency/atomics/shared_ops");
}

#[test]
fn shared_ops_runs_native() {
    assert_fixture_runs_native("concurrency/atomics/shared_ops");
}

#[test]
fn wait_notify_fixture_present() {
    assert_fixture_present("concurrency/atomics/wait_notify");
}

#[test]
fn wait_notify_runs_native() {
    assert_fixture_runs_native("concurrency/atomics/wait_notify");
}

#[test]
fn worker_share_fixture_present() {
    assert_fixture_present("concurrency/atomics/worker_share");
}

#[test]
fn worker_share_runs_native() {
    assert_fixture_runs_native("concurrency/atomics/worker_share");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("concurrency/atomics/surface");
}

#[test]
fn surface_runs_native() {
    assert_fixture_runs_native("concurrency/atomics/surface");
}

#[test]
fn js_hard_error_fixture_present() {
    assert_fixture_present("concurrency/atomics/js_hard_error");
}

#[test]
fn js_hard_error_on_js() {
    assert_fixture_runs_js("concurrency/atomics/js_hard_error");
}
