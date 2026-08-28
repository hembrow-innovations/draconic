//! ROADMAP C01.01: spawn worker isolate from module path or fn entry.
//! ROADMAP C01.02: join worker — wait for exit; capture result/error.

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
fn spawn_fn_fixture_present() {
    assert_fixture_present("concurrency/workers/spawn_fn");
}

#[test]
fn spawn_fn_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/workers/spawn_fn");
}

#[test]
fn spawn_module_fixture_present() {
    assert_fixture_present("concurrency/workers/spawn_module");
}

#[test]
fn spawn_module_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/workers/spawn_module");
}

#[test]
fn join_fn_fixture_present() {
    assert_fixture_present("concurrency/workers/join_fn");
}

#[test]
fn join_fn_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/workers/join_fn");
}

#[test]
fn join_module_fixture_present() {
    assert_fixture_present("concurrency/workers/join_module");
}

#[test]
fn join_module_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/workers/join_module");
}
