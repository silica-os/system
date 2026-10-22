//! `silica-recovery`: the text menu of the recovery system.

use silica_efi::cmd;
use silica_efi::efivars::EfiVars;
use silica_efi::error::{Context, Error, Result};
use silica_efi::loadopt::utf16_bytes;
use silica_efi::manager::{set_vendor_var, BootManager};
use silica_efi::roots::Roots;
use std::io::{BufRead, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub const FACTORY_RESET_GUID: &str = "8cf2644b-4b0b-428f-9387-6d876050dc67";
pub const FACTORY_RESET_VAR: &str = "FactoryResetRequest";

pub const MENU: &str = "\
Silica Recovery

  1) Start Silica
  2) Roll back to the previous version
  3) Install Silica on a disk
  4) Erase all data and reset
  5) Console
  6) Shut down
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Start,
    Rollback,
    Install,
    Erase,
    Console,
    Shutdown,
}

pub fn parse_choice(line: &str) -> Option<Choice> {
    match line.trim() {
        "1" => Some(Choice::Start),
        "2" => Some(Choice::Rollback),
        "3" => Some(Choice::Install),
        "4" => Some(Choice::Erase),
        "5" => Some(Choice::Console),
        "6" => Some(Choice::Shutdown),
        _ => None,
    }
}

/// What the menu can do to the machine. The real one is [`System`].
pub trait Actions {
    fn reboot(&mut self) -> Result<()>;
    fn poweroff(&mut self) -> Result<()>;
    /// Sets BootNext to the other Silica version; returns what it chose.
    fn prepare_rollback(&mut self) -> Result<String>;
    /// Replaces this process; returns only on failure.
    fn exec_install(&mut self) -> Error;
    fn request_factory_reset(&mut self) -> Result<()>;
    /// Replaces this process; returns only on failure.
    fn exec_console(&mut self) -> Error;
}

fn say(out: &mut dyn Write, text: &str) -> Result<()> {
    out.write_all(text.as_bytes())
        .and_then(|_| out.flush())
        .map_err(|e| Error::new(format!("cannot write to the console: {e}")))
}

fn read_line(input: &mut dyn BufRead) -> Result<String> {
    let mut line = String::new();
    // Invalid UTF-8 from a noisy terminal is read lossily, not fatally.
    let mut raw = Vec::new();
    let n = input
        .read_until(b'\n', &mut raw)
        .map_err(|e| Error::new(format!("cannot read from the console: {e}")))?;
    if n == 0 {
        return Err(Error::new("the console was closed"));
    }
    line.push_str(&String::from_utf8_lossy(&raw));
    Ok(line)
}

/// Shows the menu until something ends the session. `Ok` means the machine
/// is restarting or shutting down.
pub fn run_menu(
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    actions: &mut dyn Actions,
) -> Result<()> {
    loop {
        say(out, &format!("\n{MENU}\nChoose 1-6: "))?;
        let line = read_line(input)?;
        let Some(choice) = parse_choice(&line) else {
            say(out, "\nPlease type a number from 1 to 6.\n")?;
            continue;
        };
        match choice {
            Choice::Start => {
                say(out, "\nStarting Silica...\n")?;
                match actions.reboot() {
                    Ok(()) => return Ok(()),
                    Err(e) => say(out, &format!("\nCannot restart: {e}\n"))?,
                }
            }
            Choice::Rollback => match actions.prepare_rollback() {
                Ok(what) => {
                    say(out, &format!("\nNext start: {what}. Restarting...\n"))?;
                    match actions.reboot() {
                        Ok(()) => return Ok(()),
                        Err(e) => say(out, &format!("\nCannot restart: {e}\n"))?,
                    }
                }
                Err(e) => say(out, &format!("\nCannot roll back: {e}\n"))?,
            },
            Choice::Install => {
                say(out, "\nStarting the installer...\n")?;
                let e = actions.exec_install();
                say(out, &format!("\nCannot start the installer: {e}\n"))?;
            }
            Choice::Erase => {
                say(
                    out,
                    "\nThis erases ALL data of every user on this computer and starts Silica as new.\nType ERASE (in capitals) to confirm, anything else cancels: ",
                )?;
                if read_line(input)?.trim() != "ERASE" {
                    say(out, "\nCancelled. Nothing was changed.\n")?;
                    continue;
                }
                match actions.request_factory_reset() {
                    Ok(()) => {
                        say(out, "\nReset requested. Restarting...\n")?;
                        match actions.reboot() {
                            Ok(()) => return Ok(()),
                            Err(e) => say(
                                out,
                                &format!("\nThe reset is requested but the restart failed: {e}\nRestart the computer by hand.\n"),
                            )?,
                        }
                    }
                    Err(e) => say(out, &format!("\nCannot request the reset: {e}\n"))?,
                }
            }
            Choice::Console => {
                say(out, "\nStarting a console...\n")?;
                let e = actions.exec_console();
                say(out, &format!("\nCannot start a console: {e}\n"))?;
            }
            Choice::Shutdown => {
                say(out, "\nShutting down...\n")?;
                match actions.poweroff() {
                    Ok(()) => return Ok(()),
                    Err(e) => say(out, &format!("\nCannot shut down: {e}\n"))?,
                }
            }
        }
    }
}

