//! ROADMAP H06.01–H06.05: TCP listen/accept/connect/peer + read/write/shutdown + loopback echo.

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
fn tcp_listen_ephemeral_fixture_present() {
    assert_fixture_present("host/net/tcp/tcp_listen_ephemeral");
}

#[test]
fn tcp_listen_ephemeral_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_listen_ephemeral")
        .expect("host/net/tcp/tcp_listen_ephemeral");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tcp/tcp_listen_ephemeral");
}

#[test]
fn tcp_listen_backlog_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_listen_backlog");
    assert_fixture_runs("host/net/tcp/tcp_listen_backlog");
}

#[test]
fn tcp_accept_peer_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_accept_peer");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_accept_peer")
        .expect("host/net/tcp/tcp_accept_peer");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tcp/tcp_accept_peer");
}

#[test]
fn tcp_connect_ok_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_connect_ok");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_connect_ok")
        .expect("host/net/tcp/tcp_connect_ok");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tcp/tcp_connect_ok");
}

#[test]
fn tcp_connect_refused_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_connect_refused");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_connect_refused")
        .expect("host/net/tcp/tcp_connect_refused");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1, "refused → exit 1");
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("ECONN\n"),
        "refused → stderr ECONN"
    );
    assert_fixture_runs("host/net/tcp/tcp_connect_refused");
}

#[test]
fn tcp_read_write_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_read_write");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_read_write")
        .expect("host/net/tcp/tcp_read_write");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("hello-tcpabcdef9\n3\n3\n0\n")
    );
    assert_fixture_runs("host/net/tcp/tcp_read_write");
}

#[test]
fn tcp_loopback_echo_runs_native() {
    assert_fixture_present("host/net/tcp/tcp_loopback_echo");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tcp/tcp_loopback_echo")
        .expect("host/net/tcp/tcp_loopback_echo");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("echo-me7\n")
    );
    assert_fixture_runs("host/net/tcp/tcp_loopback_echo");
}
