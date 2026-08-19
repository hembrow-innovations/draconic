//! Canonical package tree content hash (Roadmap K03.04 / K08.01).
//!
//! SHA-256 over a deterministic encoding of regular files under a package root.
//! Used for lockfile `content_hash` (K02.01) and integrity checks (K08.01):
//! recompute tree hash and hard-fail when it does not match the lock pin.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Cache marker written by K03.03 checkout — not package content.
const CHECKOUT_MARKER: &str = ".draconic-checkout-oid";

/// Error while hashing a package tree (K03.04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentHashError {
    /// Package root is missing or not a directory.
    NotADirectory { path: String },
    /// Filesystem I/O failed while walking or reading.
    Io { path: String, message: String },
    /// Encountered a symlink (not followed; fail closed for integrity).
    Symlink { path: String },
    /// Relative path is not valid UTF-8.
    NonUtf8Path { path: String },
}

impl fmt::Display for ContentHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentHashError::NotADirectory { path } => {
                write!(f, "content hash: not a directory `{path}`")
            }
            ContentHashError::Io { path, message } => {
                write!(f, "content hash: I/O error at `{path}`: {message}")
            }
            ContentHashError::Symlink { path } => {
                write!(f, "content hash: symlink not allowed `{path}`")
            }
            ContentHashError::NonUtf8Path { path } => {
                write!(f, "content hash: non-UTF-8 path `{path}`")
            }
        }
    }
}

impl std::error::Error for ContentHashError {}

/// SHA-256 hex (64 lowercase) of the canonical package tree at `package_root`.
///
/// Walks regular files under `package_root` (recursive). Relative paths use `/`
/// separators, no leading `./`, sorted by UTF-8 byte order. Excludes the
/// checkout marker [`.draconic-checkout-oid`](CHECKOUT_MARKER).
///
/// Canonical stream for each file in order:
/// - `u64` big-endian path byte length
/// - path UTF-8 bytes
/// - `u64` big-endian content byte length
/// - content bytes
///
/// Empty tree (no files) hashes the empty stream (SHA-256 of zero bytes).
pub fn content_hash_tree(package_root: &Path) -> Result<String, ContentHashError> {
    let meta = fs::metadata(package_root).map_err(|e| ContentHashError::Io {
        path: package_root.display().to_string(),
        message: e.to_string(),
    })?;
    if !meta.is_dir() {
        return Err(ContentHashError::NotADirectory {
            path: package_root.display().to_string(),
        });
    }

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_files(package_root, package_root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (rel, abs) in &files {
        let mut content = Vec::new();
        let mut f = fs::File::open(abs).map_err(|e| ContentHashError::Io {
            path: abs.display().to_string(),
            message: e.to_string(),
        })?;
        f.read_to_end(&mut content)
            .map_err(|e| ContentHashError::Io {
                path: abs.display().to_string(),
                message: e.to_string(),
            })?;

        let path_bytes = rel.as_bytes();
        hasher.update((path_bytes.len() as u64).to_be_bytes());
        hasher.update(path_bytes);
        hasher.update((content.len() as u64).to_be_bytes());
        hasher.update(&content);
    }

    let digest = hasher.finalize();
    Ok(hex_lower(&digest))
}

/// Error while verifying a package tree against a lock `content_hash` (K08.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentHashVerifyError {
    /// Failed to recompute the tree hash.
    Hash(ContentHashError),
    /// Recomputed SHA-256 does not match the lock pin (tamper or wrong tree).
    Mismatch {
        /// Package root that was hashed.
        path: String,
        /// Lock pin `content_hash` (expected).
        expected: String,
        /// Freshly recomputed tree hash (actual).
        actual: String,
    },
}

impl fmt::Display for ContentHashVerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentHashVerifyError::Hash(e) => write!(f, "content hash verify: {e}"),
            ContentHashVerifyError::Mismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "content hash verify: package `{path}` tree hash mismatch (lock={expected}, actual={actual})"
            ),
        }
    }
}

impl std::error::Error for ContentHashVerifyError {}

impl From<ContentHashError> for ContentHashVerifyError {
    fn from(e: ContentHashError) -> Self {
        ContentHashVerifyError::Hash(e)
    }
}

