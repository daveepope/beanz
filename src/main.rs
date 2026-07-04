use std::process::ExitCode;

fn main() -> ExitCode {
    beanz::run(std::env::args().skip(1).collect())
}
