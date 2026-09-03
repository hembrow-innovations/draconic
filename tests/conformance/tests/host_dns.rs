//! ROADMAP H09 / H09.01–H09.02: DNS lookup + connect-by-name.
//! H09 parent locks the combined DNS surface in one Program.

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
fn dns_lookup_loopback_fixture_present() {
    assert_fixture_present("host/net/dns/dns_lookup_loopback");
}

#[test]
fn dns_lookup_loopback_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/dns/dns_lookup_loopback")
        .expect("host/net/dns/dns_lookup_loopback");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("1\n127.0.0.1\n")
    );
    assert_fixture_runs("host/net/dns/dns_lookup_loopback");
}

#[test]
fn dns_lookup_fail_runs_native() {
    assert_fixture_present("host/net/dns/dns_lookup_fail");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/dns/dns_lookup_fail")
        .expect("host/net/dns/dns_lookup_fail");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(fixture.expect_native.stderr.as_deref(), Some("EADDR\n"));
    assert_fixture_runs("host/net/dns/dns_lookup_fail");
}

#[test]
fn tcp_connect_by_name_runs_native() {
    assert_fixture_present("host/net/dns/tcp_connect_by_name");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/dns/tcp_connect_by_name")
        .expect("host/net/dns/tcp_connect_by_name");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("number\ntrue\n")
    );
    assert_fixture_runs("host/net/dns/tcp_connect_by_name");
}

#[test]
fn tcp_connect_by_name_fail_runs_native() {
    assert_fixture_present("host/net/dns/tcp_connect_by_name_fail");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/dns/tcp_connect_by_name_fail")
        .expect("host/net/dns/tcp_connect_by_name_fail");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(fixture.expect_native.stderr.as_deref(), Some("EADDR\n"));
    assert_fixture_runs("host/net/dns/tcp_connect_by_name_fail");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/net/dns/surface");
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/dns/surface")
        .expect("host/net/dns/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("1\n127.0.0.1\nnumber\ntrue\n"),
        "H09 surface must observe lookup addresses, connect-by-name typeof, and handle ok"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H09 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/net/dns/surface");
}
