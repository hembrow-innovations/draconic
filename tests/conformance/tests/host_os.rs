//! ROADMAP H16 / H16.01–H16.03: cwd/chdir, hostname/osType/osArch, tempDir/homeDir.
//! H16 parent locks the combined OS-misc surface in one Program.

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

#[test]
fn temp_home_fixture_present() {
    assert_fixture_present("host/os/temp_home");
}

#[test]
fn temp_home_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/os/temp_home");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/os/surface");
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/os/surface")
        .expect("host/os/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "host/os/surface must target js and native"
    );
    for name in [
        "cwd", "chdir", "hostname", "osType", "osArch", "tempDir", "homeDir",
    ] {
        assert!(
            fixture.source.contains(name),
            "H16 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some(
            "string\ntrue\ntrue\nstring\nstring\nstring\ntrue\ntrue\ntrue\nstring\nstring\ntrue\ntrue\n",
        ),
        "H16 surface must observe cwd/chdir, hostname/osType/osArch, and tempDir/homeDir"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H16 surface must terminate with exit 0"
    );
    assert_fixture_runs_js_and_native("host/os/surface");
}
