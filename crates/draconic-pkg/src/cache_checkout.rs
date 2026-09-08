use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cache_fetch::{run_git, CacheFetchError};
use super::{validate_commit_oid, CachePathError, ModuleCache};

impl ModuleCache {
    /// True when `mod/{path…}/{oid}/` already holds a completed checkout (K03.03).
    pub fn has_entry(&self, module_path: &str, commit_oid: &str) -> Result<bool, CachePathError> {
        let dir = self.entry_dir(module_path, commit_oid)?;
        Ok(is_complete_checkout(&dir, commit_oid))
    }

    /// Materialize the package tree at `commit_oid` under `mod/{path…}/{oid}/`.
    ///
    /// Ensures the bare VCS store via [`Self::clone_or_fetch`], then extracts the
    /// tree with `git archive` (no `.git` in the package dir). When
    /// [`Self::has_entry`] is already true, returns the existing directory and
    /// performs **no** network/`git fetch` (cache hit).
    pub fn checkout(
        &self,
        module_path: &str,
        commit_oid: &str,
        git_url: &str,
    ) -> Result<PathBuf, CacheFetchError> {
        let subdir = crate::derive_package_subdir(module_path, git_url);
        self.checkout_with_subdir(module_path, commit_oid, git_url, &subdir)
    }

    /// Checkout a pinned OID, extracting `subdir` (empty = whole tree) as the
    /// package root (K11.03).
    pub fn checkout_with_subdir(
        &self,
        module_path: &str,
        commit_oid: &str,
        git_url: &str,
        subdir: &str,
    ) -> Result<PathBuf, CacheFetchError> {
        if let Err(reason) = crate::validate_package_subdir(subdir) {
            return Err(CacheFetchError::Git(format!(
                "invalid package subdir `{subdir}`: {reason}"
            )));
        }
        if let Err(reason) = validate_commit_oid(commit_oid) {
            return Err(CachePathError::InvalidCommitOid {
                oid: commit_oid.to_string(),
                reason,
            }
            .into());
        }
        let dest = self.entry_dir(module_path, commit_oid)?;
        if is_complete_checkout(&dest, commit_oid) {
            return Ok(dest);
        }

        let vcs = self.clone_or_fetch(module_path, git_url)?;
        let vcs_str = vcs
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("VCS path is not valid UTF-8".into()))?;

        // Confirm OID exists in the bare store before writing the entry dir.
        run_git(&[
            "-C",
            vcs_str,
            "cat-file",
            "-e",
            &format!("{commit_oid}^{{commit}}"),
        ])?;

        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| {
                CacheFetchError::Io(format!(
                    "remove incomplete checkout `{}`: {e}",
                    dest.display()
                ))
            })?;
        }
        fs::create_dir_all(&dest).map_err(|e| {
            CacheFetchError::Io(format!("create checkout dir `{}`: {e}", dest.display()))
        })?;

        let dest_str = dest
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("checkout path is not valid UTF-8".into()))?;

        let tree_ish = if subdir.is_empty() {
            commit_oid.to_string()
        } else {
            let kind = run_git(&[
                "-C",
                vcs_str,
                "cat-file",
                "-t",
                &format!("{commit_oid}:{subdir}"),
            ])?;
            if kind != "tree" {
                return Err(CacheFetchError::Git(format!(
                    "package subdir `{subdir}` is not a tree at {commit_oid} (got `{kind}`)"
                )));
            }
            format!("{commit_oid}:{subdir}")
        };

        // Extract tree only (no .git) so K03.04 content hash is over package files.
        // `oid:subdir` archives that tree as the archive root (package root = subdir).
        let archive = Command::new("git")
            .args(["-C", vcs_str, "archive", "--format=tar", &tree_ish])
            .output()
            .map_err(|e| CacheFetchError::Git(format!("failed to spawn git archive: {e}")))?;
        if !archive.status.success() {
            let stderr = String::from_utf8_lossy(&archive.stderr).trim().to_string();
            let _ = fs::remove_dir_all(&dest);
            return Err(CacheFetchError::Git(if stderr.is_empty() {
                format!(
                    "git archive {commit_oid} failed with status {}",
                    archive.status
                )
            } else {
                stderr
            }));
        }

        let mut tar = Command::new("tar")
            .args(["-x", "-C", dest_str])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| CacheFetchError::Io(format!("failed to spawn tar: {e}")))?;
        {
            use std::io::Write;
            let mut stdin = tar
                .stdin
                .take()
                .ok_or_else(|| CacheFetchError::Io("tar stdin missing".into()))?;
            stdin
                .write_all(&archive.stdout)
                .map_err(|e| CacheFetchError::Io(format!("write tar stdin: {e}")))?;
        }
        let tar_out = tar
            .wait_with_output()
            .map_err(|e| CacheFetchError::Io(format!("tar wait: {e}")))?;
        if !tar_out.status.success() {
            let stderr = String::from_utf8_lossy(&tar_out.stderr).trim().to_string();
            let _ = fs::remove_dir_all(&dest);
            return Err(CacheFetchError::Io(if stderr.is_empty() {
                format!("tar extract failed with status {}", tar_out.status)
            } else {
                stderr
            }));
        }

        write_checkout_marker(&dest, commit_oid).map_err(|e| {
            let _ = fs::remove_dir_all(&dest);
            CacheFetchError::Io(e)
        })?;

        Ok(dest)
    }
}

