//! ROADMAP H13 / H13.01: HTTP/2 preface + single-stream request/response.
//! H13 parent locks the combined HTTP/2 surface in one Program.

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
fn client_preface_runs_native() {
    assert_fixture_present("host/http2/client_preface");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http2/client_preface")
        .expect("host/http2/client_preface");
    assert!(fixture.targets.contains(&Target::Native));
    assert_eq!(fixture.expect_native.stdout.as_deref(), Some("33\n"));
    assert_fixture_runs("host/http2/client_preface");
}

#[test]
fn server_preface_runs_native() {
    assert_fixture_present("host/http2/server_preface");
    assert_fixture_runs("host/http2/server_preface");
}

#[test]
fn encode_parse_request_runs_native() {
    assert_fixture_present("host/http2/encode_parse_request");
    assert_fixture_runs("host/http2/encode_parse_request");
}

#[test]
fn encode_parse_response_runs_native() {
    assert_fixture_present("host/http2/encode_parse_response");
    assert_fixture_runs("host/http2/encode_parse_response");
}

#[test]
fn single_stream_e2e_runs_native() {
    assert_fixture_present("host/http2/single_stream_e2e");
    assert_fixture_runs("host/http2/single_stream_e2e");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/http2/surface");
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/http2/surface")
        .expect("host/http2/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    for name in [
        "http2ClientPreface",
        "http2ServerPreface",
        "http2SettingsAck",
        "http2EncodeRequest",
        "http2EncodeResponse",
        "http2ParseRequest",
        "http2ParseResponse",
        "http2ClientOpen",
        "http2ServerReply",
    ] {
        assert!(
            fixture.source.contains(name),
            "H13 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("GET\n/hello\nok-body\n/hello\n33\n9\n9\n"),
        "H13 surface must observe encode/parse, loopback open/reply, and preface lengths"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H13 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/http2/surface");
}
