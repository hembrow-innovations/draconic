use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::auth::{
    clone_url_with_auth, git_auth_rejected, git_ssh_command, is_git_auth_failure, is_ssh_git_url,
    redact_secrets, sanitize_stored_git_url, GitAuth, GitAuthError,
};
use crate::proxy::ProxyError;

use super::{CachePathError, ModuleCache};

/// Error while cloning or fetching a package into the module cache (K03.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheFetchError {
    /// Module path / layout validation failed.
    Path(CachePathError),
    /// Git URL is empty or not an allowed clone URL.
    InvalidUrl { url: String, reason: &'static str },
    /// Filesystem error (create dirs, etc.).
    Io(String),
    /// `git` subprocess failed or is unavailable.
    Git(String),
    /// Private git credentials missing or rejected (K11.01).
    Auth(GitAuthError),
    /// Module proxy is off or invalid (K11.04).
    Proxy(ProxyError),
}

impl fmt::Display for CacheFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheFetchError::Path(e) => write!(f, "{e}"),
            CacheFetchError::InvalidUrl { url, reason } => {
                write!(f, "module cache: invalid git URL `{url}`: {reason}")
            }
            CacheFetchError::Io(msg) => write!(f, "module cache: I/O error: {msg}"),
            CacheFetchError::Git(msg) => write!(f, "module cache: git error: {msg}"),
            CacheFetchError::Auth(e) => write!(f, "module cache: {e}"),
            CacheFetchError::Proxy(e) => write!(f, "module cache: {e}"),
        }
    }
}

impl std::error::Error for CacheFetchError {}

impl From<CachePathError> for CacheFetchError {
    fn from(e: CachePathError) -> Self {
        CacheFetchError::Path(e)
    }
}

impl From<GitAuthError> for CacheFetchError {
    fn from(e: GitAuthError) -> Self {
        CacheFetchError::Auth(e)
    }
}

impl From<ProxyError> for CacheFetchError {
    fn from(e: ProxyError) -> Self {
        CacheFetchError::Proxy(e)
    }
}

