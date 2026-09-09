use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

/// ROADMAP K05 / K05.02: `draconic mod tidy` — lock matches manifest; fetch missing; prune unused.
pub fn cmd_mod(args: &[String]) -> ExitCode {
    let sub = match args.first().map(String::as_str) {
        Some("tidy") => "tidy",
        Some(other) => {
            eprintln!("unknown mod subcommand: {other}");
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
            return ExitCode::from(2);
        }
    };
    debug_assert_eq!(sub, "tidy");
    let rest = &args[1..];
    let parsed = match parse_mod_tidy_args(rest) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
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

    match draconic_pkg::mod_tidy(&workspace, &cache) {
        Ok(r) => {
            println!(
                "mod tidy: kept {} fetched {} pruned {}",
                r.kept.len(),
                r.fetched.len(),
                r.pruned.len()
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
struct ModTidyArgs {
    dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

fn parse_mod_tidy_args(args: &[String]) -> Result<ModTidyArgs, String> {
    let mut dir: Option<PathBuf> = None;
    let mut cache_dir: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]".into());
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
                return Err(format!("unexpected argument: {other}"));
            }
        }
        i += 1;
    }
    Ok(ModTidyArgs { dir, cache_dir })
}