/// Chooses the rollback target and sets BootNext.
pub fn prepare_rollback(mgr: &BootManager) -> Result<String> {
    let n = mgr.rollback_target()?;
    let option = mgr
        .read_entry(n)?
        .ok_or_else(|| Error::new("the boot entry to roll back to disappeared"))?;
    mgr.vars().set_boot_next(n)?;
    Ok(option.description)
}

/// Writes the firmware variable systemd's factory reset logic looks for.
pub fn write_factory_reset_variable(vars: &EfiVars) -> Result<()> {
    let mut data = utf16_bytes("1");
    data.extend_from_slice(&[0, 0]);
    set_vendor_var(vars, FACTORY_RESET_VAR, FACTORY_RESET_GUID, &data)
}

/// The real machine.
pub struct System {
    pub roots: Roots,
}

fn systemctl(verb: &str) -> Result<()> {
    cmd::run("systemctl", &[verb])
}

fn first_existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

impl Actions for System {
    fn reboot(&mut self) -> Result<()> {
        systemctl("reboot")
    }

    fn poweroff(&mut self) -> Result<()> {
        systemctl("poweroff")
    }

    fn prepare_rollback(&mut self) -> Result<String> {
        let mgr = BootManager::new(EfiVars::new(&self.roots.efivars));
        prepare_rollback(&mgr)
    }

