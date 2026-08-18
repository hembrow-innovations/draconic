//! ROADMAP H12.01–H12.03: WebSocket handshake + frames + client dial echo.

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

#[test]
fn ws_frame_text_runs_native() {
    assert_fixture_present("host/net/ws/ws_frame_text");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_frame_text")
        .expect("host/net/ws/ws_frame_text");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("7\n1\n1\n-1\nHello\n")
    );
    assert_fixture_runs("host/net/ws/ws_frame_text");
}

#[test]
fn ws_frame_binary_runs_native() {
    assert_fixture_present("host/net/ws/ws_frame_binary");
    assert_fixture_runs("host/net/ws/ws_frame_binary");
}

#[test]
fn ws_frame_close_runs_native() {
    assert_fixture_present("host/net/ws/ws_frame_close");
    assert_fixture_runs("host/net/ws/ws_frame_close");
}

#[test]
fn ws_frame_ping_pong_runs_native() {
    assert_fixture_present("host/net/ws/ws_frame_ping_pong");
    assert_fixture_runs("host/net/ws/ws_frame_ping_pong");
}

#[test]
fn ws_frame_bad_runs_native() {
    assert_fixture_present("host/net/ws/ws_frame_bad");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_frame_bad")
        .expect("host/net/ws/ws_frame_bad");
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EINVAL\n")
    );
    assert_fixture_runs("host/net/ws/ws_frame_bad");
}

#[test]
fn ws_client_echo_runs_native() {
    assert_fixture_present("host/net/ws/ws_client_echo");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_client_echo")
        .expect("host/net/ws/ws_client_echo");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.stdout.as_deref(), Some("hello\n"));
    assert_fixture_runs("host/net/ws/ws_client_echo");
}

#[test]
fn ws_client_bad_accept_runs_native() {
    assert_fixture_present("host/net/ws/ws_client_bad_accept");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/ws/ws_client_bad_accept")
        .expect("host/net/ws/ws_client_bad_accept");
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EINVAL\n")
    );
    assert_fixture_runs("host/net/ws/ws_client_bad_accept");
}
