//! ROADMAP F02 / F02.01–F02.02: export Draconic fn as C function pointer; host invoke.
//! F02 parent locks the combined surface in one Program.

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
fn export_fnptr_fixture_present() {
    assert_fixture_present("ffi/callback/export_fnptr");
}

#[test]
fn export_fnptr_runs_native() {
    assert_fixture_runs_native("ffi/callback/export_fnptr");
}

#[test]
fn invoke_scalar_fixture_present() {
    assert_fixture_present("ffi/callback/invoke_scalar");
}

#[test]
fn invoke_scalar_runs_native() {
    assert_fixture_runs_native("ffi/callback/invoke_scalar");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("ffi/callback/surface");
}

#[test]
fn surface_runs_native() {
    assert_fixture_runs_native("ffi/callback/surface");
}
