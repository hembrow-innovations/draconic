//! ROADMAP H03 / H03.01–H03.03: pathJoin / pathNormalize / dirname / basename / extname / isAbsolute / pathResolve.
//! H03 parent locks the combined path-helper surface in one Program.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs(id: &str) {
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
fn path_join_fixture_present() {
    assert_fixture_present("host/path/path_join");
}

#[test]
fn path_join_runs_js_and_native() {
    assert_fixture_runs("host/path/path_join");
}

#[test]
fn path_normalize_fixture_present() {
    assert_fixture_present("host/path/path_normalize");
}

#[test]
fn path_normalize_runs_js_and_native() {
    assert_fixture_runs("host/path/path_normalize");
}

#[test]
fn path_dirname_fixture_present() {
    assert_fixture_present("host/path/path_dirname");
}

#[test]
fn path_dirname_runs_js_and_native() {
    assert_fixture_runs("host/path/path_dirname");
}

#[test]
fn path_basename_fixture_present() {
    assert_fixture_present("host/path/path_basename");
}

#[test]
fn path_basename_runs_js_and_native() {
    assert_fixture_runs("host/path/path_basename");
}

#[test]
fn path_extname_fixture_present() {
    assert_fixture_present("host/path/path_extname");
}

#[test]
fn path_extname_runs_js_and_native() {
    assert_fixture_runs("host/path/path_extname");
}

#[test]
fn path_is_absolute_fixture_present() {
    assert_fixture_present("host/path/path_is_absolute");
}

#[test]
fn path_is_absolute_runs_js_and_native() {
    assert_fixture_runs("host/path/path_is_absolute");
}

#[test]
fn path_resolve_fixture_present() {
    assert_fixture_present("host/path/path_resolve");
}

#[test]
fn path_resolve_runs_js_and_native() {
    assert_fixture_runs("host/path/path_resolve");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/path/surface");
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/path/surface")
        .expect("host/path/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "host/path/surface must target js and native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("foo/bar\nfoo/bar\n/foo/bar\nbaz.txt\n.html\ntrue\nfalse\n/foo/bar\ntrue\n"),
        "H03 surface must observe join, normalize, dirname, basename, extname, isAbsolute, and resolve"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H03 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/path/surface");
}
