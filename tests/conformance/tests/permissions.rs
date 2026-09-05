//! ROADMAP R02.04: default permission policy (permissive).
//! A Program with no explicit grant subset may use host fs and TCP.
//! ROADMAP R02.01: explicit grants for fs read/write and net listen/connect succeed.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn load_named(id: &str) -> draconic_conformance::Fixture {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    fixtures.iter().find(|f| f.id == id).cloned().expect(id)
}

fn assert_no_explicit_grant_subset(id: &str) {
    let fixture = load_named(id);
    assert!(
        fixture.expect_js.args.is_empty() && fixture.expect_native.args.is_empty(),
        "{id} must run with no CLI grant args (no explicit grant subset)"
    );
    assert!(
        !fixture.source.contains("--allow"),
        "{id} must not pass --allow flags"
    );
}

fn assert_fixture_runs_both(id: &str) {
    let fixture = load_named(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target js and native, got {:?}",
        fixture.targets
    );
    assert!(
        fixture.expect_native.stdout.is_some(),
        "{id} must assert native.stdout (not B08 hello stub)"
    );
    assert_eq!(fixture.expect_js.exit, 0, "{id} js must succeed by default");
    assert_eq!(
        fixture.expect_native.exit, 0,
        "{id} native must succeed by default"
    );
    for r in run_fixture(&fixture) {
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
fn default_fs_fixture_present() {
    assert_fixture_present("security/permissions/default_fs");
}

#[test]
fn default_fs_no_explicit_grant_subset() {
    assert_no_explicit_grant_subset("security/permissions/default_fs");
}

#[test]
fn default_fs_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/default_fs");
}

#[test]
fn default_net_fixture_present() {
    assert_fixture_present("security/permissions/default_net");
}

#[test]
fn default_net_no_explicit_grant_subset() {
    assert_no_explicit_grant_subset("security/permissions/default_net");
}

#[test]
fn default_net_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/default_net");
}

fn assert_explicit_grant_subset(id: &str, tokens: &[&str]) {
    let fixture = load_named(id);
    assert!(
        !fixture.grants.is_empty(),
        "{id} must declare an explicit grant subset"
    );
    for token in tokens {
        assert!(
            fixture.grants.iter().any(|g| g == token),
            "{id} must grant {token}, got {:?}",
            fixture.grants
        );
    }
}

#[test]
fn grant_fs_fixture_present() {
    assert_fixture_present("security/permissions/grant_fs");
}

#[test]
fn grant_fs_explicit_grant_subset() {
    assert_explicit_grant_subset("security/permissions/grant_fs", &["fs-read", "fs-write"]);
}

#[test]
fn grant_fs_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/grant_fs");
}

#[test]
fn grant_net_fixture_present() {
    assert_fixture_present("security/permissions/grant_net");
}

#[test]
fn grant_net_explicit_grant_subset() {
    assert_explicit_grant_subset(
        "security/permissions/grant_net",
        &["net-listen", "net-connect"],
    );
}

#[test]
fn grant_net_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/grant_net");
}
