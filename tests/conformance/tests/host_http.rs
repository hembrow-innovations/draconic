//! ROADMAP H10.01–H10.06: HTTP/1.1 parse, write, server, keep-alive, client, chunked.

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
fn parse_get_fixture_present() {
    assert_fixture_present("host/http/parse_get");
}

#[test]
fn parse_get_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_get")
        .expect("host/http/parse_get");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("GET\n/hello\nHTTP/1.1\n\nexample.com\n")
    );
    assert_fixture_runs("host/http/parse_get");
}

#[test]
fn parse_post_body_runs_native() {
    assert_fixture_present("host/http/parse_post_body");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_post_body")
        .expect("host/http/parse_post_body");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("POST\n/echo\nhello\n5\n")
    );
    assert_fixture_runs("host/http/parse_post_body");
}

#[test]
fn parse_malformed_runs_native() {
    assert_fixture_present("host/http/parse_malformed");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_malformed")
        .expect("host/http/parse_malformed");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EINVAL\n")
    );
    assert_fixture_runs("host/http/parse_malformed");
}

#[test]
fn write_ok_runs_native() {
    assert_fixture_present("host/http/write_ok");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_ok")
        .expect("host/http/write_ok");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello\n")
    );
    assert_fixture_runs("host/http/write_ok");
}

#[test]
fn write_default_reason_runs_native() {
    assert_fixture_present("host/http/write_default_reason");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_default_reason")
        .expect("host/http/write_default_reason");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n\n")
    );
    assert_fixture_runs("host/http/write_default_reason");
}

#[test]
fn write_bad_status_runs_native() {
    assert_fixture_present("host/http/write_bad_status");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_bad_status")
        .expect("host/http/write_bad_status");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(fixture.expect_native.exit, 1);
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EINVAL\n")
    );
    assert_fixture_runs("host/http/write_bad_status");
}

#[test]
fn server_oneshot_runs_native() {
    assert_fixture_present("host/http/server_oneshot");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/server_oneshot")
        .expect("host/http/server_oneshot");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 6\r\n\r\n/hello")
    );
    assert_fixture_runs("host/http/server_oneshot");
}

#[test]
fn keep_alive_runs_native() {
    assert_fixture_present("host/http/keep_alive");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/keep_alive")
        .expect("host/http/keep_alive");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: keep-alive\r\nContent-Length: 2\r\n\r\n/a\
HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\nContent-Length: 2\r\n\r\n/b"
        )
    );
    assert_fixture_runs("host/http/keep_alive");
}

#[test]
fn write_request_runs_native() {
    assert_fixture_present("host/http/write_request");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_request")
        .expect("host/http/write_request");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("GET /hello HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n\n")
    );
    assert_fixture_runs("host/http/write_request");
}

#[test]
fn write_request_post_runs_native() {
    assert_fixture_present("host/http/write_request_post");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_request_post")
        .expect("host/http/write_request_post");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some(
            "POST /echo HTTP/1.1\r\nHost: x\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nhi\n"
        )
    );
    assert_fixture_runs("host/http/write_request_post");
}

#[test]
fn parse_response_runs_native() {
    assert_fixture_present("host/http/parse_response");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_response")
        .expect("host/http/parse_response");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1\n200\nOK\nhello\ntext/plain\n")
    );
    assert_fixture_runs("host/http/parse_response");
}

#[test]
fn client_e2e_runs_native() {
    assert_fixture_present("host/http/client_e2e");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/client_e2e")
        .expect("host/http/client_e2e");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1\n200\nOK\n/hello\ntext/plain\n")
    );
    assert_fixture_runs("host/http/client_e2e");
}

#[test]
fn parse_chunked_runs_native() {
    assert_fixture_present("host/http/parse_chunked");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_chunked")
        .expect("host/http/parse_chunked");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("POST\n/up\nhello world\nchunked\n")
    );
    assert_fixture_runs("host/http/parse_chunked");
}

#[test]
fn write_chunked_runs_native() {
    assert_fixture_present("host/http/write_chunked");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/write_chunked")
        .expect("host/http/write_chunked");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n\n")
    );
    assert_fixture_runs("host/http/write_chunked");
}

#[test]
fn parse_response_chunked_runs_native() {
    assert_fixture_present("host/http/parse_response_chunked");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http/parse_response_chunked")
        .expect("host/http/parse_response_chunked");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("200\nfoo\n")
    );
    assert_fixture_runs("host/http/parse_response_chunked");
}
