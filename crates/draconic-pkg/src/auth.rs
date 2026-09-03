//! Private git auth for package fetch (Roadmap K11.01).
//!
//! HTTPS token or SSH credentials may authenticate clone/fetch of a private
//! module. Credentials are resolved from the environment or an explicit
//! [`GitAuth`] value — never from `draconic.toml` or `draconic.lock`, and never
//! written back into those files.

use std::fmt;
use std::path::{Path, PathBuf};

/// How to authenticate a private git fetch (K11.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitAuth {
    /// No extra credentials (public HTTPS, `file://` fixtures, or ambient git).
    None,
    /// HTTPS username + token (token used as the password). Typical username: `git`.
    HttpsToken { username: String, token: String },
    /// SSH. Optional identity file (`ssh -i`); otherwise ssh-agent / default keys.
    Ssh { identity_file: Option<PathBuf> },
}

/// Fail-closed private git auth error (missing or rejected credentials).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitAuthError {
    /// HTTPS token required but empty / unset.
    MissingHttpsToken,
    /// SSH identity file was named but is not a readable file.
    MissingSshIdentity { path: String },
    /// Git/ssh rejected the credentials (or could not authenticate).
    Rejected { url: String, reason: String },
}

impl fmt::Display for GitAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitAuthError::MissingHttpsToken => write!(
                f,
                "private git auth: missing HTTPS token (set DRACONIC_GIT_TOKEN)"
            ),
            GitAuthError::MissingSshIdentity { path } => {
                write!(f, "private git auth: missing SSH identity file `{path}`")
            }
            GitAuthError::Rejected { url, reason } => {
                write!(f, "private git auth rejected for `{url}`: {reason}")
            }
        }
    }
}

impl std::error::Error for GitAuthError {}

impl GitAuth {
    /// HTTPS token auth. Empty token fails closed (missing credentials).
    pub fn https_token(username: &str, token: &str) -> Result<Self, GitAuthError> {
        if token.is_empty() {
            return Err(GitAuthError::MissingHttpsToken);
        }
        let username = if username.is_empty() {
            "git".to_string()
        } else {
            username.to_string()
        };
        Ok(GitAuth::HttpsToken {
            username,
            token: token.to_string(),
        })
    }

    /// SSH identity-file auth. Missing file fails closed.
    pub fn ssh_identity(path: impl AsRef<Path>) -> Result<Self, GitAuthError> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(GitAuthError::MissingSshIdentity {
                path: path.display().to_string(),
            });
        }
        Ok(GitAuth::Ssh {
            identity_file: Some(path.to_path_buf()),
        })
    }

    /// Resolve credentials from process env. Never reads manifest/lock.
    ///
    /// - `DRACONIC_GIT_TOKEN` → [`GitAuth::HttpsToken`] (user: `DRACONIC_GIT_TOKEN_USER` or `git`)
    /// - else `DRACONIC_GIT_SSH_KEY` → [`GitAuth::Ssh`] with that identity file path
    /// - else [`GitAuth::None`]
    pub fn from_env() -> Self {
        git_auth_from_vars(|k| std::env::var(k).ok())
    }
}

/// Resolve [`GitAuth`] from a key/value lookup (env in production; map in tests).
pub fn git_auth_from_vars<F>(mut get: F) -> GitAuth
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(token) = get("DRACONIC_GIT_TOKEN").filter(|s| !s.is_empty()) {
        let username = get("DRACONIC_GIT_TOKEN_USER")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "git".to_string());
        return GitAuth::HttpsToken { username, token };
    }
    if let Some(key) = get("DRACONIC_GIT_SSH_KEY").filter(|s| !s.is_empty()) {
        return GitAuth::Ssh {
            identity_file: Some(PathBuf::from(key)),
        };
    }
    GitAuth::None
}

/// Strip userinfo (`user:token@`) from http(s) git URLs so secrets are not stored.
///
/// SSH, `file://`, and local paths are returned unchanged (they have no token userinfo).
pub fn sanitize_stored_git_url(url: &str) -> String {
    for scheme in ["https://", "http://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            if let Some((_, host_and_path)) = rest.split_once('@') {
                // Only treat as userinfo when the left side has no slash (not a path).
                if !rest[..rest.len() - host_and_path.len() - 1].contains('/') {
                    return format!("{scheme}{host_and_path}");
                }
            }
            return url.to_string();
        }
    }
    url.to_string()
}

/// True if `url` is an SSH git remote (`git@host:path` or `ssh://…`).
pub fn is_ssh_git_url(url: &str) -> bool {
    url.starts_with("git@") || url.starts_with("ssh://")
}

