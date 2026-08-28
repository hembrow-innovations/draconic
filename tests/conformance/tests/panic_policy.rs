//! ROADMAP R04.01: catchable exceptions — user `throw` is handled by `try`/`catch`
//! on native (process continues; not abort).

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
    assert!(
        fixture.expect_native.stdout.is_some(),
        "{id} must assert native.stdout (not B08 hello stub)"
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
fn catchable_exceptions_fixture_present() {
    assert_fixture_present("security/panic_policy/catchable_exceptions");
}

#[test]
fn catchable_exceptions_runs_native() {
    assert_fixture_runs_native("security/panic_policy/catchable_exceptions");
}
