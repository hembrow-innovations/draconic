//! ROADMAP H07.02–H07.03: async TCP Promises; concurrent connections.

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
fn tcp_async_accept_connect_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_accept_connect");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_accept_connect")
        .expect("host/net/async/tcp_async_accept_connect");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_accept_connect");
}

#[test]
fn tcp_async_read_write_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_read_write");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_read_write")
        .expect("host/net/async/tcp_async_read_write");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_read_write");
}

#[test]
fn tcp_async_cancel_close_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_cancel_close");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_cancel_close")
        .expect("host/net/async/tcp_async_cancel_close");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_cancel_close");
}

#[test]
fn tcp_async_concurrent_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_concurrent");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_concurrent")
        .expect("host/net/async/tcp_async_concurrent");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_concurrent");
}
