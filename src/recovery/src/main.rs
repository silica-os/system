use silica_efi::roots::Roots;
use silica_recovery::{run_menu, System};
use std::io::{self, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = BufReader::new(io::stdin());
    let mut stdout = io::stdout();
    let mut system = System {
        roots: Roots::from_env(),
    };
    match run_menu(&mut input, &mut stdout, &mut system) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("silica-recovery: {e}");
            ExitCode::from(1)
        }
    }
}