    fn exec_install(&mut self) -> Error {
        let program = cmd::find_program("silica-install")
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("silica-install")))
                    .filter(|p| p.is_file())
            })
            .unwrap_or_else(|| PathBuf::from("silica-install"));
        let err = Command::new(&program).exec();
        Error::new(format!("{}: {err}", program.display()))
    }

    fn request_factory_reset(&mut self) -> Result<()> {
        if let Some(tool) = cmd::find_program("systemd-factory-reset") {
            return cmd::run(&tool.to_string_lossy(), &["request"]);
        }
        write_factory_reset_variable(&EfiVars::new(&self.roots.efivars)).context(|| {
            "systemd-factory-reset is not installed and the firmware variable could not be set"
                .to_string()
        })
    }

    fn exec_console(&mut self) -> Error {
        let Some(program) = first_existing(&["/usr/bin/login", "/bin/sh", "/usr/bin/sh"]) else {
            return Error::new("neither /usr/bin/login nor /bin/sh exists");
        };
        let err = Command::new(&program).exec();
        Error::new(format!("{}: {err}", program.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silica_efi::esp::EspInfo;
    use silica_efi::fixture::tmpdir;
    use silica_efi::manager::Position;
    use std::io::Cursor;

    #[derive(Default)]
    struct Fake {
        calls: Vec<&'static str>,
        fail_reboot: bool,
    }

    impl Actions for Fake {
        fn reboot(&mut self) -> Result<()> {
            self.calls.push("reboot");
            if self.fail_reboot {
                Err(Error::new("no systemctl"))
            } else {
                Ok(())
            }
        }
        fn poweroff(&mut self) -> Result<()> {
            self.calls.push("poweroff");
            Ok(())
        }
        fn prepare_rollback(&mut self) -> Result<String> {
            self.calls.push("rollback");
            Ok("Silica 1.0.0".into())
        }
        fn exec_install(&mut self) -> Error {
            self.calls.push("install");
            Error::new("not found")
        }
        fn request_factory_reset(&mut self) -> Result<()> {
            self.calls.push("reset");
            Ok(())
        }
        fn exec_console(&mut self) -> Error {
            self.calls.push("console");
            Error::new("not found")
        }
    }

    fn drive(input: &str, fake: &mut Fake) -> (Result<()>, String) {
        let mut out = Vec::new();
        let r = run_menu(&mut Cursor::new(input.as_bytes().to_vec()), &mut out, fake);
        (r, String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn choices() {
        assert_eq!(parse_choice(" 4\n"), Some(Choice::Erase));
        assert_eq!(parse_choice("7"), None);
        assert_eq!(parse_choice(""), None);
        assert_eq!(parse_choice("one"), None);
        assert_eq!(parse_choice("1 2"), None);
    }

    #[test]
    fn start_and_shutdown() {
        let mut f = Fake::default();
        let (r, out) = drive("1\n", &mut f);
        assert!(r.is_ok() && f.calls == ["reboot"], "{out}");
        let mut f = Fake::default();
        let (r, _) = drive("6\n", &mut f);
        assert!(r.is_ok() && f.calls == ["poweroff"]);
    }

    #[test]
    fn bad_input_is_survived() {
        let mut f = Fake::default();
        let (r, out) = drive("\nabc\n\u{0}\n99\n  \n6\n", &mut f);
        assert!(r.is_ok());
        assert_eq!(out.matches("Please type a number").count(), 5);
        let mut f = Fake::default();
        let (r, _) = drive("\u{ff}\u{fe}\n6\n", &mut f);
        assert!(r.is_ok());
        // invalid UTF-8 bytes
        let mut out = Vec::new();
        let mut f = Fake::default();
        let r = run_menu(
            &mut Cursor::new(vec![0xff, 0xfe, b'\n', b'6', b'\n']),
            &mut out,
            &mut f,
        );
        assert!(r.is_ok() && f.calls == ["poweroff"]);
    }

    #[test]
    fn closed_console_is_an_error() {
        let mut f = Fake::default();
        let (r, _) = drive("", &mut f);
        assert!(r.unwrap_err().to_string().contains("closed"));
    }

    #[test]
    fn erase_needs_exact_confirmation() {
        let mut f = Fake::default();
        let (r, out) = drive("4\nerase\n6\n", &mut f);
        assert!(r.is_ok() && f.calls == ["poweroff"], "{out}");
        assert!(out.contains("Cancelled"));
        let mut f = Fake::default();
        let (r, _) = drive("4\nERASE\n", &mut f);
        assert!(r.is_ok() && f.calls == ["reset", "reboot"]);
    }

    #[test]
    fn failures_return_to_the_menu() {
        let mut f = Fake {
            fail_reboot: true,
            ..Fake::default()
        };
        let (r, out) = drive("1\n2\n3\n5\n6\n", &mut f);
        assert!(r.is_ok());
        assert_eq!(
            f.calls,
            ["reboot", "rollback", "reboot", "install", "console", "poweroff"]
        );
        assert!(out.contains("Cannot restart") && out.contains("Cannot start the installer"));
    }

    #[test]
    fn rollback_sets_boot_next() {
        let d = tmpdir("rec-rollback");
        let mgr = BootManager::new(EfiVars::new(&d));
        assert!(prepare_rollback(&mgr).is_err());
        let esp = EspInfo {
            disk: "vda".into(),
            partition_number: 1,
            start: 1,
            size: 1,
            guid: [1; 16],
        };
        for v in ["1.0.0", "1.1.0"] {
            mgr.add(
                &format!("Silica {v}"),
                &esp,
                &format!("\\EFI\\Linux\\silica_{v}.efi"),
                None,
                Position::Append,
            )
            .unwrap();
        }
        // not booted from Silica: current = newest, so roll back to 1.0.0
        let what = prepare_rollback(&mgr).unwrap();
        assert_eq!(what, "Silica 1.0.0");
        assert_eq!(mgr.vars().boot_next().unwrap(), Some(0));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn factory_reset_variable_content() {
        let d = tmpdir("rec-reset");
        let vars = EfiVars::new(&d);
        write_factory_reset_variable(&vars).unwrap();
        let raw =
            std::fs::read(d.join(format!("{FACTORY_RESET_VAR}-{FACTORY_RESET_GUID}"))).unwrap();
        assert_eq!(raw, vec![7, 0, 0, 0, b'1', 0, 0, 0]);
        let _ = std::fs::remove_dir_all(d);
    }
}
