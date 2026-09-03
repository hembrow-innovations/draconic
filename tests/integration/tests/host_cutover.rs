//! ROADMAP H17: Success Programs & host cutover.
//! Both example servers are pure Draconic native HTTP (no C host owns listen/accept).

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn assert_no_c_host(dir: &Path) {
    fn walk(path: &Path) {
        if path.is_dir() {
            for entry in
                fs::read_dir(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
            {
                walk(&entry.unwrap().path());
            }
            return;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(
            !matches!(ext.as_str(), "c" | "h" | "cc" | "cpp" | "cxx"),
            "C host file {} would own listen/accept (ADR-0008 / H17)",
            path.display()
        );
    }
    walk(dir);
}

/// H17: http-echo and todo serve from Draconic native Programs, not a C host.
#[test]
fn success_programs_are_pure_draconic_native_hosts() {
    let root = repo_root();
    let echo_src = root.join("examples/http-echo/main.drac");
    let todo_src = root.join("examples/todo/server.drac");
    assert!(echo_src.is_file(), "missing {}", echo_src.display());
    assert!(todo_src.is_file(), "missing {}", todo_src.display());

    let echo = fs::read_to_string(&echo_src).expect("read http-echo");
    let todo = fs::read_to_string(&todo_src).expect("read todo server");
    assert!(
        echo.contains("tcpListen"),
        "http-echo must listen in Draconic"
    );
    assert!(
        todo.contains("tcpListen"),
        "todo server must listen in Draconic"
    );
    assert!(echo.contains("8080"), "http-echo must bind 8080");
    assert!(
        todo.contains("18083"),
        "todo server must bind 18083 (not clash with http-echo)"
    );

    assert_no_c_host(&root.join("examples/http-echo"));
    assert_no_c_host(&root.join("examples/todo"));
}
