//! Website pipeline seam (issues-21, issues-22, issues-23, issues-24,
//! issues-25): compile the Draconic generator, run it on Learn and Reference
//! pages, assert nav, status, and markdown subset; extract shipped `drac`
//! fences and `draconic build` them. Learn and Reference skeletons are walkable.

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

/// Spec labels from issues-24: Install, from JavaScript, from systems, Dual
/// worlds, modules, native types, host I/O, packages.
const LEARN_CHAPTERS: &[(&str, &str)] = &[
    ("install.html", "Install"),
    ("from-javascript.html", "from JavaScript"),
    ("from-systems.html", "from systems"),
    ("dual-worlds.html", "Dual worlds"),
    ("modules.html", "modules"),
    ("native-types.html", "native types"),
    ("host-io.html", "host I/O"),
    ("packages.html", "packages"),
];

/// Spec labels from issues-25: CLI, types, Dual-world rules, host I/O, packages.
const REFERENCE_PAGES: &[(&str, &str)] = &[
    ("cli.html", "CLI"),
    ("types.html", "types"),
    ("dual-world-rules.html", "Dual-world rules"),
    ("reference-host-io.html", "host I/O"),
    ("reference-packages.html", "packages"),
];

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

fn draconic_bin() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin = repo_root().join("target").join(profile).join("draconic");
    assert!(
        bin.is_file(),
        "missing {} (build draconic-cli first)",
        bin.display()
    );
    bin
}

