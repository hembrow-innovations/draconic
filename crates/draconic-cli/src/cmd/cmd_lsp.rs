use std::process::ExitCode;

pub fn cmd_lsp(args: &[String]) -> ExitCode {
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                println!("usage: draconic lsp");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option: {other}");
                eprintln!("usage: draconic lsp");
                return ExitCode::from(2);
            }
            other => {
                eprintln!("unexpected argument: {other}");
                eprintln!("usage: draconic lsp");
                return ExitCode::from(2);
            }
        }
    }
    match draconic_lsp::serve_stdio() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lsp: {e}");
            ExitCode::from(1)
        }
    }
}
