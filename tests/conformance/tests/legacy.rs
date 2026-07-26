//! ROADMAP E17: non-strict legacy fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_declared_targets(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        !fixture.targets.is_empty(),
        "{id} must declare at least one target"
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
fn with_basic_fixture_present() {
    assert_fixture_present("es/legacy/with_basic");
}

#[test]
fn with_basic_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_basic");
}

#[test]
fn with_nested_fixture_present() {
    assert_fixture_present("es/legacy/with_nested");
}

#[test]
fn with_nested_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_nested");
}

#[test]
fn arguments_callee_fixture_present() {
    assert_fixture_present("es/legacy/arguments_callee");
}

#[test]
fn arguments_callee_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_callee");
}
