//! ROADMAP H04.01–H04.06: whole-file + directory + rename/copy/delete + open handle host APIs.

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
fn read_file_text_fixture_present() {
    assert_fixture_present("host/fs/read_file_text");
}

#[test]
fn read_file_text_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/fs/read_file_text")
        .expect("host/fs/read_file_text");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "must target js and native"
    );
    assert_fixture_runs("host/fs/read_file_text");
}

#[test]
fn read_file_bytes_fixture_present() {
    assert_fixture_present("host/fs/read_file_bytes");
}

#[test]
fn read_file_bytes_runs() {
    assert_fixture_runs("host/fs/read_file_bytes");
}

#[test]
fn read_file_empty_fixture_present() {
    assert_fixture_present("host/fs/read_file_empty");
}

#[test]
fn read_file_empty_runs() {
    assert_fixture_runs("host/fs/read_file_empty");
}

#[test]
fn read_file_missing_js_typed_error() {
    assert_fixture_present("host/fs/read_file_missing");
    assert_fixture_runs("host/fs/read_file_missing");
}

#[test]
fn read_file_missing_native_enoent() {
    assert_fixture_present("host/fs/read_file_missing_native");
    assert_fixture_runs("host/fs/read_file_missing_native");
}

#[test]
fn write_file_text_runs() {
    assert_fixture_present("host/fs/write_file_text");
    assert_fixture_runs("host/fs/write_file_text");
}

#[test]
fn write_file_truncate_runs() {
    assert_fixture_present("host/fs/write_file_truncate");
    assert_fixture_runs("host/fs/write_file_truncate");
}

#[test]
fn append_file_text_runs() {
    assert_fixture_present("host/fs/append_file_text");
    assert_fixture_runs("host/fs/append_file_text");
}

#[test]
fn write_file_bytes_runs() {
    assert_fixture_present("host/fs/write_file_bytes");
    assert_fixture_runs("host/fs/write_file_bytes");
}

#[test]
fn write_file_empty_runs() {
    assert_fixture_present("host/fs/write_file_empty");
    assert_fixture_runs("host/fs/write_file_empty");
}

#[test]
fn write_file_missing_parent_js() {
    assert_fixture_present("host/fs/write_file_missing_parent");
    assert_fixture_runs("host/fs/write_file_missing_parent");
}

#[test]
fn exists_basic_runs() {
    assert_fixture_present("host/fs/exists_basic");
    assert_fixture_runs("host/fs/exists_basic");
}

#[test]
fn stat_file_runs() {
    assert_fixture_present("host/fs/stat_file");
    assert_fixture_runs("host/fs/stat_file");
}

#[test]
fn stat_dir_runs() {
    assert_fixture_present("host/fs/stat_dir");
    assert_fixture_runs("host/fs/stat_dir");
}

#[test]
fn stat_missing_js() {
    assert_fixture_present("host/fs/stat_missing");
    assert_fixture_runs("host/fs/stat_missing");
}

#[test]
fn stat_missing_native() {
    assert_fixture_present("host/fs/stat_missing_native");
    assert_fixture_runs("host/fs/stat_missing_native");
}

#[test]
fn mkdir_basic_runs() {
    assert_fixture_present("host/fs/mkdir_basic");
    assert_fixture_runs("host/fs/mkdir_basic");
}

#[test]
fn mkdir_all_runs() {
    assert_fixture_present("host/fs/mkdir_all");
    assert_fixture_runs("host/fs/mkdir_all");
}

#[test]
fn readdir_basic_runs() {
    assert_fixture_present("host/fs/readdir_basic");
    assert_fixture_runs("host/fs/readdir_basic");
}

#[test]
fn rmdir_basic_runs() {
    assert_fixture_present("host/fs/rmdir_basic");
    assert_fixture_runs("host/fs/rmdir_basic");
}

#[test]
fn remove_file_runs() {
    assert_fixture_present("host/fs/remove_file");
    assert_fixture_runs("host/fs/remove_file");
}

#[test]
fn rename_file_runs() {
    assert_fixture_present("host/fs/rename_file");
    assert_fixture_runs("host/fs/rename_file");
}

#[test]
fn copy_file_runs() {
    assert_fixture_present("host/fs/copy_file");
    assert_fixture_runs("host/fs/copy_file");
}

#[test]
fn delete_file_runs() {
    assert_fixture_present("host/fs/delete_file");
    assert_fixture_runs("host/fs/delete_file");
}

#[test]
fn open_handle_rw_runs() {
    assert_fixture_present("host/fs/open_handle_rw");
    assert_fixture_runs("host/fs/open_handle_rw");
}
