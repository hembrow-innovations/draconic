use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use crate::cmd::cmd_build::{build_program, parse_target, Target};
use crate::work_dir::run_work_dir;

/// ROADMAP U14: `draconic run` — build to a temp artifact and execute immediately.
/// Default target is `js` (Node). Use `--target native` for the LLVM path.
/// Remaining args after the input file are forwarded to the program.
pub fn cmd_run(args: &[String]) -> ExitCode {
    let parsed = match parse_run_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]");
            return ExitCode::from(2);
        }
    };

    if let Err(code) = crate::toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    let work = match run_work_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    let artifact = match parsed.target {
        Target::Js => work.join("out.js"),
        Target::Native => work.join("out"),
    };

    if let Err(d) = build_program(&parsed.input, parsed.target, &artifact, false, &[], false) {
        let _ = fs::remove_dir_all(&work);
        eprintln!("error: {d}");
        return ExitCode::from(1);
    }

    let status = execute_artifact(
        parsed.target,
        &artifact,
        &parsed.program_args,
        &parsed.grants,
    );
    let _ = fs::remove_dir_all(&work);

    match status {
        Ok(code) => exit_from_code(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct RunArgs {
    target: Target,
    input: PathBuf,
    program_args: Vec<String>,
    /// R02.03 opt-in grant subset from `--allow-*` flags.
    grants: Vec<String>,
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut target = Target::Js; // default for shebang / quick scripts
    let mut input: Option<PathBuf> = None;
    let mut program_args: Vec<String> = Vec::new();
    let mut grants: Vec<String> = Vec::new();
    let mut i = 0;
    let mut saw_separator = false;

    while i < args.len() {
        let a = &args[i];
        if saw_separator {
            program_args.push(a.clone());
            i += 1;
            continue;
        }
        match a.as_str() {
            "--" => {
                saw_separator = true;
            }
            "--target" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for --target".to_string())?;
                target = parse_target(val)?;
            }
            t if let Some(rest) = t.strip_prefix("--target=") => {
                target = parse_target(rest)?;
            }
            "--allow-fs-read" => grants.push("fs-read".into()),
            "--allow-fs-write" => grants.push("fs-write".into()),
            "--allow-net-listen" => grants.push("net-listen".into()),
            "--allow-net-connect" => grants.push("net-connect".into()),
            "-h" | "--help" => {
                return Err("usage: draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]".into());
            }
            other if other.starts_with('-') && input.is_none() => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if input.is_none() {
                    input = Some(PathBuf::from(other));
                } else {
                    // After the Program path, remaining tokens are program argv.
                    program_args.push(other.to_string());
                }
            }
        }
        i += 1;
    }

    let input = input.ok_or_else(|| "missing input file".to_string())?;
    Ok(RunArgs {
        target,
        input,
        program_args,
        grants,
    })
}

fn execute_artifact(
    target: Target,
    artifact: &Path,
    program_args: &[String],
    grants: &[String],
) -> Result<i32, String> {
    let mut cmd = match target {
        Target::Js => {
            let mut c = Command::new("node");
            c.arg(artifact);
            c
        }
        Target::Native => Command::new(artifact),
    };
    cmd.args(program_args);
    if !grants.is_empty() {
        cmd.env("DRACONIC_PERMISSIONS", grants.join(","));
    }
    // Inherit stdio so run feels like a real process (shebang-friendly).
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().map_err(|e| match target {
        Target::Js => format!("spawn node failed: {e}"),
        Target::Native => format!("spawn binary failed: {e}"),
    })?;
    Ok(status.code().unwrap_or(1))
}

fn exit_from_code(code: i32) -> ExitCode {
    if code == 0 {
        ExitCode::SUCCESS
    } else if (1..=255).contains(&code) {
        ExitCode::from(code as u8)
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_run_args_defaults_js_and_forwards() {
        let args = vec!["a.drac".into(), "x".into(), "y".into()];
        let p = parse_run_args(&args).unwrap();
        assert_eq!(p.target, Target::Js);
        assert_eq!(p.input, PathBuf::from("a.drac"));
        assert_eq!(p.program_args, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn parse_run_args_native_target() {
        let args = vec!["--target".into(), "native".into(), "a.drac".into()];
        let p = parse_run_args(&args).unwrap();
        assert_eq!(p.target, Target::Native);
        assert_eq!(p.input, PathBuf::from("a.drac"));
        assert!(p.program_args.is_empty());
        assert!(p.grants.is_empty());
    }

    #[test]
    fn parse_run_args_allow_grant_flags() {
        let args = vec![
            "--allow-fs-read".into(),
            "--allow-fs-write".into(),
            "--allow-net-listen".into(),
            "--allow-net-connect".into(),
            "a.drac".into(),
        ];
        let p = parse_run_args(&args).unwrap();
        assert_eq!(
            p.grants,
            vec![
                "fs-read".to_string(),
                "fs-write".to_string(),
                "net-listen".to_string(),
                "net-connect".to_string(),
            ]
        );
        assert_eq!(p.input, PathBuf::from("a.drac"));
    }
}