/// Marker file written after a successful K03.03 checkout (OID pin).
const CHECKOUT_MARKER: &str = ".draconic-checkout-oid";

fn checkout_marker_path(entry_dir: &Path) -> PathBuf {
    entry_dir.join(CHECKOUT_MARKER)
}

fn write_checkout_marker(entry_dir: &Path, commit_oid: &str) -> Result<(), String> {
    fs::write(checkout_marker_path(entry_dir), format!("{commit_oid}\n"))
        .map_err(|e| format!("write checkout marker: {e}"))
}

fn is_complete_checkout(entry_dir: &Path, commit_oid: &str) -> bool {
    let marker = checkout_marker_path(entry_dir);
    let Ok(contents) = fs::read_to_string(&marker) else {
        return false;
    };
    contents.trim() == commit_oid
}

#[cfg(test)]
mod tests {
    use super::super::cache_fetch::is_bare_git_repo;
    use super::super::is_entry_under_root;
    use super::*;

    const PATH: &str = "github.com/org/lib";
    const OID: &str = "0123456789abcdef0123456789abcdef01234567";

    fn temp_dir_k0303(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0303-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
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

    fn fixture_repo(root: &Path) -> PathBuf {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("hello.txt"), "hello from fixture\n").unwrap();
        git_ok(&["add", "hello.txt"], &repo);
        git_ok(&["commit", "-m", "initial"], &repo);
        repo
    }