/// True if `url` is an http(s) git remote.
pub fn is_https_git_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// Remote URL passed to `git clone`/`git fetch` for this auth.
///
/// HTTPS token is embedded as userinfo for the subprocess only. The stored
/// origin / lock URL must go through [`sanitize_stored_git_url`].
pub fn clone_url_with_auth(url: &str, auth: &GitAuth) -> Result<String, GitAuthError> {
    match auth {
        GitAuth::None => Ok(url.to_string()),
        GitAuth::HttpsToken { username, token } => {
            if !is_https_git_url(url) {
                return Ok(url.to_string());
            }
            if token.is_empty() {
                return Err(GitAuthError::MissingHttpsToken);
            }
            Ok(embed_https_userinfo(url, username, token))
        }
        GitAuth::Ssh { identity_file } => {
            if is_ssh_git_url(url) {
                if let Some(path) = identity_file {
                    if !path.is_file() {
                        return Err(GitAuthError::MissingSshIdentity {
                            path: path.display().to_string(),
                        });
                    }
                }
            }
            Ok(url.to_string())
        }
    }
}

fn embed_https_userinfo(url: &str, username: &str, token: &str) -> String {
    let sanitized = sanitize_stored_git_url(url);
    if let Some(rest) = sanitized.strip_prefix("https://") {
        return format!("https://{username}:{token}@{rest}");
    }
    if let Some(rest) = sanitized.strip_prefix("http://") {
        return format!("http://{username}:{token}@{rest}");
    }
    sanitized
}

/// `GIT_SSH_COMMAND` value for SSH auth (`BatchMode` = fail closed, no prompt).
pub fn git_ssh_command(auth: &GitAuth) -> Option<String> {
    match auth {
        GitAuth::Ssh {
            identity_file: Some(path),
        } => Some(format!(
            "ssh -i {} -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes",
            shell_single_quote(&path.display().to_string())
        )),
        GitAuth::Ssh {
            identity_file: None,
        } => Some("ssh -o BatchMode=yes -o StrictHostKeyChecking=yes".to_string()),
        GitAuth::None | GitAuth::HttpsToken { .. } => None,
    }
}

fn shell_single_quote(s: &str) -> String {
    // Safe for GIT_SSH_COMMAND: wrap in single quotes, escape inner quotes.
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// True when git/ssh stderr indicates credentials were missing or rejected.
pub fn is_git_auth_failure(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "authentication failed",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "permission denied (publickey)",
        "permission denied (keyboard-interactive",
        "permission denied (password)",
        "no matching host key",
        "host key verification failed",
        "invalid userinfo",
        "401 unauthorized",
        "403 forbidden",
        "fatal: could not read from remote repository",
        "error: repository not found",
        "remote: invalid username or password",
        "remote: support for password authentication was removed",
        "remote: http basic: access denied",
    ];
    NEEDLES.iter().any(|n| s.contains(n))
}

/// Redact `token` (and https userinfo) from a diagnostic string.
pub fn redact_secrets(text: &str, auth: &GitAuth) -> String {
    let mut out = sanitize_userinfo_in_text(text);
    if let GitAuth::HttpsToken { token, .. } = auth {
        if !token.is_empty() {
            out = out.replace(token, "***");
        }
    }
    out
}

fn sanitize_userinfo_in_text(text: &str) -> String {
    // Replace https://user:pass@host with https://host in free text.
    let mut out = text.to_string();
    for scheme in ["https://", "http://"] {
        let mut search_from = 0;
        while let Some(rel) = out[search_from..].find(scheme) {
            let start = search_from + rel;
            let after = start + scheme.len();
            if let Some(at_rel) = out[after..].find('@') {
                let at = after + at_rel;
                let userinfo = &out[after..at];
                if !userinfo.is_empty()
                    && !userinfo.contains('/')
                    && !userinfo.contains(' ')
                    && userinfo.contains(':')
                {
                    out.replace_range(after..at + 1, "");
                    search_from = after;
                    continue;
                }
            }
            search_from = after;
        }
    }
    out
}

