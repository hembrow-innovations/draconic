//! ROADMAP H13.01: HTTP/2 preface + single-stream request/response.

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
