//! ROADMAP H10.01: HTTP/1.1 request parse — line + headers + Content-Length body.

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