/// Recompute the package tree SHA-256 and hard-fail unless it equals `expected_hash` (K08.01).
///
/// `expected_hash` is the lockfile pin (`content_hash`). Empty or format-invalid
/// expected hashes still fail closed via mismatch (or hash error on the tree side).
pub fn verify_content_hash(
    package_root: &Path,
    expected_hash: &str,
) -> Result<(), ContentHashVerifyError> {
    let actual = content_hash_tree(package_root)?;
    if actual == expected_hash {
        return Ok(());
    }
    Err(ContentHashVerifyError::Mismatch {
        path: package_root.display().to_string(),
        expected: expected_hash.to_string(),
        actual,
    })
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn collect_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), ContentHashError> {
    let entries = fs::read_dir(dir).map_err(|e| ContentHashError::Io {
        path: dir.display().to_string(),
        message: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ContentHashError::Io {
            path: dir.display().to_string(),
            message: e.to_string(),
        })?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| ContentHashError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        if ft.is_symlink() {
            return Err(ContentHashError::Symlink {
                path: path.display().to_string(),
            });
        }

        if ft.is_dir() {
            // Do not descend into nested .git if present.
            if entry.file_name() == ".git" {
                continue;
            }
            collect_files(root, &path, out)?;
            continue;
        }

        if !ft.is_file() {
            continue;
        }

        let rel = path.strip_prefix(root).map_err(|_| ContentHashError::Io {
            path: path.display().to_string(),
            message: "path not under package root".into(),
        })?;
        let rel_str = rel_to_unix(rel)?;
        if rel_str == CHECKOUT_MARKER {
            continue;
        }
        out.push((rel_str, path));
    }
    Ok(())
}

