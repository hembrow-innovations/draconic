//! K11: post-v1 packaging is later, not the v1 bar.
//!
//! v1 bar is K01–K08 + K09.02 (manifest, lock, cache, resolve, CLI, import,
//! build, integrity, E2E). Children K11.01–K11.05 exist as opt-in surfaces and
//! are never the default fetch/resolve path.

use std::fmt;

use crate::{
    advisory_from_vars, derive_package_subdir, git_auth_from_vars, module_proxy_from_vars,
    AdvisoryError, AdvisorySource, GitAuth, Manifest, ModuleProxy, ProxyError,
};

/// Later packaging knobs (K11.01–K11.05). All-false is the v1 bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaterPackaging {
    /// K11.01: private git auth (HTTPS token / SSH) is in use.
    pub private_git_auth: bool,
    /// K11.02: a `[replace]` override applies to this module path.
    pub replace: bool,
    /// K11.03: module path maps to a git subdirectory (monorepo).
    pub monorepo_subdir: bool,
    /// K11.04: fetch would try a proxy/mirror (not direct-only git).
    pub module_proxy: bool,
    /// K11.05: a yank/retract advisory source is configured.
    pub yank: bool,
}

/// Error while classifying later packaging from env-shaped vars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaterPackagingError {
    /// `DRACONIC_PROXY` could not be parsed.
    Proxy(ProxyError),
    /// `DRACONIC_ADVISORY` could not be loaded.
    Advisory(AdvisoryError),
}

impl fmt::Display for LaterPackagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaterPackagingError::Proxy(e) => write!(f, "later packaging: {e}"),
            LaterPackagingError::Advisory(e) => write!(f, "later packaging: {e}"),
        }
    }
}

impl std::error::Error for LaterPackagingError {}

impl From<ProxyError> for LaterPackagingError {
    fn from(e: ProxyError) -> Self {
        LaterPackagingError::Proxy(e)
    }
}

impl From<AdvisoryError> for LaterPackagingError {
    fn from(e: AdvisoryError) -> Self {
        LaterPackagingError::Advisory(e)
    }
}

impl LaterPackaging {
    /// v1 bar: none of the later knobs.
    pub fn v1_bar() -> Self {
        Self::default()
    }

    /// True when no later K11 child is opted in.
    pub fn is_v1_bar(&self) -> bool {
        *self == Self::v1_bar()
    }

    /// Classify later knobs from the same inputs v1 get/resolve would see.
    pub fn classify(
        manifest: &Manifest,
        module_path: &str,
        git_url: &str,
        auth: &GitAuth,
        proxy: &ModuleProxy,
        advisory: Option<&AdvisorySource>,
    ) -> Self {
        Self {
            private_git_auth: !matches!(auth, GitAuth::None),
            replace: manifest.replace.contains_key(module_path),
            monorepo_subdir: !derive_package_subdir(module_path, git_url).is_empty(),
            module_proxy: !proxy.is_direct_only(),
            yank: advisory.is_some(),
        }
    }

