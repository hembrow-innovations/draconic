//! ROADMAP R02 / R02.01 / R02.02 / R02.03 / R02.04: permission model.
//! R02 parent locks the combined grant/deny fs+net policy in one Program.
//! A Program with no explicit grant subset may use host fs and TCP.
//! ROADMAP R02.01: explicit grants for fs read/write and net listen/connect succeed.
//! ROADMAP R02.02: host op without a grant fails with a clear diagnostic.
//! ROADMAP R02.03: CLI `--allow-*` flags grant a subset (opt-in).

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

fn assert_lacks_grant(id: &str, missing: &str) {
    let fixture = load_named(id);
    assert!(
        !fixture.grants.iter().any(|g| g == missing),
        "{id} must not grant {missing}, got {:?}",
        fixture.grants
    );
}

fn assert_deny_js(id: &str) {
    let fixture = load_named(id);
    assert!(
        fixture.targets.contains(&Target::Js),
        "{id} must target js, got {:?}",
        fixture.targets
    );
    assert_eq!(fixture.expect_js.exit, 0, "{id} js catch path must exit 0");
    let check = fixture
        .expect_js
        .check
        .as_deref()
        .expect("{id} must lock js.check");
    assert!(
        check.contains("EPERM")
            && check.contains("HostError")
            && check.contains("permission denied"),
        "{id} js.check must lock HostError EPERM permission denied, got {check}"
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

fn assert_deny_native(id: &str) {
    let fixture = load_named(id);
    assert!(
        fixture.targets.contains(&Target::Native),
        "{id} must target native, got {:?}",
        fixture.targets
    );
    assert_eq!(
        fixture.expect_native.exit, 1,
        "{id} native deny must exit 1"
    );
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EPERM\n"),
        "{id} native stderr must be the EPERM diagnostic"
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
fn deny_fs_fixture_present() {
    assert_fixture_present("security/permissions/deny_fs");
}

#[test]
fn deny_fs_lacks_fs_read_grant() {
    assert_explicit_grant_subset("security/permissions/deny_fs", &["fs-write"]);
    assert_lacks_grant("security/permissions/deny_fs", "fs-read");
}

#[test]
fn deny_fs_clear_diagnostic_js() {
    assert_deny_js("security/permissions/deny_fs");
}

#[test]
fn deny_fs_native_fixture_present() {
    assert_fixture_present("security/permissions/deny_fs_native");
}

#[test]
fn deny_fs_native_lacks_fs_read_grant() {
    assert_explicit_grant_subset("security/permissions/deny_fs_native", &["fs-write"]);
    assert_lacks_grant("security/permissions/deny_fs_native", "fs-read");
}

#[test]
fn deny_fs_native_clear_diagnostic() {
    assert_deny_native("security/permissions/deny_fs_native");
}

#[test]
fn deny_net_fixture_present() {
    assert_fixture_present("security/permissions/deny_net");
}

#[test]
fn deny_net_lacks_net_listen_grant() {
    assert_explicit_grant_subset("security/permissions/deny_net", &["net-connect"]);
    assert_lacks_grant("security/permissions/deny_net", "net-listen");
}

#[test]
fn deny_net_clear_diagnostic_js() {
    assert_deny_js("security/permissions/deny_net");
}

#[test]
fn deny_net_native_fixture_present() {
    assert_fixture_present("security/permissions/deny_net_native");
}

#[test]
fn deny_net_native_lacks_net_listen_grant() {
    assert_explicit_grant_subset("security/permissions/deny_net_native", &["net-connect"]);
    assert_lacks_grant("security/permissions/deny_net_native", "net-listen");
}

#[test]
fn deny_net_native_clear_diagnostic() {
    assert_deny_native("security/permissions/deny_net_native");
}

fn assert_names_allow_flags(id: &str, flags: &[&str]) {
    let fixture = load_named(id);
    for flag in flags {
        assert!(
            fixture.source.contains(flag),
            "{id} must name CLI flag {flag}"
        );
    }
}

#[test]
fn allow_fs_fixture_present() {
    assert_fixture_present("security/permissions/allow_fs");
}

#[test]
fn allow_fs_names_cli_flags() {
    assert_names_allow_flags(
        "security/permissions/allow_fs",
        &["--allow-fs-read", "--allow-fs-write"],
    );
    assert_explicit_grant_subset("security/permissions/allow_fs", &["fs-read", "fs-write"]);
}

#[test]
fn allow_fs_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/allow_fs");
}

#[test]
fn allow_net_fixture_present() {
    assert_fixture_present("security/permissions/allow_net");
}

#[test]
fn allow_net_names_cli_flags() {
    assert_names_allow_flags(
        "security/permissions/allow_net",
        &["--allow-net-listen", "--allow-net-connect"],
    );
    assert_explicit_grant_subset(
        "security/permissions/allow_net",
        &["net-listen", "net-connect"],
    );
}

#[test]
fn allow_net_runs_both_targets() {
    assert_fixture_runs_both("security/permissions/allow_net");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("security/permissions/surface");
}

#[test]
fn surface_mixed_grant_deny_js() {
    let fixture = load_named("security/permissions/surface");
    assert!(
        fixture.targets.contains(&Target::Js),
        "security/permissions/surface must target js, got {:?}",
        fixture.targets
    );
    assert_explicit_grant_subset("security/permissions/surface", &["fs-read", "fs-write"]);
    assert_lacks_grant("security/permissions/surface", "net-listen");
    assert_lacks_grant("security/permissions/surface", "net-connect");
    for name in ["writeFileText", "readFileText", "tcpListen"] {
        assert!(
            fixture.source.contains(name),
            "R02 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_js.exit, 0,
        "R02 surface js catch path must exit 0"
    );
    let check = fixture
        .expect_js
        .check
        .as_deref()
        .expect("security/permissions/surface must lock js.check");
    assert!(
        check.contains("r02-fs")
            && check.contains("EPERM")
            && check.contains("HostError")
            && check.contains("permission denied"),
        "R02 surface js.check must lock granted fs and HostError EPERM permission denied, got {check}"
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
fn surface_native_grant_deny_fixtures() {
    // Native LLVM lowering is per host-API family: fs and TCP cannot share one
    // Program. The parent native policy is the grant + deny fixtures together.
    assert_fixture_runs_both("security/permissions/grant_fs");
    assert_fixture_runs_both("security/permissions/grant_net");
    assert_deny_native("security/permissions/deny_fs_native");
    assert_deny_native("security/permissions/deny_net_native");
}
