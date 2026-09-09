use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

/// ROADMAP K05 / K05.01: `draconic get <module_path>@<ver>` — fetch, update manifest+lock+cache.
pub fn cmd_get(args: &[String]) -> ExitCode {
    let parsed = match parse_get_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!(
                "usage: draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]"
            );
            return ExitCode::from(2);
        }
    };

    let workspace = parsed
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Err(code) = crate::toolchain_pin::enforce(&workspace) {
        return code;
    }
    let cache_root = parsed
        .cache_dir
        .unwrap_or_else(|| draconic_pkg::default_cache_root(&workspace));
    let cache = draconic_pkg::ModuleCache::new(cache_root);

    match draconic_pkg::get_package_spec(&workspace, &parsed.spec, parsed.url.as_deref(), &cache) {
        Ok(r) => {
            println!(
                "got {}@{} (resolved {}) oid={}",
                r.path, r.version_req, r.resolved_version, r.commit_oid
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct GetArgs {
    spec: String,
    url: Option<String>,
    dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

fn parse_get_args(args: &[String]) -> Result<GetArgs, String> {
    let mut spec: Option<String> = None;
    let mut url: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut cache_dir: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                return Err(
                    "usage: draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]"
                        .into(),
                );
            }
            "--url" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --url".into());
                };
                url = Some(v.clone());
            }
            t if let Some(rest) = t.strip_prefix("--url=") => {
                url = Some(rest.to_string());
            }
            "--dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--dir=") => {
                dir = Some(PathBuf::from(rest));
            }
            "--cache-dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --cache-dir".into());
                };
                cache_dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--cache-dir=") => {
                cache_dir = Some(PathBuf::from(rest));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if spec.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                spec = Some(other.to_string());
            }
        }
        i += 1;
    }
    let spec = spec.ok_or_else(|| "missing <module_path>@<ver>".to_string())?;
    Ok(GetArgs {
        spec,
        url,
        dir,
        cache_dir,
    })
}
