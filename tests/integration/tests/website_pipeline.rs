//! Website pipeline seam: Start static publish, Learn and Reference HTML, and
//! fence compile. Shipped `drac` fences must build (`public-site.fences:shipped-must-build`).
//! Not-yet pages must not contain fences (`public-site.fences:forbid-not-yet-fences`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const LEARN_TITLE: &str = "UniqueLearnTitleZ9q";
const REFERENCE_TITLE: &str = "UniqueRefTitleK3w";

/// Spec labels from issues-24: Install, from JavaScript, from systems, Dual
/// worlds, modules, native types, host I/O, packages.
const LEARN_CHAPTERS: &[(&str, &str)] = &[
    ("install", "Install"),
    ("from-javascript", "from JavaScript"),
    ("from-systems", "from systems"),
    ("dual-worlds", "Dual worlds"),
    ("modules", "modules"),
    ("native-types", "native types"),
    ("host-io", "host I/O"),
    ("packages", "packages"),
];

/// Spec labels from issues-25: CLI, types, Dual-world rules, host I/O, packages.
const REFERENCE_PAGES: &[(&str, &str)] = &[
    ("cli", "CLI"),
    ("types", "types"),
    ("dual-world-rules", "Dual-world rules"),
    ("reference-host-io", "host I/O"),
    ("reference-packages", "packages"),
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

fn page(title: &str, section: &str, status: &str, body: &str) -> String {
    format!("---\ntitle: {title}\nsection: {section}\nstatus: {status}\n---\n\n# {title}\n\n{body}\n")
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

fn check_fences(website: &Path) -> Result<PathBuf, String> {
    let fence_dir = temp_dir().join(".fences");
    let mut fence_i = 0u32;
    let entries = fs::read_dir(website).map_err(|e| format!("read website: {e}"))?;
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
    Ok(fence_dir)
}

fn published_pages() -> &'static PathBuf {
    static DIST: OnceLock<PathBuf> = OnceLock::new();
    DIST.get_or_init(|| {
        let out = temp_dir().join("pages");
        let script = repo_root().join("scripts/generate-website.sh");
        let output = Command::new("bash")
            .arg(&script)
            .arg("--out")
            .arg(&out)
            .current_dir(repo_root())
            .output()
            .expect("run generate-website.sh");
        assert!(
            output.status.success(),
            "generate-website.sh failed: status={:?} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        out
    })
}

fn published_page(slug: &str) -> (PathBuf, String) {
    let dist = published_pages();
    let candidates = if slug.is_empty() || slug == "index" {
        vec![dist.join("index.html")]
    } else {
        vec![
            dist.join(format!("{slug}.html")),
            dist.join(slug).join("index.html"),
        ]
    };
    for path in candidates {
        if path.is_file() {
            let html = fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("read {}", path.display()));
            return (path, html);
        }
    }
    panic!(
        "expected published HTML for {slug} under {}",
        dist.display()
    );
}

fn contains_href(html: &str, slug: &str) -> bool {
    [
        format!("href=\"/{slug}\""),
        format!("href=\"/{slug}/\""),
        format!("href=\"/draconic/{slug}\""),
        format!("href=\"/draconic/{slug}/\""),
    ]
    .iter()
    .any(|needle| html.contains(needle.as_str()))
}

fn contains_labeled_link(html: &str, slug: &str, label: &str) -> bool {
    contains_href(html, slug)
        && (html.contains(&format!(">{label}</a>")) || html.contains(&format!(">{label}<")))
}

fn assert_nav(html: &str) {
    assert!(
        contains_labeled_link(html, "learn", "Learn"),
        "expected Learn nav link, got:\n{html}"
    );
    assert!(
        contains_labeled_link(html, "reference", "Reference"),
        "expected Reference nav link, got:\n{html}"
    );
}

fn assert_visible_status(html: &str, path: &str) {
    let shipped = html.contains("shipped");
    let not_yet = html.contains("not-yet");
    assert!(
        shipped || not_yet,
        "expected visible status shipped or not-yet in {path}, got:\n{html}"
    );
}

fn assert_html_document(html: &str) {
    assert!(
        html.contains("<!DOCTYPE html>") || html.contains("<html"),
        "expected HTML document, got:\n{html}"
    );
}

#[test]
fn website_pipeline_learn_and_reference_nav_and_status() {
    check_fences(&repo_root().join("website/content")).expect("repo fences");

    let (_, learn) = published_page("learn");
    assert_html_document(&learn);
    assert_nav(&learn);
    assert!(
        learn.contains("Learn"),
        "expected learn title in HTML, got:\n{learn}"
    );
    assert!(
        learn.contains("shipped"),
        "expected learn status shipped in HTML, got:\n{learn}"
    );

    let (_, reference) = published_page("reference");
    assert_html_document(&reference);
    assert_nav(&reference);
    assert!(
        reference.contains("Reference"),
        "expected reference title in HTML, got:\n{reference}"
    );
    assert_visible_status(&reference, "reference");
}

fn assert_markdown_subset(html: &str) {
    assert!(
        html.contains("<h1>") && html.contains("Install"),
        "expected heading Install as h1, got:\n{html}"
    );
    assert!(
        html.contains("<h2") && html.contains("Reproducibility"),
        "expected heading Reproducibility as h2, got:\n{html}"
    );
    assert!(
        html.contains("<p>") && html.contains("Get the toolchain"),
        "expected paragraph wrapping Get the toolchain, got:\n{html}"
    );
    assert!(
        html.contains("<ul>") && html.contains("<li>") && html.contains("linux/amd64"),
        "expected list item linux/amd64 in HTML, got:\n{html}"
    );
    assert!(
        html.contains("<pre") && html.contains("<code") && html.contains("hello.drac"),
        "expected fenced code hello.drac in HTML, got:\n{html}"
    );
    assert!(
        contains_labeled_link(html, "from-javascript", "from JavaScript"),
        "expected link from JavaScript -> from-javascript, got:\n{html}"
    );
}

#[test]
fn website_pipeline_renders_markdown_subset() {
    let (_, install) = published_page("install");
    assert_nav(&install);
    assert!(
        install.contains("shipped"),
        "expected install status shipped in HTML, got:\n{install}"
    );
    assert_markdown_subset(&install);

    let (_, from_systems) = published_page("from-systems");
    assert_nav(&from_systems);
    assert!(
        from_systems.contains("shipped"),
        "expected from-systems status shipped in HTML, got:\n{from_systems}"
    );
}

#[test]
fn website_pipeline_shipped_drac_fence_builds() {
    let work = temp_dir();
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    fs::write(
        website.join("learn.md"),
        page(
            LEARN_TITLE,
            "learn",
            "shipped",
            "```drac\nlet sample = 1 + 2;\n```\n",
        ),
    )
    .unwrap();
    fs::write(
        website.join("reference.md"),
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "Reference fixture.",
        ),
    )
    .unwrap();

    let fence_dir = check_fences(&website).expect("pipeline");

    let built = fence_dir.join("fence-0.js");
    assert!(
        built.is_file(),
        "expected draconic build output at {}",
        built.display()
    );
}

