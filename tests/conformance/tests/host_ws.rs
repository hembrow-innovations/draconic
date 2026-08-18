//! ROADMAP H12.01: WebSocket handshake (HTTP/1.1 upgrade) server-side.

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
fn ws_handshake_response_fixture_present() {
    assert_fixture_present("host/net/ws/ws_handshake_response");
}

#[test]
fn ws_handshake_response_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_handshake_response")
        .expect("host/net/ws/ws_handshake_response");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n\n"
        )
    );
    assert_fixture_runs("host/net/ws/ws_handshake_response");
}

#[test]
fn ws_handshake_empty_key_runs_native() {
    assert_fixture_present("host/net/ws/ws_handshake_empty_key");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_handshake_empty_key")
        .expect("host/net/ws/ws_handshake_empty_key");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EINVAL\n")
    );
    assert_fixture_runs("host/net/ws/ws_handshake_empty_key");
}

#[test]
fn ws_handshake_server_runs_native() {
    assert_fixture_present("host/net/ws/ws_handshake_server");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_handshake_server")
        .expect("host/net/ws/ws_handshake_server");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some(
            "/chat\nHTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n"
        )
    );
    assert_fixture_runs("host/net/ws/ws_handshake_server");
}
