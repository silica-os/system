use silica_efi::error::Failure;
use silica_efi::roots::Roots;
use silica_install::{run, Tools, USAGE};
use std::io::{self, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input = BufReader::new(io::stdin());
    let mut stdout = io::stdout();
    match run(
        &args,
        &Roots::from_env(),
        &Tools::default(),
        &mut input,
        &mut stdout,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(msg)) => {
            eprintln!("silica-install: {msg}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Error(e)) => {
            eprintln!("silica-install: {e}");
            ExitCode::from(1)
        }
    }
}
