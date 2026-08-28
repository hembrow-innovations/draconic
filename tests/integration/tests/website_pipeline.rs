//! Website pipeline seam (issues-21, issues-22): compile the Draconic generator,
//! run it on Learn and Reference pages, assert nav, status, and markdown subset.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_path;

const LEARN_TITLE: &str = "UniqueLearnTitleZ9q";
const REFERENCE_TITLE: &str = "UniqueRefTitleK3w";
const SUBSET_HEADING: &str = "MdSubsetHeadingQ7x";
const SUBSET_PARA: &str = "MdSubsetParaW2n";
const SUBSET_LIST: &str = "MdSubsetListJ8k";
const SUBSET_FENCE: &str = "MdSubsetFenceR4p";
const SUBSET_LINK_TEXT: &str = "MdSubsetLinkY1c";
const SUBSET_LINK_HREF: &str = "https://example.com/md-subset-z5";

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

fn page(title: &str, section: &str, status: &str, body: &str) -> String {
    format!(
        "---\ntitle: {title}\nsection: {section}\nstatus: {status}\n---\n\n# {title}\n\n{body}\n"
    )
}

fn assert_nav(html: &str) {
    assert!(
        html.contains("<a href=\"learn.html\">Learn</a>"),
        "expected Learn nav link, got:\n{html}"
    );
    assert!(
        html.contains("<a href=\"reference.html\">Reference</a>"),
        "expected Reference nav link, got:\n{html}"
    );
}

#[test]
fn website_pipeline_learn_and_reference_nav_and_status() {
    let work = temp_dir();
    fs::create_dir_all(work.join("website")).unwrap();
    fs::write(
        work.join("website/learn.md"),
        page(LEARN_TITLE, "learn", "shipped", "Learn fixture."),
    )
    .unwrap();
    fs::write(
        work.join("website/reference.md"),
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "Reference fixture.",
        ),
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

    let learn = fs::read_to_string(work.join("website/learn.html")).expect("learn.html");
    assert!(
        learn.contains("<!DOCTYPE html>") || learn.contains("<html"),
        "expected HTML document, got:\n{learn}"
    );
    assert_nav(&learn);
    assert!(
        learn.contains(LEARN_TITLE),
        "expected learn title {LEARN_TITLE:?} in HTML, got:\n{learn}"
    );
    assert!(
        learn.contains("shipped"),
        "expected learn status shipped in HTML, got:\n{learn}"
    );

    let reference = fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
    assert!(
        reference.contains("<!DOCTYPE html>") || reference.contains("<html"),
        "expected HTML document, got:\n{reference}"
    );
    assert_nav(&reference);
    assert!(
        reference.contains(REFERENCE_TITLE),
        "expected reference title {REFERENCE_TITLE:?} in HTML, got:\n{reference}"
    );
    assert!(
        reference.contains("not-yet"),
        "expected reference status not-yet in HTML, got:\n{reference}"
    );
}

fn subset_body() -> String {
    format!(
        "## {SUBSET_HEADING}\n\n{SUBSET_PARA} with a [{SUBSET_LINK_TEXT}]({SUBSET_LINK_HREF}).\n\n- {SUBSET_LIST}\n\n```\n{SUBSET_FENCE}\n```\n"
    )
}

fn assert_markdown_subset(html: &str) {
    assert!(
        html.contains(&format!("<h2>{SUBSET_HEADING}</h2>")),
        "expected heading {SUBSET_HEADING:?} as h2, got:\n{html}"
    );
    assert!(
        html.contains(&format!("<p>{SUBSET_PARA}")),
        "expected paragraph wrapping {SUBSET_PARA:?}, got:\n{html}"
    );
    assert!(
        html.contains("<ul>") && html.contains(&format!("<li>{SUBSET_LIST}</li>")),
        "expected list item {SUBSET_LIST:?} in HTML, got:\n{html}"
    );
    assert!(
        html.contains("<pre>")
            && html.contains("<code>")
            && html.contains(SUBSET_FENCE),
        "expected fenced code {SUBSET_FENCE:?} in HTML, got:\n{html}"
    );
    assert!(
        html.contains(&format!(
            "<a href=\"{SUBSET_LINK_HREF}\">{SUBSET_LINK_TEXT}</a>"
        )),
        "expected link {SUBSET_LINK_TEXT:?} -> {SUBSET_LINK_HREF:?}, got:\n{html}"
    );
}

#[test]
fn website_pipeline_renders_markdown_subset() {
    let work = temp_dir();
    fs::create_dir_all(work.join("website")).unwrap();
    fs::write(
        work.join("website/learn.md"),
        page(LEARN_TITLE, "learn", "shipped", &subset_body()),
    )
    .unwrap();
    fs::write(
        work.join("website/reference.md"),
        page(REFERENCE_TITLE, "reference", "not-yet", "Reference fixture."),
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

    let learn = fs::read_to_string(work.join("website/learn.html")).expect("learn.html");
    assert_nav(&learn);
    assert!(
        learn.contains("shipped"),
        "expected learn status shipped in HTML, got:\n{learn}"
    );
    assert_markdown_subset(&learn);

    let reference = fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
    assert_nav(&reference);
    assert!(
        reference.contains("not-yet"),
        "expected reference status not-yet in HTML, got:\n{reference}"
    );
}