    fn head_oid(repo: &Path) -> String {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .expect("rev-parse");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn checkout_materializes_tree_at_oid() {
        let root = temp_dir_k0303("checkout");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        assert!(!cache.has_entry(PATH, &oid).unwrap());
        let entry = cache.checkout(PATH, &oid, url).expect("checkout");

        assert_eq!(entry, cache.entry_dir(PATH, &oid).unwrap());
        assert!(cache.has_entry(PATH, &oid).unwrap());
        let hello = fs::read_to_string(entry.join("hello.txt")).expect("hello.txt");
        assert_eq!(hello, "hello from fixture\n");
        // Package tree has no .git (archive extract).
        assert!(!entry.join(".git").exists());
        // Marker pins the OID.
        assert_eq!(
            fs::read_to_string(entry.join(CHECKOUT_MARKER))
                .unwrap()
                .trim(),
            oid
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_cache_hit_skips_network() {
        let root = temp_dir_k0303("hit");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let entry1 = cache.checkout(PATH, &oid, url).expect("first checkout");
        // Remove upstream so a second checkout cannot clone/fetch.
        fs::remove_dir_all(&upstream).expect("remove upstream");
        let entry2 = cache
            .checkout(PATH, &oid, url)
            .expect("cache hit must not need remote");
        assert_eq!(entry1, entry2);
        assert!(cache.has_entry(PATH, &oid).unwrap());
        assert_eq!(
            fs::read_to_string(entry2.join("hello.txt")).unwrap(),
            "hello from fixture\n"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_two_oids_are_isolated() {
        let root = temp_dir_k0303("two-oid");
        let upstream = fixture_repo(&root);
        let oid1 = head_oid(&upstream);
        fs::write(upstream.join("hello.txt"), "second revision\n").unwrap();
        git_ok(&["add", "hello.txt"], &upstream);
        git_ok(&["commit", "-m", "second"], &upstream);
        let oid2 = head_oid(&upstream);
        assert_ne!(oid1, oid2);

        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();
        let e1 = cache.checkout(PATH, &oid1, url).expect("oid1");
        let e2 = cache.checkout(PATH, &oid2, url).expect("oid2");
        assert_ne!(e1, e2);
        assert_eq!(
            fs::read_to_string(e1.join("hello.txt")).unwrap(),
            "hello from fixture\n"
        );
        assert_eq!(
            fs::read_to_string(e2.join("hello.txt")).unwrap(),
            "second revision\n"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_rejects_unknown_oid() {
        let root = temp_dir_k0303("bad-oid");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();
        let missing = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let err = cache.checkout(PATH, missing, url).expect_err("unknown oid");
        assert!(matches!(err, CacheFetchError::Git(_)), "{err:?}");
        assert!(!cache.has_entry(PATH, missing).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_rejects_invalid_oid_shape() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .checkout(PATH, "not-an-oid", "https://example.com/x.git")
            .expect_err("bad oid");
        assert!(
            matches!(
                err,
                CacheFetchError::Path(CachePathError::InvalidCommitOid { .. })
            ),
            "{err:?}"
        );
    }

    #[test]
    fn has_entry_false_when_marker_missing() {
        let root = temp_dir_k0303("no-marker");
        let cache = ModuleCache::new(root.join("cache"));
        let entry = cache.entry_dir(PATH, OID).unwrap();
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("hello.txt"), "orphan\n").unwrap();
        assert!(!cache.has_entry(PATH, OID).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_content_hash_stable_and_excludes_marker() {
        let root = temp_dir_k0303("hash");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();
        let entry = cache.checkout(PATH, &oid, url).expect("checkout");

        let h1 = crate::content_hash_tree(&entry).expect("hash1");
        let h2 = crate::content_hash_tree(&entry).expect("hash2");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);

        // Same files without marker → same hash.
        let bare = root.join("bare-tree");
        fs::create_dir_all(&bare).unwrap();
        fs::write(bare.join("hello.txt"), "hello from fixture\n").unwrap();
        let h_bare = crate::content_hash_tree(&bare).expect("bare");
        assert_eq!(h1, h_bare);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k03_combined_layout_clone_checkout_hash() {
        let root = temp_dir_k0303("k03-combined");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        assert_eq!(oid.len(), 40);
        assert!(oid.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));

        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        // Layout keyed by module path + commit OID (K03.01).
        let entry_rel = cache.entry_rel(PATH, &oid).expect("entry rel");
        assert_eq!(
            entry_rel,
            PathBuf::from("mod")
                .join("github.com")
                .join("org")
                .join("lib")
                .join(&oid)
        );
        let vcs_rel = cache.vcs_rel(PATH).expect("vcs rel");
        assert_eq!(
            vcs_rel,
            PathBuf::from("vcs")
                .join("github.com")
                .join("org")
                .join("lib")
        );
        let entry_dir = cache.entry_dir(PATH, &oid).expect("entry dir");
        assert!(is_entry_under_root(&cache.root, &entry_dir));

        // Clone/fetch into the VCS store (K03.02).
        assert!(!cache.has_vcs(PATH).unwrap());
        let vcs = cache.clone_or_fetch(PATH, url).expect("clone");
        assert_eq!(vcs, cache.vcs_dir(PATH).unwrap());
        assert!(is_bare_git_repo(&vcs));
        assert!(cache.has_vcs(PATH).unwrap());

        // Checkout pinned OID into mod store (K03.03).
        assert!(!cache.has_entry(PATH, &oid).unwrap());
        let entry = cache.checkout(PATH, &oid, url).expect("checkout");
        assert_eq!(entry, entry_dir);
        assert!(cache.has_entry(PATH, &oid).unwrap());
        assert_eq!(
            fs::read_to_string(entry.join("hello.txt")).expect("hello.txt"),
            "hello from fixture\n"
        );
        assert!(!entry.join(".git").exists());
        assert_eq!(
            crate::read_checkout_oid(&entry).expect("marker").as_deref(),
            Some(oid.as_str())
        );

        // SHA-256 over the canonical package tree (K03.04); marker is not content.
        let hash = crate::content_hash_tree(&entry).expect("tree hash");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        let bare = root.join("canonical-tree");
        fs::create_dir_all(&bare).unwrap();
        fs::write(bare.join("hello.txt"), "hello from fixture\n").unwrap();
        assert_eq!(crate::content_hash_tree(&bare).expect("bare hash"), hash);

        // Cache hit skips network: upstream gone, second checkout still serves the tree.
        fs::remove_dir_all(&upstream).expect("remove upstream");
        let hit = cache
            .checkout(PATH, &oid, url)
            .expect("cache hit must not need remote");
        assert_eq!(hit, entry);
        assert_eq!(
            fs::read_to_string(hit.join("hello.txt")).unwrap(),
            "hello from fixture\n"
        );
        assert_eq!(crate::content_hash_tree(&hit).expect("hit hash"), hash);

        let _ = fs::remove_dir_all(&root);
    }
}
