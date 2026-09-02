//! ROADMAP C03: `once` / thread-safe init; mutex only if Runtime internals need it.
//! ROADMAP C03.01: `once` / thread-safe init primitive.

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

#[test]
fn once_basic_fixture_present() {
    assert_fixture_present("concurrency/sync/once_basic");
}

#[test]
fn once_basic_runs_native() {
    assert_fixture_runs_native("concurrency/sync/once_basic");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("concurrency/sync/surface");
}

#[test]
fn surface_runs_native() {
    assert_fixture_runs_native("concurrency/sync/surface");
}
