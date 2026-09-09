use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use draconic_conformance::{
    load_path, run_fixture, run_fixture_cov, CoverageReport, Fixture, RunResult,
};

pub fn cmd_test(args: &[String]) -> ExitCode {
    let opts = match parse_test_args(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic test [--coverage] [--jobs <n>] <path>");
            eprintln!("  --coverage  report JS line coverage for fixture sources (U11)");
            eprintln!("  --jobs <n>  worker pool size (C04; default N>1 when multiple fixtures)");
            eprintln!("  <path>      fixture directory or single .drac file (with optional .meta)");
            return ExitCode::from(2);
        }
    };

    if let Err(code) = crate::toolchain_pin::enforce(&opts.path) {
        return code;
    }

    let fixtures = match load_path(&opts.path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    if fixtures.is_empty() {
        eprintln!("error: no .drac fixtures under {}", opts.path.display());
        return ExitCode::from(1);
    }

    let jobs = test_jobs(&opts, fixtures.len());
    let mut coverage = if opts.coverage {
        Some(CoverageReport::new())
    } else {
        None
    };
    let batches = if let Some(report) = coverage.as_mut() {
        let mut batches = Vec::with_capacity(fixtures.len());
        for fixture in &fixtures {
            batches.push(run_fixture_cov(fixture, Some(report)));
        }
        batches
    } else {
        run_fixtures_pool(&fixtures, jobs)
    };

    let mut flat: Vec<&RunResult> = batches.iter().flatten().collect();
    flat.sort_by(|a, b| {
        a.fixture_id
            .cmp(&b.fixture_id)
            .then_with(|| a.target.as_str().cmp(b.target.as_str()))
    });

    let mut passed = 0u32;
    let mut failed = 0u32;
    for result in flat {
        if result.ok {
            passed += 1;
            println!("ok {} {}", result.fixture_id, result.target.as_str());
        } else {
            failed += 1;
            println!(
                "FAIL {} {}: {}",
                result.fixture_id,
                result.target.as_str(),
                result.message
            );
        }
    }

    if let Some(report) = &coverage {
        print!("{}", report.format_summary());
    }

    let total = passed + failed;
    if failed == 0 {
        println!("{passed} passed");
        ExitCode::SUCCESS
    } else {
        println!("{passed} passed, {failed} failed, {total} total");
        ExitCode::from(1)
    }
}

struct TestOpts {
    path: PathBuf,
    coverage: bool,
    jobs: Option<usize>,
}

fn parse_test_args(args: &[String]) -> Result<TestOpts, String> {
    let mut coverage = false;
    let mut jobs: Option<usize> = None;
    let mut path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic test [--coverage] [--jobs <n>] <path>".into());
            }
            "--coverage" => coverage = true,
            "--jobs" => {
                i += 1;
                let raw = args
                    .get(i)
                    .ok_or_else(|| "missing value for --jobs".to_string())?;
                jobs = Some(parse_jobs(raw)?);
            }
            other if other.starts_with("--jobs=") => {
                jobs = Some(parse_jobs(&other["--jobs=".len()..])?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if path.is_some() {
                    return Err("usage: draconic test [--coverage] [--jobs <n>] <path>".into());
                }
                path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    let path =
        path.ok_or_else(|| "usage: draconic test [--coverage] [--jobs <n>] <path>".to_string())?;
    Ok(TestOpts {
        path,
        coverage,
        jobs,
    })
}

fn parse_jobs(raw: &str) -> Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|_| format!("invalid --jobs value: {raw}"))
        .and_then(|n| {
            if n == 0 {
                Err("--jobs must be >= 1".into())
            } else {
                Ok(n)
            }
        })
}

fn test_jobs(opts: &TestOpts, fixture_count: usize) -> usize {
    if let Some(n) = opts.jobs {
        return n.max(1);
    }
    if let Ok(raw) = env::var("DRACONIC_TEST_JOBS") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            return n.max(1);
        }
    }
    if fixture_count <= 1 {
        return 1;
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .max(2)
}

fn run_fixtures_pool(fixtures: &[Fixture], jobs: usize) -> Vec<Vec<RunResult>> {
    let jobs = jobs.max(1).min(fixtures.len().max(1));
    if jobs == 1 || fixtures.len() <= 1 {
        return fixtures.iter().map(run_fixture).collect();
    }
    let next = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<Vec<RunResult>>>> =
        (0..fixtures.len()).map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= fixtures.len() {
                    break;
                }
                let results = run_fixture(&fixtures[i]);
                *slots[i].lock().expect("test worker slot") = Some(results);
            });
        }
    });
    slots
        .into_iter()
        .map(|m| m.into_inner().expect("test worker slot").expect("filled"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_args_coverage() {
        let p = parse_test_args(&["--coverage".into(), "fix".into()]).unwrap();
        assert!(p.coverage);
        assert_eq!(p.path, PathBuf::from("fix"));
        let p2 = parse_test_args(&["fix".into(), "--coverage".into()]).unwrap();
        assert!(p2.coverage);
        let p3 = parse_test_args(&["fix".into()]).unwrap();
        assert!(!p3.coverage);
    }

    #[test]
    fn parse_test_args_jobs() {
        let p = parse_test_args(&["--jobs".into(), "4".into(), "fix".into()]).unwrap();
        assert_eq!(p.jobs, Some(4));
        let p2 = parse_test_args(&["--jobs=3".into(), "fix".into()]).unwrap();
        assert_eq!(p2.jobs, Some(3));
        let p3 = parse_test_args(&["fix".into()]).unwrap();
        assert_eq!(p3.jobs, None);
    }
}