    /// Classify from env-shaped vars. Unset auth/proxy/advisory is the v1 bar.
    pub fn from_vars<F>(
        manifest: &Manifest,
        module_path: &str,
        git_url: &str,
        mut get: F,
    ) -> Result<Self, LaterPackagingError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let auth = git_auth_from_vars(&mut get);
        let proxy = module_proxy_from_vars(&mut get)?;
        let advisory = advisory_from_vars(&mut get)?;
        Ok(Self::classify(
            manifest,
            module_path,
            git_url,
            &auth,
            &proxy,
            advisory.as_ref(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        advisory_from_vars, default_git_url, derive_package_subdir, git_auth_from_vars,
        module_proxy_from_vars, parse_manifest, resolve_git_url, GitAuth, ModuleProxy, PROXY_ENV,
    };

    fn v1_manifest() -> crate::Manifest {
        parse_manifest(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[urls]
"github.com/org/lib" = "https://git.example.com/org/lib.git"
"#,
        )
        .expect("v1 manifest")
    }

    fn empty_env(_key: &str) -> Option<String> {
        None
    }

    #[test]
    fn k11_v1_surface_does_not_silently_ship_later_features() {
        let manifest = v1_manifest();
        let path = "github.com/org/lib";
        let git_url = resolve_git_url(&manifest, path);

        assert!(manifest.replace.is_empty(), "v1 manifest has no [replace]");
        assert_eq!(git_url, "https://git.example.com/org/lib.git");
        assert_eq!(
            derive_package_subdir(path, &git_url),
            "",
            "default mapped URL is repo root, not a monorepo subdir"
        );
        assert_eq!(git_auth_from_vars(empty_env), GitAuth::None);
        let proxy = module_proxy_from_vars(empty_env).expect("unset proxy");
        assert!(proxy.is_direct_only(), "{proxy:?}");
        assert_eq!(advisory_from_vars(empty_env).expect("unset advisory"), None);

        let later = LaterPackaging::from_vars(&manifest, path, &git_url, empty_env)
            .expect("v1 defaults classify");
        assert_eq!(later, LaterPackaging::v1_bar());
        assert!(later.is_v1_bar());
        assert!(!later.private_git_auth);
        assert!(!later.replace);
        assert!(!later.monorepo_subdir);
        assert!(!later.module_proxy);
        assert!(!later.yank);
    }

    #[test]
    fn k11_default_git_url_is_repo_root_not_monorepo_subdir() {
        let path = "github.com/org/lib";
        let url = default_git_url(path);
        assert_eq!(url, "https://github.com/org/lib.git");
        assert_eq!(derive_package_subdir(path, &url), "");
        let later = LaterPackaging::classify(
            &v1_manifest(),
            path,
            &url,
            &GitAuth::None,
            &ModuleProxy::direct(),
            None,
        );
        assert!(later.is_v1_bar());
        assert!(!later.monorepo_subdir);
    }

    #[test]
    fn k11_01_auth_is_later_not_v1() {
        let manifest = v1_manifest();
        let path = "github.com/org/lib";
        let git_url = resolve_git_url(&manifest, path);
        let later = LaterPackaging::from_vars(&manifest, path, &git_url, |k| match k {
            "DRACONIC_GIT_TOKEN" => Some("s3cret-not-v1".into()),
            _ => None,
        })
        .expect("token env");
        assert!(!later.is_v1_bar());
        assert!(later.private_git_auth);
        assert!(!later.replace);
        assert!(!later.monorepo_subdir);
        assert!(!later.module_proxy);
        assert!(!later.yank);
    }

    #[test]
    fn k11_02_replace_is_later_not_v1() {
        let manifest = parse_manifest(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[replace]
"github.com/org/lib" = { git = "https://github.com/fork/lib.git" }
"#,
        )
        .expect("replace manifest");
        let path = "github.com/org/lib";
        let git_url = resolve_git_url(&manifest, path);
        assert_eq!(git_url, "https://github.com/fork/lib.git");
        let later =
            LaterPackaging::from_vars(&manifest, path, &git_url, empty_env).expect("replace");
        assert!(!later.is_v1_bar());
        assert!(later.replace);
        assert!(!later.private_git_auth);
        assert!(!later.monorepo_subdir);
        assert!(!later.module_proxy);
        assert!(!later.yank);
    }

    #[test]
    fn k11_03_monorepo_subdir_is_later_not_v1() {
        let path = "github.com/org/mono/pkg/foo";
        let git_url = "https://github.com/org/mono.git";
        assert_eq!(derive_package_subdir(path, git_url), "pkg/foo");
        let later = LaterPackaging::classify(
            &v1_manifest(),
            path,
            git_url,
            &GitAuth::None,
            &ModuleProxy::direct(),
            None,
        );
        assert!(!later.is_v1_bar());
        assert!(later.monorepo_subdir);
        assert!(!later.private_git_auth);
        assert!(!later.replace);
        assert!(!later.module_proxy);
        assert!(!later.yank);
    }

    #[test]
    fn k11_04_module_proxy_is_later_not_v1() {
        let manifest = v1_manifest();
        let path = "github.com/org/lib";
        let git_url = resolve_git_url(&manifest, path);
        let later = LaterPackaging::from_vars(&manifest, path, &git_url, |k| {
            if k == PROXY_ENV {
                Some("https://proxy.example.com,direct".into())
            } else {
                None
            }
        })
        .expect("proxy env");
        assert!(!later.is_v1_bar());
        assert!(later.module_proxy);
        assert!(!later.private_git_auth);
        assert!(!later.replace);
        assert!(!later.monorepo_subdir);
        assert!(!later.yank);
    }

    #[test]
    fn k11_05_yank_advisory_is_later_not_v1() {
        let advisory = crate::AdvisorySource::parse("mem", "yank github.com/org/lib 1.0.0\n")
            .expect("advisory");
        let later = LaterPackaging::classify(
            &v1_manifest(),
            "github.com/org/lib",
            "https://git.example.com/org/lib.git",
            &GitAuth::None,
            &ModuleProxy::direct(),
            Some(&advisory),
        );
        assert!(!later.is_v1_bar());
        assert!(later.yank);
        assert!(!later.private_git_auth);
        assert!(!later.replace);
        assert!(!later.monorepo_subdir);
        assert!(!later.module_proxy);
    }
}
