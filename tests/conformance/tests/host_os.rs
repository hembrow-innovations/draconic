//! ROADMAP H16.01: cwd get + chdir.
//! ROADMAP H16.02: hostname / OS type / arch strings.

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
fn cwd_chdir_fixture_present() {
    assert_fixture_present("host/os/cwd_chdir");
}

#[test]
fn cwd_chdir_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/os/cwd_chdir");
}

#[test]
fn hostname_os_fixture_present() {
    assert_fixture_present("host/os/hostname_os");
}

#[test]
fn hostname_os_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/os/hostname_os");
}
