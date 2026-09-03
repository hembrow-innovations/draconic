//! ROADMAP H08 / H08.01–H08.02: UDP bind/sendto/recvfrom + loopback e2e.
//! H08 parent locks the combined UDP surface in one Program.

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
fn udp_bind_ephemeral_fixture_present() {
    assert_fixture_present("host/net/udp/udp_bind_ephemeral");
}

#[test]
fn udp_bind_ephemeral_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/udp/udp_bind_ephemeral")
        .expect("host/net/udp/udp_bind_ephemeral");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/udp/udp_bind_ephemeral");
}

#[test]
fn udp_sendto_recvfrom_runs_native() {
    assert_fixture_present("host/net/udp/udp_sendto_recvfrom");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/udp/udp_sendto_recvfrom")
        .expect("host/net/udp/udp_sendto_recvfrom");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.stdout.as_deref(), Some("udp-hi6\n"));
    assert_fixture_runs("host/net/udp/udp_sendto_recvfrom");
}

#[test]
fn udp_loopback_echo_runs_native() {
    assert_fixture_present("host/net/udp/udp_loopback_echo");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/udp/udp_loopback_echo")
        .expect("host/net/udp/udp_loopback_echo");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.stdout.as_deref(), Some("echo-me7\n"));
    assert_fixture_runs("host/net/udp/udp_loopback_echo");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/net/udp/surface");
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/udp/surface")
        .expect("host/net/udp/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("echo-menumber\ntrue\ntrue\n7\n"),
        "H08 surface must observe ephemeral port, loopback echo, and close"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H08 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/net/udp/surface");
}