#[test]
fn website_pipeline_shipped_invalid_drac_fence_fails() {
    let work = temp_dir();
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    fs::write(
        website.join("learn.md"),
        page(
            LEARN_TITLE,
            "learn",
            "shipped",
            "```drac\nthis is not valid draconic !!!\n```\n",
        ),
    )
    .unwrap();
    fs::write(
        website.join("reference.md"),
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "Reference fixture.",
        ),
    )
    .unwrap();

    let err = check_fences(&website).expect_err("invalid shipped fence must fail build");
    assert!(
        err.contains("draconic build"),
        "expected draconic build failure, got: {err}"
    );
}

#[test]
fn website_pipeline_not_yet_page_with_fence_fails() {
    let work = temp_dir();
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    fs::write(
        website.join("learn.md"),
        page(LEARN_TITLE, "learn", "shipped", "Learn fixture."),
    )
    .unwrap();
    fs::write(
        website.join("reference.md"),
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "```\nsneaky sample\n```\n",
        ),
    )
    .unwrap();

    let err = check_fences(&website).expect_err("not-yet fence must fail");
    assert!(
        err.contains("not-yet") && err.contains("fence"),
        "expected not-yet fence failure, got: {err}"
    );
}

#[test]
fn website_pipeline_not_yet_page_without_fence_generates() {
    let work = temp_dir();
    let website = work.join("website");
    fs::create_dir_all(&website).unwrap();
    fs::write(
        website.join("learn.md"),
        page(LEARN_TITLE, "learn", "shipped", "Learn fixture."),
    )
    .unwrap();
    fs::write(
        website.join("reference.md"),
        page(
            REFERENCE_TITLE,
            "reference",
            "not-yet",
            "Reference fixture.",
        ),
    )
    .unwrap();

    check_fences(&website).expect("pipeline");

    let (_, modules) = published_page("modules");
    assert_nav(&modules);
    assert!(
        modules.contains("modules"),
        "expected modules title in HTML, got:\n{modules}"
    );
    assert!(
        modules.contains("shipped"),
        "expected modules status shipped in HTML, got:\n{modules}"
    );
}

fn assert_learn_chapter_nav(html: &str) {
    for (slug, label) in LEARN_CHAPTERS {
        assert!(
            contains_labeled_link(html, slug, label),
            "expected Learn nav link {label} -> {slug}, got:\n{html}"
        );
    }
}

#[test]
fn website_pipeline_learn_skeleton_is_walkable() {
    check_fences(&repo_root().join("website/content")).expect("repo fences");

    let (_, learn) = published_page("learn");
    assert_nav(&learn);
    assert_learn_chapter_nav(&learn);
    assert_visible_status(&learn, "learn");

    for (slug, _) in LEARN_CHAPTERS {
        let (path, html) = published_page(slug);
        assert_nav(&html);
        assert_learn_chapter_nav(&html);
        assert_visible_status(&html, &path.display().to_string());
    }

    let (_, from_js) = published_page("from-javascript");
    assert!(
        contains_href(&from_js, "dual-worlds"),
        "JS landing must join at Dual worlds, got:\n{from_js}"
    );
    let (_, from_sys) = published_page("from-systems");
    assert!(
        contains_href(&from_sys, "dual-worlds"),
        "systems landing must join at Dual worlds, got:\n{from_sys}"
    );
}

