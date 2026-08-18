//! ROADMAP H11.01 / H11.02: TLS client + server wrap.

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
fn tls_client_plain_peer_fails_fixture_present() {
    assert_fixture_present("host/net/tls/tls_client_plain_peer_fails");
}

#[test]
fn tls_client_plain_peer_fails_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tls/tls_client_plain_peer_fails")
        .expect("host/net/tls/tls_client_plain_peer_fails");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tls/tls_client_plain_peer_fails");
}

#[test]
fn tls_server_missing_cert_fixture_present() {
    assert_fixture_present("host/net/tls/tls_server_missing_cert");
}

#[test]
fn tls_server_missing_cert_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tls/tls_server_missing_cert")
        .expect("host/net/tls/tls_server_missing_cert");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tls/tls_server_missing_cert");
}
