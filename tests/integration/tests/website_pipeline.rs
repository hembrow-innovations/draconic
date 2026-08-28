//! Website pipeline seam (issues-20): compile the Draconic generator, run it
//! on one markdown page, assert the HTML contains the page title.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_path;

const PAGE_TITLE: &str = "UniqueTitleZ9q";

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-website-pipeline-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn build_generator() -> PathBuf {
    let src = repo_root().join("website/generate.drac");
    assert!(src.is_file(), "missing {}", src.display());
    let dir = temp_dir();
    let out = dir.join("generate");
    let module = compile_path(&src).expect("compile website/generate.drac");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("build_native_binary");
    assert!(out.is_file(), "binary missing at {}", out.display());
    out
}

#[test]
fn website_pipeline_one_page_html_contains_title() {
    let work = temp_dir();
    fs::create_dir_all(work.join("website")).unwrap();
    fs::write(
        work.join("website/page.md"),
        format!("# {PAGE_TITLE}\n\nA one-page fixture for the website pipeline.\n"),
    )
    .unwrap();

    let bin = build_generator();
    let output = Command::new(&bin)
        .current_dir(&work)
        .output()
        .expect("run generate");
    assert!(
        output.status.success(),
        "generate failed: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let html = fs::read_to_string(work.join("website/page.html")).expect("page.html");
    assert!(
        html.contains("<!DOCTYPE html>") || html.contains("<html"),
        "expected HTML document, got:\n{html}"
    );
    assert!(
        html.contains(PAGE_TITLE),
        "expected page title {PAGE_TITLE:?} in HTML, got:\n{html}"
    );
}