fn rel_to_unix(rel: &Path) -> Result<String, ContentHashError> {
    let mut parts: Vec<&str> = Vec::new();
    for c in rel.components() {
        match c {
            std::path::Component::Normal(s) => {
                let s = s.to_str().ok_or_else(|| ContentHashError::NonUtf8Path {
                    path: rel.display().to_string(),
                })?;
                parts.push(s);
            }
            std::path::Component::CurDir => {}
            _ => {
                return Err(ContentHashError::Io {
                    path: rel.display().to_string(),
                    message: "unexpected path component".into(),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0304-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// SHA-256 of empty input (empty package tree).
    const EMPTY_SHA256: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn empty_tree_is_sha256_of_empty() {
        let root = temp_dir("empty");
        let hash = content_hash_tree(&root).expect("hash");
        assert_eq!(hash, EMPTY_SHA256);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn single_file_stable_hash() {
        let root = temp_dir("one");
        fs::write(root.join("hello.txt"), b"hello\n").unwrap();
        let a = content_hash_tree(&root).unwrap();
        let b = content_hash_tree(&root).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, EMPTY_SHA256);
        assert_eq!(a.len(), 64);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_order_independent_of_creation() {
        let root_a = temp_dir("order-a");
        fs::write(root_a.join("b.txt"), b"B").unwrap();
        fs::write(root_a.join("a.txt"), b"A").unwrap();
        let ha = content_hash_tree(&root_a).unwrap();

        let root_b = temp_dir("order-b");
        fs::write(root_b.join("a.txt"), b"A").unwrap();
        fs::write(root_b.join("b.txt"), b"B").unwrap();
        let hb = content_hash_tree(&root_b).unwrap();

        assert_eq!(ha, hb);
        let _ = fs::remove_dir_all(&root_a);
        let _ = fs::remove_dir_all(&root_b);
    }

    #[test]
    fn nested_paths_use_slash_and_sort() {
        let root = temp_dir("nested");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.drac"), b"export let x = 1;\n").unwrap();
        fs::write(root.join("draconic.toml"), b"module = \"github.com/org/lib\"\n").unwrap();
        let h1 = content_hash_tree(&root).unwrap();
        let h2 = content_hash_tree(&root).unwrap();
        assert_eq!(h1, h2);
        // Content change changes hash.
        fs::write(root.join("src/lib.drac"), b"export let x = 2;\n").unwrap();
        let h3 = content_hash_tree(&root).unwrap();
        assert_ne!(h1, h3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_marker_excluded() {
        let root = temp_dir("marker");
        fs::write(root.join("hello.txt"), b"hello\n").unwrap();
        let without = content_hash_tree(&root).unwrap();
        fs::write(
            root.join(CHECKOUT_MARKER),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();
        let with = content_hash_tree(&root).unwrap();
        assert_eq!(without, with);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn content_change_changes_hash() {
        let root = temp_dir("mutate");
        fs::write(root.join("f.txt"), b"one").unwrap();
        let h1 = content_hash_tree(&root).unwrap();
        fs::write(root.join("f.txt"), b"two").unwrap();
        let h2 = content_hash_tree(&root).unwrap();
        assert_ne!(h1, h2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn path_name_is_part_of_hash() {
        let root_a = temp_dir("name-a");
        fs::write(root_a.join("a.txt"), b"same").unwrap();
        let ha = content_hash_tree(&root_a).unwrap();

        let root_b = temp_dir("name-b");
        fs::write(root_b.join("b.txt"), b"same").unwrap();
        let hb = content_hash_tree(&root_b).unwrap();

        assert_ne!(ha, hb);
        let _ = fs::remove_dir_all(&root_a);
        let _ = fs::remove_dir_all(&root_b);
    }

    #[test]
    fn not_a_directory_errors() {
        let root = temp_dir("file-root");
        let file = root.join("x");
        fs::write(&file, b"x").unwrap();
        let err = content_hash_tree(&file).expect_err("file");
        assert!(matches!(err, ContentHashError::NotADirectory { .. }));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_root_is_io_error() {
        let missing = std::env::temp_dir().join(format!(
            "draconic-pkg-k0304-missing-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let err = content_hash_tree(&missing).expect_err("missing");
        assert!(matches!(err, ContentHashError::Io { .. }));
    }

    #[test]
    fn known_vector_single_empty_file() {
        // path "" is not used; empty file named "e" with empty content.
        let root = temp_dir("vector");
        fs::write(root.join("e"), b"").unwrap();
        let hash = content_hash_tree(&root).unwrap();
        // Manually: path "e" (1 byte) + content empty.
        let mut h = Sha256::new();
        h.update(1u64.to_be_bytes());
        h.update(b"e");
        h.update(0u64.to_be_bytes());
        h.update(b"");
        let expected = hex_lower(&h.finalize());
        assert_eq!(hash, expected);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_is_rejected() {
        let root = temp_dir("symlink");
        fs::write(root.join("real.txt"), b"data").unwrap();
        std::os::unix::fs::symlink("real.txt", root.join("link.txt")).unwrap();
        let err = content_hash_tree(&root).expect_err("symlink");
        assert!(matches!(err, ContentHashError::Symlink { .. }));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_tree_hash_matches_hand_built() {
        // Integration with cache layout: hash after writing package files only.
        let root = temp_dir("hand");
        fs::create_dir_all(root.join("pkg")).unwrap();
        let pkg = root.join("pkg");
        fs::write(pkg.join("a.txt"), b"A\n").unwrap();
        fs::write(pkg.join("b.txt"), b"B\n").unwrap();
        let hash = content_hash_tree(&pkg).unwrap();

        let mut h = Sha256::new();
        // a.txt then b.txt (sorted)
        h.update(5u64.to_be_bytes());
        h.update(b"a.txt");
        h.update(2u64.to_be_bytes());
        h.update(b"A\n");
        h.update(5u64.to_be_bytes());
        h.update(b"b.txt");
        h.update(2u64.to_be_bytes());
        h.update(b"B\n");
        assert_eq!(hash, hex_lower(&h.finalize()));
        let _ = fs::remove_dir_all(&root);
    }

    // --- K08.01: recompute tree SHA-256; match lock or hard-fail ---

    #[test]
    fn verify_content_hash_ok_when_matches() {
        let root = temp_dir("verify-ok");
        fs::write(root.join("index.drac"), b"export let x = 1;\n").unwrap();
        let expected = content_hash_tree(&root).unwrap();
        verify_content_hash(&root, &expected).expect("match");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_content_hash_ok_empty_tree() {
        let root = temp_dir("verify-empty");
        verify_content_hash(&root, EMPTY_SHA256).expect("empty match");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_content_hash_mismatch_tampered_file() {
        let root = temp_dir("verify-tamper");
        fs::write(root.join("index.drac"), b"export let x = 1;\n").unwrap();
        let expected = content_hash_tree(&root).unwrap();
        fs::write(root.join("index.drac"), b"export let x = 999;\n").unwrap();
        let err = verify_content_hash(&root, &expected).expect_err("tamper");
        match &err {
            ContentHashVerifyError::Mismatch {
                expected: e,
                actual: a,
                ..
            } => {
                assert_eq!(e, &expected);
                assert_ne!(a, &expected);
                assert_eq!(a.len(), 64);
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("mismatch"), "{msg}");
        assert!(msg.contains(&expected), "{msg}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_content_hash_mismatch_wrong_lock_hash() {
        let root = temp_dir("verify-wrong-lock");
        fs::write(root.join("a.txt"), b"A").unwrap();
        let bogus = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let err = verify_content_hash(&root, bogus).expect_err("wrong lock");
        match err {
            ContentHashVerifyError::Mismatch { expected, actual, .. } => {
                assert_eq!(expected, bogus);
                assert_ne!(actual, bogus);
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_content_hash_extra_file_is_mismatch() {
        let root = temp_dir("verify-extra");
        fs::write(root.join("a.txt"), b"A").unwrap();
        let expected = content_hash_tree(&root).unwrap();
        fs::write(root.join("evil.txt"), b"x").unwrap();
        let err = verify_content_hash(&root, &expected).expect_err("extra file");
        assert!(matches!(err, ContentHashVerifyError::Mismatch { .. }), "{err:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_content_hash_marker_change_does_not_mismatch() {
        // Checkout marker is not part of the tree hash (K03.04 / K08.01).
        let root = temp_dir("verify-marker");
        fs::write(root.join("a.txt"), b"A").unwrap();
        let expected = content_hash_tree(&root).unwrap();
        fs::write(root.join(".draconic-checkout-oid"), "deadbeef\n").unwrap();
        verify_content_hash(&root, &expected).expect("marker ignored");
        let _ = fs::remove_dir_all(&root);
    }
}
