use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(err) = skillpack::cli::run() {
        eprintln!("{err:?}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