/// Accept clone URLs for K03.02: https/http (production), plus `file://` and
/// absolute local paths for fixture repos in tests.
fn validate_clone_url(url: &str) -> Result<(), &'static str> {
    if url.is_empty() {
        return Err("must not be empty");
    }
    if url != url.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if url.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }

    if let Some(rest) = url.strip_prefix("https://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("https URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("http://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("http URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("file://") {
        if rest.is_empty() {
            return Err("file URL must include a path");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("git@") {
        if !rest.contains(':') || !rest.contains('.') {
            return Err("ssh git URL must look like git@host:path");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        if rest.is_empty() {
            return Err("ssh URL must include a host");
        }
        return Ok(());
    }
    // Absolute local path (fixture repos): Unix `/…` or Windows drive `C:\…` / `C:/…`.
    let path = Path::new(url);
    if path.is_absolute() {
        return Ok(());
    }

    Err("must be https://, http://, file://, git@, ssh://, or an absolute local path")
}

/// True if `dir` looks like an existing bare git repository.
pub(crate) fn is_bare_git_repo(dir: &Path) -> bool {
    dir.join("HEAD").is_file() && dir.join("objects").is_dir() && dir.join("refs").is_dir()
}

pub(crate) fn run_git(args: &[&str]) -> Result<String, CacheFetchError> {
    run_git_authed(args, &GitAuth::None, None)
}

fn run_git_authed(
    args: &[&str],
    auth: &GitAuth,
    remote_url: Option<&str>,
) -> Result<String, CacheFetchError> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    // Fail closed: never hang on a password prompt (K11.01).
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GCM_INTERACTIVE", "never");
    if remote_url.is_some_and(is_ssh_git_url) {
        // Always BatchMode: fail closed, never hang on a passphrase/host-key prompt.
        let ssh = git_ssh_command(auth)
            .unwrap_or_else(|| "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes".to_string());
        cmd.env("GIT_SSH_COMMAND", ssh);
    }
    let output = cmd
        .output()
        .map_err(|e| CacheFetchError::Git(format!("failed to spawn git: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git {:?} failed with status {}", args, output.status)
        };
        let detail = redact_secrets(&detail, auth);
        if is_git_auth_failure(&detail) {
            let url = remote_url.unwrap_or("remote");
            return Err(CacheFetchError::Auth(git_auth_rejected(url, &detail, auth)));
        }
        Err(CacheFetchError::Git(detail))
    }
}

impl ModuleCache {
    /// Clone `git_url` into the module VCS store, or `git fetch` if already present.
    ///
    /// Uses [`GitAuth::from_env`] (K11.01). See [`Self::clone_or_fetch_with_auth`].
    pub fn clone_or_fetch(
        &self,
        module_path: &str,
        git_url: &str,
    ) -> Result<PathBuf, CacheFetchError> {
        self.clone_or_fetch_with_auth(module_path, git_url, &GitAuth::from_env())
    }

    /// Clone/fetch with explicit private git credentials (K11.01).
    ///
    /// HTTPS token is applied to the git subprocess only; the stored `origin` URL
    /// is sanitized so secrets never land in the cache VCS config. SSH URLs
    /// (`git@` / `ssh://`) are accepted. Missing or rejected credentials fail
    /// closed (`CacheFetchError::Auth`) without a password prompt.
    pub fn clone_or_fetch_with_auth(
        &self,
        module_path: &str,
        git_url: &str,
        auth: &GitAuth,
    ) -> Result<PathBuf, CacheFetchError> {
        self.clone_or_fetch_split(module_path, git_url, git_url, auth)
    }

    /// Clone from `fetch_url` but persist `stored_url` as `origin` (K11.04).
    pub(crate) fn clone_or_fetch_split(
        &self,
        module_path: &str,
        fetch_url: &str,
        stored_url: &str,
        auth: &GitAuth,
    ) -> Result<PathBuf, CacheFetchError> {
        if let Err(reason) = validate_clone_url(fetch_url) {
            return Err(CacheFetchError::InvalidUrl {
                url: fetch_url.to_string(),
                reason,
            });
        }
        let authed_fetch = clone_url_with_auth(fetch_url, auth)?;
        let stored_url = sanitize_stored_git_url(stored_url);
        let dest = self.vcs_dir(module_path)?;
        if is_bare_git_repo(&dest) {
            let dest_str = dest
                .to_str()
                .ok_or_else(|| CacheFetchError::Io("VCS path is not valid UTF-8".into()))?;
            // Fetch from the authed URL (not stored origin) so tokens are not persisted.
            run_git_authed(
                &[
                    "-C",
                    dest_str,
                    "fetch",
                    "--force",
                    &authed_fetch,
                    "+refs/*:refs/*",
                ],
                auth,
                Some(fetch_url),
            )?;
            let _ = run_git(&["-C", dest_str, "remote", "set-url", "origin", &stored_url]);
            return Ok(dest);
        }
        if dest.exists() {
            return Err(CacheFetchError::Io(format!(
                "VCS path `{}` exists but is not a bare git repository",
                dest.display()
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CacheFetchError::Io(format!("create VCS parent `{}`: {e}", parent.display()))
            })?;
        }
        let dest_str = dest
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("VCS path is not valid UTF-8".into()))?;
        if let Err(e) = run_git_authed(
            &["clone", "--bare", &authed_fetch, dest_str],
            auth,
            Some(fetch_url),
        ) {
            let _ = fs::remove_dir_all(&dest);
            return Err(e);
        }
        if !is_bare_git_repo(&dest) {
            let _ = fs::remove_dir_all(&dest);
            return Err(CacheFetchError::Git(format!(
                "clone succeeded but `{}` is not a bare repository",
                dest.display()
            )));
        }
        let _ = run_git(&["-C", dest_str, "remote", "set-url", "origin", &stored_url]);
        Ok(dest)
    }

    /// True when the module already has a bare VCS store in this cache.
    pub fn has_vcs(&self, module_path: &str) -> Result<bool, CachePathError> {
        Ok(is_bare_git_repo(&self.vcs_dir(module_path)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "github.com/org/lib";

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0302-{tag}-{}-{}",
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

    /// Create a non-bare fixture repo with one commit; return its path.
    fn fixture_repo(root: &Path) -> PathBuf {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        // Default branch name stable across git versions.
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
    fn clone_or_fetch_clones_fixture_into_vcs_store() {
        let root = temp_dir("clone");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));

        assert!(!cache.has_vcs(PATH).unwrap());
        let vcs = cache
            .clone_or_fetch(PATH, upstream.to_str().unwrap())
            .expect("clone");

        assert!(vcs.starts_with(root.join("cache")));
        assert!(is_bare_git_repo(&vcs));
        assert!(cache.has_vcs(PATH).unwrap());
        assert_eq!(vcs, cache.vcs_dir(PATH).unwrap());

        // Bare clone retains the commit object.
        let out = Command::new("git")
            .args(["cat-file", "-t", &oid])
            .current_dir(&vcs)
            .output()
            .expect("cat-file");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "commit");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_file_url_fixture() {
        let root = temp_dir("file-url");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = format!("file://{}", upstream.display());
        let vcs = cache.clone_or_fetch(PATH, &url).expect("clone file url");
        assert!(is_bare_git_repo(&vcs));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_second_call_fetches() {
        let root = temp_dir("fetch");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let vcs1 = cache.clone_or_fetch(PATH, url).expect("first clone");
        // Add a second commit on upstream.
        fs::write(upstream.join("hello.txt"), "second\n").unwrap();
        git_ok(&["add", "hello.txt"], &upstream);
        git_ok(&["commit", "-m", "second"], &upstream);
        let oid2 = head_oid(&upstream);

        let vcs2 = cache.clone_or_fetch(PATH, url).expect("fetch");
        assert_eq!(vcs1, vcs2);
        let out = Command::new("git")
            .args(["cat-file", "-t", &oid2])
            .current_dir(&vcs2)
            .output()
            .expect("cat-file oid2");
        assert!(
            out.status.success(),
            "fetch should bring new commit: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_rejects_invalid_module_path() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .clone_or_fetch("not-a-path", "https://example.com/x.git")
            .expect_err("bad path");
        assert!(matches!(err, CacheFetchError::Path(_)), "{err:?}");
    }

    #[test]
    fn clone_or_fetch_rejects_empty_url() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache.clone_or_fetch(PATH, "").expect_err("empty url");
        match err {
            CacheFetchError::InvalidUrl { url, reason } => {
                assert_eq!(url, "");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidUrl, got {other:?}"),
        }
    }

    #[test]
    fn clone_or_fetch_rejects_ftp_url() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .clone_or_fetch(PATH, "ftp://example.com/x.git")
            .expect_err("ftp");
        assert!(matches!(err, CacheFetchError::InvalidUrl { .. }), "{err:?}");
    }

    #[test]
    fn validate_clone_url_accepts_https() {
        assert!(validate_clone_url("https://github.com/org/pkg.git").is_ok());
        assert!(validate_clone_url("http://git.example.com/org/pkg.git").is_ok());
    }

    #[test]
    fn k11_01_validate_clone_url_accepts_ssh() {
        assert!(validate_clone_url("git@github.com:org/pkg.git").is_ok());
        assert!(validate_clone_url("ssh://git@github.com/org/pkg.git").is_ok());
    }

    #[test]
    fn k11_01_clone_or_fetch_ssh_url_is_not_invalid_url() {
        let root = temp_dir("ssh-url");
        let cache = ModuleCache::new(root.join("cache"));
        let err = cache
            .clone_or_fetch(PATH, "ssh://git@127.0.0.1:1/org/lib.git")
            .expect_err("ssh to closed port");
        assert!(
            !matches!(err, CacheFetchError::InvalidUrl { .. }),
            "ssh URLs must be accepted for clone: {err:?}"
        );
        assert!(
            matches!(err, CacheFetchError::Git(_) | CacheFetchError::Auth(_)),
            "{err:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_01_clone_missing_ssh_identity_fails_closed() {
        let cache = ModuleCache::new("/tmp/k11-01-ssh-missing");
        let auth = GitAuth::Ssh {
            identity_file: Some(PathBuf::from("/no/such/k11-01-ssh-key")),
        };
        let err = cache
            .clone_or_fetch_with_auth(PATH, "git@github.com:org/lib.git", &auth)
            .expect_err("missing identity");
        match err {
            CacheFetchError::Auth(crate::GitAuthError::MissingSshIdentity { ref path }) => {
                assert!(path.contains("k11-01-ssh-key"), "{path}");
            }
            other => panic!("expected Auth(MissingSshIdentity), got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("missing"), "{msg}");
        assert!(!msg.contains("InvalidUrl"), "{msg}");
    }

    #[test]
    fn k11_01_clone_does_not_persist_https_token_in_origin() {
        let root = temp_dir("auth-origin");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let auth = GitAuth::https_token("git", "s3cret-token").unwrap();
        let url = format!("file://{}", upstream.display());
        let vcs = cache
            .clone_or_fetch_with_auth(PATH, &url, &auth)
            .expect("clone file url with token auth");
        let origin = Command::new("git")
            .args([
                "-C",
                vcs.to_str().unwrap(),
                "config",
                "--get",
                "remote.origin.url",
            ])
            .output()
            .expect("origin url");
        let origin = String::from_utf8_lossy(&origin.stdout);
        assert!(
            !origin.contains("s3cret-token"),
            "origin leaked token: {origin}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_missing_remote_is_git_error() {
        let root = temp_dir("missing");
        let cache = ModuleCache::new(root.join("cache"));
        let missing = root.join("no-such-repo");
        let err = cache
            .clone_or_fetch(PATH, missing.to_str().unwrap())
            .expect_err("missing remote");
        assert!(matches!(err, CacheFetchError::Git(_)), "{err:?}");
        assert!(!cache.has_vcs(PATH).unwrap());
        let _ = fs::remove_dir_all(&root);
    }
}
