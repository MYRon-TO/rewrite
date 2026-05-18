use std::process::ExitCode;

fn main() -> ExitCode {
    match rewrite::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[{:?}]: {error}", error.category());
            ExitCode::FAILURE
        }
    }
}
