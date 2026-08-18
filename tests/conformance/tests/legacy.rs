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

#[test]
fn arguments_mapped_fixture_present() {
    assert_fixture_present("es/legacy/arguments_mapped");
}

#[test]
fn arguments_mapped_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_mapped");
}

#[test]
fn delete_identifier_fixture_present() {
    assert_fixture_present("es/legacy/delete_identifier");
}

#[test]
fn delete_identifier_runs() {
    assert_fixture_runs_declared_targets("es/legacy/delete_identifier");
}

#[test]
fn duplicate_params_fixture_present() {
    assert_fixture_present("es/legacy/duplicate_params");
}

#[test]
fn duplicate_params_runs() {
    assert_fixture_runs_declared_targets("es/legacy/duplicate_params");
}

#[test]
fn function_caller_arguments_fixture_present() {
    assert_fixture_present("es/legacy/function_caller_arguments");
}

#[test]
fn function_caller_arguments_runs() {
    assert_fixture_runs_declared_targets("es/legacy/function_caller_arguments");
}

#[test]
fn sloppy_this_fixture_present() {
    assert_fixture_present("es/legacy/sloppy_this");
}

#[test]
fn sloppy_this_runs() {
    assert_fixture_runs_declared_targets("es/legacy/sloppy_this");
}

#[test]
fn implicit_global_fixture_present() {
    assert_fixture_present("es/legacy/implicit_global");
}

#[test]
fn implicit_global_runs() {
    assert_fixture_runs_declared_targets("es/legacy/implicit_global");
}

#[test]
fn future_reserved_idents_fixture_present() {
    assert_fixture_present("es/legacy/future_reserved_idents");
}

#[test]
fn future_reserved_idents_runs() {
    assert_fixture_runs_declared_targets("es/legacy/future_reserved_idents");
}

#[test]
fn for_in_of_implicit_global_fixture_present() {
    assert_fixture_present("es/legacy/for_in_of_implicit_global");
}

#[test]
fn for_in_of_implicit_global_runs() {
    assert_fixture_runs_declared_targets("es/legacy/for_in_of_implicit_global");
}
