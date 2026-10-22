use silica_boot::{run, USAGE};
use silica_efi::error::Failure;
use silica_efi::roots::Roots;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdout = std::io::stdout();
    match run(&args, Roots::from_env(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(msg)) => {
            eprintln!("silica-boot: {msg}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Error(e)) => {
            eprintln!("silica-boot: {e}");
            ExitCode::from(1)
        }
    }
}