fn page_status_and_fences(src: &str) -> (String, Vec<(String, String)>) {
    let mut status = String::new();
    let mut fences = Vec::new();
    let mut in_front = false;
    let mut seen_fm = false;
    let mut in_fence = false;
    let mut lang = String::new();
    let mut body = String::new();
    for line in src.lines() {
        if in_front {
            if line == "---" {
                in_front = false;
            } else if let Some(rest) = line.strip_prefix("status:") {
                status = rest.trim().to_string();
            }
            continue;
        }
        if in_fence {
            if line.starts_with("```") {
                fences.push((std::mem::take(&mut lang), std::mem::take(&mut body)));
                in_fence = false;
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        if line == "---" && !seen_fm {
            in_front = true;
            seen_fm = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("```") {
            in_fence = true;
            lang = rest
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            body.clear();
        }
    }
    (status, fences)
}

fn ensure_learn_chapter_sources(work: &Path) {
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    for (href, label) in LEARN_CHAPTERS {
        let slug = href.trim_end_matches(".html");
        let path = website.join(format!("{slug}.md"));
        if !path.exists() {
            fs::write(
                &path,
                page(label, "learn", "not-yet", "Learn chapter stub."),
            )
            .unwrap();
        }
    }
}

fn ensure_reference_page_sources(work: &Path) {
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    for (href, label) in REFERENCE_PAGES {
        let slug = href.trim_end_matches(".html");
        let path = website.join(format!("{slug}.md"));
        if !path.exists() {
            fs::write(
                &path,
                page(label, "reference", "not-yet", "Reference page stub."),
            )
            .unwrap();
        }
    }
}

fn run_website_pipeline(work: &Path) -> Result<(), String> {
    ensure_learn_chapter_sources(work);
    ensure_reference_page_sources(work);
    let bin = build_generator();
    let output = Command::new(&bin)
        .current_dir(work)
        .output()
        .map_err(|e| format!("run generate: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "generate failed: status={:?} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let website = work.join("website");
    let mut fence_i = 0u32;
    let entries = fs::read_dir(&website).map_err(|e| format!("read website: {e}"))?;
    for ent in entries {
        let ent = ent.map_err(|e| format!("read website entry: {e}"))?;
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let src = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let (status, fences) = page_status_and_fences(&src);
        if status == "not-yet" && !fences.is_empty() {
            return Err(format!("not-yet page {} contains a fence", path.display()));
        }
        if status != "shipped" {
            continue;
        }
        for (lang, body) in fences {
            if lang != "drac" {
                continue;
            }
            let fence_dir = website.join(".fences");
            fs::create_dir_all(&fence_dir).map_err(|e| format!("mkdir fences: {e}"))?;
            let src_path = fence_dir.join(format!("fence-{fence_i}.drac"));
            let out_path = fence_dir.join(format!("fence-{fence_i}.js"));
            fs::write(&src_path, &body).map_err(|e| format!("write fence: {e}"))?;
            fence_i += 1;
            let built = Command::new(draconic_bin())
                .arg("build")
                .arg("--target")
                .arg("js")
                .arg(&src_path)
                .arg("-o")
                .arg(&out_path)
                .output()
                .map_err(|e| format!("draconic build: {e}"))?;
            if !built.status.success() {
                return Err(format!(
                    "draconic build failed for {}: status={:?} stdout={} stderr={}",
                    src_path.display(),
                    built.status,
                    String::from_utf8_lossy(&built.stdout),
                    String::from_utf8_lossy(&built.stderr)
                ));
            }
        }
    }
    Ok(())
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

    run_website_pipeline(&work).expect("pipeline");

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

    let reference =
        fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
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
        html.contains("<pre>") && html.contains("<code>") && html.contains(SUBSET_FENCE),
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
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "Reference fixture.",
        ),
    )
    .unwrap();

    run_website_pipeline(&work).expect("pipeline");

    let learn = fs::read_to_string(work.join("website/learn.html")).expect("learn.html");
    assert_nav(&learn);
    assert!(
        learn.contains("shipped"),
        "expected learn status shipped in HTML, got:\n{learn}"
    );
    assert_markdown_subset(&learn);

    let reference =
        fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
    assert_nav(&reference);
    assert!(
        reference.contains("not-yet"),
        "expected reference status not-yet in HTML, got:\n{reference}"
    );
}

#[test]
fn website_pipeline_shipped_drac_fence_builds() {
    let work = temp_dir();
    fs::create_dir_all(work.join("website")).unwrap();
    fs::write(
        work.join("website/learn.md"),
        page(
            LEARN_TITLE,
            "learn",
            "shipped",
            "```drac\nlet sample = 1 + 2;\n```\n",
        ),
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

    run_website_pipeline(&work).expect("pipeline");

    let learn = fs::read_to_string(work.join("website/learn.html")).expect("learn.html");
    assert_nav(&learn);
    assert!(
        learn.contains("shipped"),
        "expected learn status shipped in HTML, got:\n{learn}"
    );
    let built = work.join("website/.fences/fence-0.js");
    assert!(
        built.is_file(),
        "expected draconic build output at {}",
        built.display()
    );
}

#[test]
fn website_pipeline_shipped_invalid_drac_fence_fails() {
    let work = temp_dir();
    fs::create_dir_all(work.join("website")).unwrap();
    fs::write(
        work.join("website/learn.md"),
        page(
            LEARN_TITLE,
            "learn",
            "shipped",
            "```drac\nthis is not valid draconic !!!\n```\n",
        ),
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

    let err = run_website_pipeline(&work).expect_err("invalid shipped fence must fail build");
    assert!(
        err.contains("draconic build"),
        "expected draconic build failure, got: {err}"
    );
}

#[test]
fn website_pipeline_not_yet_page_with_fence_fails() {
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
            "```\nsneaky sample\n```\n",
        ),
    )
    .unwrap();

    let err = run_website_pipeline(&work).expect_err("not-yet fence must fail");
    assert!(
        err.contains("not-yet") && err.contains("fence"),
        "expected not-yet fence failure, got: {err}"
    );
}

#[test]
fn website_pipeline_not_yet_page_without_fence_generates() {
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

    run_website_pipeline(&work).expect("pipeline");

    let reference =
        fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
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

fn copy_repo_website_pages(work: &Path) {
    let src = repo_root().join("website");
    let dst = work.join("website");
    fs::create_dir_all(&dst).unwrap();
    for ent in fs::read_dir(&src).unwrap() {
        let ent = ent.unwrap();
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            fs::copy(&path, dst.join(ent.file_name())).unwrap();
        }
    }
}

fn assert_learn_chapter_nav(html: &str) {
    for (href, label) in LEARN_CHAPTERS {
        let needle = format!("<a href=\"{href}\">{label}</a>");
        assert!(
            html.contains(&needle),
            "expected Learn nav link {needle}, got:\n{html}"
        );
    }
}

fn assert_visible_status(html: &str, path: &str) {
    assert!(
        html.contains("<p class=\"status\">"),
        "expected visible status tag in {path}, got:\n{html}"
    );
    let shipped = html.contains("<p class=\"status\">shipped</p>");
    let not_yet = html.contains("<p class=\"status\">not-yet</p>");
    assert!(
        shipped || not_yet,
        "expected status shipped or not-yet in {path}, got:\n{html}"
    );
}

#[test]
fn website_pipeline_learn_skeleton_is_walkable() {
    let work = temp_dir();
    copy_repo_website_pages(&work);

    run_website_pipeline(&work).expect("pipeline");

    let learn = fs::read_to_string(work.join("website/learn.html")).expect("learn.html");
    assert_nav(&learn);
    assert_learn_chapter_nav(&learn);
    assert_visible_status(&learn, "learn.html");

    for (href, _) in LEARN_CHAPTERS {
        let html_path = work.join("website").join(href);
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|_| panic!("expected generated {}", html_path.display()));
        assert_nav(&html);
        assert_learn_chapter_nav(&html);
        assert_visible_status(&html, href);
    }

    let from_js = fs::read_to_string(work.join("website/from-javascript.html"))
        .expect("from-javascript.html");
    assert!(
        from_js.contains("href=\"dual-worlds.html\""),
        "JS landing must join at Dual worlds, got:\n{from_js}"
    );
    let from_sys =
        fs::read_to_string(work.join("website/from-systems.html")).expect("from-systems.html");
    assert!(
        from_sys.contains("href=\"dual-worlds.html\""),
        "systems landing must join at Dual worlds, got:\n{from_sys}"
    );
}

fn assert_reference_page_nav(html: &str) {
    for (href, label) in REFERENCE_PAGES {
        let needle = format!("<a href=\"{href}\">{label}</a>");
        assert!(
            html.contains(&needle),
            "expected Reference nav link {needle}, got:\n{html}"
        );
    }
}

#[test]
fn website_pipeline_reference_skeleton_is_walkable() {
    let work = temp_dir();
    copy_repo_website_pages(&work);

    run_website_pipeline(&work).expect("pipeline");

    let reference =
        fs::read_to_string(work.join("website/reference.html")).expect("reference.html");
    assert_nav(&reference);
    assert_reference_page_nav(&reference);
    assert_visible_status(&reference, "reference.html");

    for (href, _) in REFERENCE_PAGES {
        let html_path = work.join("website").join(href);
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|_| panic!("expected generated {}", html_path.display()));
        assert_nav(&html);
        assert_reference_page_nav(&html);
        assert_visible_status(&html, href);
    }
}