fn assert_reference_page_nav(html: &str) {
    for (slug, label) in REFERENCE_PAGES {
        assert!(
            contains_labeled_link(html, slug, label),
            "expected Reference nav link {label} -> {slug}, got:\n{html}"
        );
    }
}

#[test]
fn website_pipeline_reference_skeleton_is_walkable() {
    check_fences(&repo_root().join("website/content")).expect("repo fences");

    let (_, reference) = published_page("reference");
    assert_nav(&reference);
    assert_reference_page_nav(&reference);
    assert_visible_status(&reference, "reference");

    for (slug, _) in REFERENCE_PAGES {
        let (path, html) = published_page(slug);
        assert_nav(&html);
        assert_reference_page_nav(&html);
        assert_visible_status(&html, &path.display().to_string());
    }
}

const PUBLIC_SITE: &str = "https://hembrow-innovations.github.io/draconic";

#[test]
fn readme_links_public_docs_site_and_stays_onboarding() {
    let readme = repo_root().join("README.md");
    let text = fs::read_to_string(&readme).expect("read README");
    assert!(
        text.contains(PUBLIC_SITE),
        "README should link the public Learn and Reference site ({PUBLIC_SITE}):\n{text}"
    );
    assert!(
        text.contains("parse") && text.contains("hello.drac"),
        "README should still document write-parse:\n{text}"
    );
    assert!(
        text.contains("build --target js") && text.contains("build --target native"),
        "README should still document write-parse-build:\n{text}"
    );
}

#[test]
fn generated_html_is_not_authoring_source() {
    let root = repo_root();
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("read .gitignore");
    assert!(
        gitignore.contains("/website/*.html") || gitignore.contains("website/*.html"),
        "generated website HTML must be gitignored:\n{gitignore}"
    );
    assert!(
        gitignore.contains("/dist"),
        "dist must be gitignored so it is not the authoring source:\n{gitignore}"
    );
    let tracked = Command::new("git")
        .args(["ls-files", "website"])
        .current_dir(&root)
        .output()
        .expect("git ls-files website");
    assert!(
        tracked.status.success(),
        "git ls-files website failed: {}",
        String::from_utf8_lossy(&tracked.stderr)
    );
    let tracked = String::from_utf8_lossy(&tracked.stdout);
    for line in tracked.lines() {
        assert!(
            !line.ends_with(".html"),
            "website/ must not track generated HTML ({line}); markdown is the source of truth"
        );
    }
    assert!(
        !root.join("website/generate.drac").exists(),
        "generate.drac must not remain as the publisher"
    );
}

#[test]
fn ci_workflow_generates_site_and_deploys_pages() {
    let workflow = repo_root().join(".github/workflows/docs-pages.yml.disabled");
    assert!(
        workflow.is_file(),
        "missing {} (issues-26 GitHub Pages workflow)",
        workflow.display()
    );
    let text = fs::read_to_string(&workflow).expect("read workflow");
    assert!(
        text.contains("generate-website.sh") || text.contains("scripts/generate-website"),
        "workflow should run the website generate script:\n{text}"
    );
    assert!(
        text.contains("upload-pages-artifact"),
        "workflow should upload generated HTML as a Pages artifact:\n{text}"
    );
    assert!(
        text.contains("deploy-pages"),
        "workflow should deploy HTML to GitHub Pages:\n{text}"
    );
    assert!(
        text.contains("dist/pages") || text.contains("dist/pages/"),
        "workflow should publish staged HTML from dist, not committed HTML:\n{text}"
    );
}

#[test]
fn generate_website_script_stages_html_to_dist() {
    let root = repo_root();
    let script = root.join("scripts/generate-website.sh");
    assert!(
        script.is_file(),
        "missing {} (Start static publish + stage HTML)",
        script.display()
    );
    let script_text = fs::read_to_string(&script).expect("read generate-website.sh");
    assert!(
        script_text.contains("pnpm"),
        "generate-website.sh should wrap the Start pnpm build:\n{script_text}"
    );
    assert!(
        !script_text.contains("generate.drac"),
        "generate.drac must not remain the publisher:\n{script_text}"
    );
    assert!(
        !root.join("website/generate.drac").exists(),
        "website/generate.drac must be retired as renderer"
    );

    let out = published_pages();
    let index = fs::read_to_string(out.join("index.html")).expect("index.html");
    assert_html_document(&index);
    assert!(
        index.contains("JavaScript you already know"),
        "staged index should be the language homepage, got:\n{index}"
    );
    assert!(
        !index.contains("Learn is the public path"),
        "staged index must not be Learn copied to index, got:\n{index}"
    );
    assert_nav(&index);
    let (_, learn) = published_page("learn");
    assert_visible_status(&learn, "learn");
    assert!(
        out.join(".nojekyll").is_file(),
        "staged Pages dist should include .nojekyll"
    );
}
