//! ROADMAP L03.01: SHA-256 digest over bytes; known test vectors.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn sha256_vectors_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/sha256_vectors"),
        "missing stdlib/crypto/sha256_vectors fixture, got {ids:?}"
    );
}

#[test]
fn sha256_vectors_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/sha256_vectors")
        .expect("stdlib/crypto/sha256_vectors");
    assert_eq!(fixture.targets.len(), 2, "L03.01 targets both js and native");
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
fn sha256_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/sha256_invalid"),
        "missing stdlib/crypto/sha256_invalid fixture, got {ids:?}"
    );
}

#[test]
fn sha256_invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/sha256_invalid")
        .expect("stdlib/crypto/sha256_invalid");
    assert_eq!(fixture.targets.len(), 2, "L03.01 targets both js and native");
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
