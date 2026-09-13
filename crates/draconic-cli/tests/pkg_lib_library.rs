//! `toolchain.cli:build-js-library-esm` on git library source (`examples/pkg-lib`).
//! Node imports the `--library` artifact. Does not feed that JS file to the Linker.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn pkg_lib_dir() -> PathBuf {
    repo_root().join("examples/pkg-lib")
}

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg_lib_library-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git_ok(args: &[&str], cwd: &Path) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Draconic Test")
        .env("GIT_AUTHOR_EMAIL", "test@draconic.local")
        .env("GIT_COMMITTER_NAME", "Draconic Test")
        .env("GIT_COMMITTER_EMAIL", "test@draconic.local")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn tagged_git_library_source(root: &Path) -> PathBuf {
    let src = pkg_lib_dir();
    assert!(
        src.is_dir(),
        "examples/pkg-lib missing at {}",
        src.display()
    );
    let repo = root.join("pkg-lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    for name in ["draconic.toml", "index.drac", "README.md"] {
        let from = src.join(name);
        if from.is_file() {
            fs::copy(&from, repo.join(name)).unwrap();
        }
    }
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v0.1.0"], &repo);
    git_ok(&["tag", "v0.1.0"], &repo);
    repo
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    (
        output.status.code().unwrap_or(1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn pkg_lib_library() {
    let root = uniq_dir("js");
    let lib = tagged_git_library_source(&root);
    let src = lib.join("index.drac");
    let out = root.join("pkg-lib.mjs");
    let examples_scratch = pkg_lib_dir().join("index.out.js");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library build failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(out.is_file(), "build -o must write {}", out.display());
    assert!(
        !examples_scratch.exists(),
        "must not emit scratch next to examples: {}",
        examples_scratch.display()
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "import {{ VERSION, greet }} from '{url}';\
         if (VERSION !== '0.1.0') process.exit(1);\
         if (greet('world') !== 'hello, world') process.exit(2);\
         console.log('ok');"
    );
    let (ncode, nout, nerr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(&script));
    assert_eq!(
        ncode, 0,
        "Node import {{ VERSION, greet }} failed\nstdout={nout}\nstderr={nerr}"
    );
    assert!(nout.contains("ok"), "stdout={nout}");

    let _ = fs::remove_dir_all(&root);
}
