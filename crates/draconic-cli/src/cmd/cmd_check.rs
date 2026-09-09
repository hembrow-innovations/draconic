use std::path::PathBuf;
use std::process::ExitCode;

use draconic_frontend::check_path;

pub fn cmd_check(args: &[String]) -> ExitCode {
    let parsed = match parse_check_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic check [--watch] <file>");
            return ExitCode::from(2);
        }
    };

    if let Err(code) = crate::toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    if parsed.watch {
        return crate::watch::run_watch_loop(&parsed.input, || match check_path(&parsed.input) {
            Ok(_) => {
                crate::watch::touch_watch_marker();
                Ok(())
            }
            Err(d) => Err(d.to_string()),
        });
    }

    match check_path(&parsed.input) {
        Ok(_) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("error: {d}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct CheckArgs {
    input: PathBuf,
    watch: bool,
}

fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut watch = false;
    let mut input: Option<PathBuf> = None;

    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic check [--watch] <file>".into());
            }
            "--watch" => watch = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if input.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                input = Some(PathBuf::from(other));
            }
        }
    }

    let input = input.ok_or_else(|| "missing input file".to_string())?;
    Ok(CheckArgs { input, watch })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_check_args_watch() {
        let args = vec!["--watch".into(), "a.drac".into()];
        let p = parse_check_args(&args).unwrap();
        assert!(p.watch);
        assert_eq!(p.input, PathBuf::from("a.drac"));
    }
}
