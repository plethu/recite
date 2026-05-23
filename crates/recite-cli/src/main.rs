use std::process::ExitCode;

fn main() -> ExitCode {
    recite_cli::run(std::env::args_os())
}
