use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

const MOD_USAGE: &str =
    "usage: draconic mod init <module_path> [--dir <path>]\n       draconic mod tidy [--dir <path>] [--cache-dir <path>]";
const TIDY_USAGE: &str = "usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]";
const INIT_USAGE: &str = "usage: draconic mod init <module_path> [--dir <path>]";

pub fn cmd_mod(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("tidy") => cmd_mod_tidy(&args[1..]),
        Some("init") => cmd_mod_init(&args[1..]),
        Some(other) => {
            eprintln!("unknown mod subcommand: {other}");
            eprintln!("{MOD_USAGE}");
            ExitCode::from(2)
        }
        None => {
            eprintln!("{MOD_USAGE}");
            ExitCode::from(2)
        }
    }
}

fn cmd_mod_init(args: &[String]) -> ExitCode {
    let parsed = match parse_mod_init_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("{INIT_USAGE}");
            return ExitCode::from(2);
        }
    };
    let workspace = parsed
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Err(code) = crate::toolchain_pin::enforce(&workspace) {
        return code;
    }
    match draconic_pkg::mod_init(&workspace, &parsed.module_path) {
        Ok(r) => {
            println!("mod init: {}", r.module);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_mod_tidy(args: &[String]) -> ExitCode {
    let parsed = match parse_mod_tidy_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("{TIDY_USAGE}");
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
struct ModInitArgs {
    module_path: String,
    dir: Option<PathBuf>,
}

fn parse_mod_init_args(args: &[String]) -> Result<ModInitArgs, String> {
    let mut module_path: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                return Err(INIT_USAGE.into());
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
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if module_path.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                module_path = Some(other.to_string());
            }
        }
        i += 1;
    }
    let module_path = module_path.ok_or_else(|| "missing <module_path>".to_string())?;
    Ok(ModInitArgs { module_path, dir })
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
                return Err(TIDY_USAGE.into());
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