/// Fail-closed diagnostic for a git auth failure. Never includes the token.
pub fn git_auth_rejected(url: &str, stderr: &str, auth: &GitAuth) -> GitAuthError {
    let url = sanitize_stored_git_url(url);
    let reason = redact_secrets(stderr, auth);
    let reason = if reason.is_empty() {
        "authentication failed".to_string()
    } else {
        reason
    };
    GitAuthError::Rejected { url, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn lookup<'a>(map: &'a HashMap<String, String>) -> impl FnMut(&str) -> Option<String> + 'a {
        |k| map.get(k).cloned()
    }

    #[test]
    fn k11_01_https_token_from_env() {
        let map = vars(&[("DRACONIC_GIT_TOKEN", "s3cret-token")]);
        match git_auth_from_vars(lookup(&map)) {
            GitAuth::HttpsToken { username, token } => {
                assert_eq!(username, "git");
                assert_eq!(token, "s3cret-token");
            }
            other => panic!("expected HttpsToken, got {other:?}"),
        }
    }

    #[test]
    fn k11_01_https_token_user_from_env() {
        let map = vars(&[
            ("DRACONIC_GIT_TOKEN", "s3cret-token"),
            ("DRACONIC_GIT_TOKEN_USER", "x-access-token"),
        ]);
        match git_auth_from_vars(lookup(&map)) {
            GitAuth::HttpsToken { username, token } => {
                assert_eq!(username, "x-access-token");
                assert_eq!(token, "s3cret-token");
            }
            other => panic!("expected HttpsToken, got {other:?}"),
        }
    }

    #[test]
    fn k11_01_ssh_identity_from_env() {
        let map = vars(&[("DRACONIC_GIT_SSH_KEY", "/home/me/.ssh/id_ed25519")]);
        match git_auth_from_vars(lookup(&map)) {
            GitAuth::Ssh { identity_file } => {
                assert_eq!(
                    identity_file.as_deref(),
                    Some(Path::new("/home/me/.ssh/id_ed25519"))
                );
            }
            other => panic!("expected Ssh, got {other:?}"),
        }
    }

    #[test]
    fn k11_01_env_without_credentials_is_none() {
        let map = vars(&[]);
        assert_eq!(git_auth_from_vars(lookup(&map)), GitAuth::None);
    }

    #[test]
    fn k11_01_token_env_wins_over_ssh_key() {
        let map = vars(&[
            ("DRACONIC_GIT_TOKEN", "s3cret-token"),
            ("DRACONIC_GIT_SSH_KEY", "/home/me/.ssh/id_ed25519"),
        ]);
        assert!(matches!(
            git_auth_from_vars(lookup(&map)),
            GitAuth::HttpsToken { .. }
        ));
    }

    #[test]
    fn k11_01_missing_https_token_fails_closed() {
        let err = GitAuth::https_token("git", "").expect_err("empty token");
        assert_eq!(err, GitAuthError::MissingHttpsToken);
        let msg = err.to_string();
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("DRACONIC_GIT_TOKEN"), "{msg}");
    }

    #[test]
    fn k11_01_missing_ssh_identity_fails_closed() {
        let err = GitAuth::ssh_identity("/no/such/k11-01-ssh-key").expect_err("missing file");
        match &err {
            GitAuthError::MissingSshIdentity { path } => {
                assert!(path.contains("k11-01-ssh-key"), "{path}");
            }
            other => panic!("expected MissingSshIdentity, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("SSH"), "{msg}");
    }

    #[test]
    fn k11_01_sanitize_strips_https_userinfo() {
        assert_eq!(
            sanitize_stored_git_url("https://git:s3cret-token@github.com/org/lib.git"),
            "https://github.com/org/lib.git"
        );
        assert_eq!(
            sanitize_stored_git_url("http://user:p@git.example.com/org/lib.git"),
            "http://git.example.com/org/lib.git"
        );
        assert_eq!(
            sanitize_stored_git_url("https://github.com/org/lib.git"),
            "https://github.com/org/lib.git"
        );
        assert_eq!(
            sanitize_stored_git_url("git@github.com:org/lib.git"),
            "git@github.com:org/lib.git"
        );
        assert_eq!(
            sanitize_stored_git_url("ssh://git@github.com/org/lib.git"),
            "ssh://git@github.com/org/lib.git"
        );
    }

    #[test]
    fn k11_01_clone_url_embeds_https_token() {
        let auth = GitAuth::https_token("git", "s3cret-token").unwrap();
        let clone = clone_url_with_auth("https://github.com/org/lib.git", &auth).expect("embed");
        assert_eq!(clone, "https://git:s3cret-token@github.com/org/lib.git");
        assert_eq!(
            sanitize_stored_git_url(&clone),
            "https://github.com/org/lib.git"
        );
    }

    #[test]
    fn k11_01_clone_url_empty_https_token_fails_closed() {
        let auth = GitAuth::HttpsToken {
            username: "git".into(),
            token: String::new(),
        };
        let err =
            clone_url_with_auth("https://github.com/org/lib.git", &auth).expect_err("empty token");
        assert_eq!(err, GitAuthError::MissingHttpsToken);
    }

    #[test]
    fn k11_01_clone_url_missing_ssh_identity_fails_closed() {
        let auth = GitAuth::Ssh {
            identity_file: Some(PathBuf::from("/no/such/k11-01-ssh-key")),
        };
        let err =
            clone_url_with_auth("git@github.com:org/lib.git", &auth).expect_err("missing identity");
        assert!(matches!(err, GitAuthError::MissingSshIdentity { .. }));
    }

    #[test]
    fn k11_01_clone_url_does_not_embed_token_in_ssh_or_file() {
        let auth = GitAuth::https_token("git", "s3cret-token").unwrap();
        assert_eq!(
            clone_url_with_auth("git@github.com:org/lib.git", &auth).unwrap(),
            "git@github.com:org/lib.git"
        );
        assert_eq!(
            clone_url_with_auth("file:///tmp/fixture.git", &auth).unwrap(),
            "file:///tmp/fixture.git"
        );
    }

    #[test]
    fn k11_01_git_ssh_command_uses_identity_and_batchmode() {
        let auth = GitAuth::Ssh {
            identity_file: Some(PathBuf::from("/home/me/.ssh/id_ed25519")),
        };
        let cmd = git_ssh_command(&auth).expect("ssh command");
        assert!(cmd.contains("ssh"), "{cmd}");
        assert!(cmd.contains("/home/me/.ssh/id_ed25519"), "{cmd}");
        assert!(cmd.contains("BatchMode=yes"), "{cmd}");
        assert!(cmd.contains("IdentitiesOnly=yes"), "{cmd}");
    }

    #[test]
    fn k11_01_is_git_auth_failure_needles() {
        assert!(is_git_auth_failure(
            "fatal: Authentication failed for 'https://github.com/org/lib.git'"
        ));
        assert!(is_git_auth_failure("Permission denied (publickey)."));
        assert!(is_git_auth_failure(
            "could not read Username for 'https://github.com'"
        ));
        assert!(is_git_auth_failure(
            "fatal: could not read from remote repository."
        ));
        assert!(is_git_auth_failure("remote: Invalid username or password"));
        assert!(!is_git_auth_failure(
            "fatal: repository '/tmp/nope' does not exist"
        ));
        assert!(!is_git_auth_failure(""));
    }

    #[test]
    fn k11_01_rejected_credentials_diagnostic_redacts_token() {
        let auth = GitAuth::https_token("git", "s3cret-token").unwrap();
        let err = git_auth_rejected(
            "https://git:s3cret-token@github.com/org/lib.git",
            "fatal: Authentication failed for 'https://git:s3cret-token@github.com/org/lib.git'",
            &auth,
        );
        match &err {
            GitAuthError::Rejected { url, reason } => {
                assert_eq!(url, "https://github.com/org/lib.git");
                assert!(!reason.contains("s3cret-token"), "{reason}");
                assert!(!url.contains("s3cret-token"), "{url}");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("rejected") || msg.contains("auth"), "{msg}");
        assert!(!msg.contains("s3cret-token"), "{msg}");
        assert!(msg.contains("github.com/org/lib.git"), "{msg}");
    }

    #[test]
    fn k11_01_redact_secrets_strips_token_and_userinfo() {
        let auth = GitAuth::https_token("git", "s3cret-token").unwrap();
        let redacted = redact_secrets(
            "fatal: Authentication failed for 'https://git:s3cret-token@github.com/x.git'",
            &auth,
        );
        assert!(!redacted.contains("s3cret-token"), "{redacted}");
        assert!(redacted.contains("github.com/x.git"), "{redacted}");
    }

    #[test]
    fn k11_01_ssh_identity_accepts_existing_file() {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "draconic-k11-01-key-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, "dummy-key\n").unwrap();
        let auth = GitAuth::ssh_identity(&path).expect("existing file");
        match auth {
            GitAuth::Ssh { identity_file } => {
                assert_eq!(identity_file.as_deref(), Some(path.as_path()));
            }
            other => panic!("expected Ssh, got {other:?}"),
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn k11_01_is_ssh_and_https_url_kinds() {
        assert!(is_ssh_git_url("git@github.com:org/lib.git"));
        assert!(is_ssh_git_url("ssh://git@github.com/org/lib.git"));
        assert!(!is_ssh_git_url("https://github.com/org/lib.git"));
        assert!(is_https_git_url("https://github.com/org/lib.git"));
        assert!(is_https_git_url("http://git.example.com/lib.git"));
        assert!(!is_https_git_url("git@github.com:org/lib.git"));
    }
}
