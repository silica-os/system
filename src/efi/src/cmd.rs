//! Running external programs and finding them.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Command;

/// Runs `program` with the console inherited and fails on a non-zero exit.
pub fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::new(format!("{program} is not installed"))
        } else {
            Error::new(format!("cannot run {program}: {e}"))
        }
    })?;
    if status.success() {
        Ok(())
    } else {
        match status.code() {
            Some(c) => Err(Error::new(format!("{program} failed with exit code {c}"))),
            None => Err(Error::new(format!("{program} was killed by a signal"))),
        }
    }
}

/// Looks `name` up in `PATH` and in the usual systemd directories.
pub fn find_program(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in ["/usr/bin", "/usr/sbin", "/usr/lib/systemd", "/bin", "/sbin"] {
        dirs.push(PathBuf::from(extra));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_reports_failures() {
        assert!(run("true", &[]).is_ok());
        let e = run("false", &[]).unwrap_err();
        assert!(e.to_string().contains("exit code 1"));
        let e = run("/nonexistent/prog", &[]).unwrap_err();
        assert!(e.to_string().contains("not installed"));
        assert!(find_program("sh").is_some());
        assert!(find_program("definitely-not-a-program").is_none());
    }
}
