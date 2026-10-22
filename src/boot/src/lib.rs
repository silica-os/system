//! `silica-boot`: manage the UEFI boot entries of Silica.
//!
//! Silica has no bootloader; the firmware starts signed UKIs from the ESP
//! through its own `Boot####` entries. This tool keeps those entries in step
//! with the UKIs on the ESP and drives the A/B update flow.

use silica_efi::args::{self, Parsed};
use silica_efi::efivars::EfiVars;
use silica_efi::error::{Context, Error, Failure, Result};
use silica_efi::esp::{self, EspInfo};
use silica_efi::guid::format_guid;
use silica_efi::loadopt::LoadOption;
use silica_efi::manager::{kind, scan_ukis, Bless, BootManager, Kind, Position, SyncOptions};
use silica_efi::roots::Roots;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const USAGE: &str = "\
usage: silica-boot [options] <command>

commands:
  status                 show BootCurrent, BootNext, BootOrder and all entries
  add --label L --path P [--options @1]
                         create (or reuse) an entry; P is a UEFI path such as
                         '\\EFI\\Linux\\silica_1.0.efi'
  next NNNN              boot entry NNNN (hex) on the next boot only
  bless                  put the running Silica entry first in BootOrder
  sync                   make entries match EFI/Linux/silica_*.efi on the ESP
                         and set BootNext when a newer version is installed

options:
  --esp DIR              ESP mount point (default /efi)
  --esp-device DEV       ESP device node instead of looking up --esp
  --efivars DIR          UEFI variable directory (default /sys/firmware/efi/efivars)
  --sysfs DIR            sysfs directory (default /sys)
  --booted VERSION       (sync) the running version, when known
environment:
  SILICA_ROOT            treat this directory as the root of the system
";

pub fn run(args: &[String], roots: Roots, out: &mut dyn Write) -> std::result::Result<(), Failure> {
    let parsed = args::parse(
        args,
        &[
            "esp",
            "esp-device",
            "efivars",
            "sysfs",
            "label",
            "path",
            "options",
            "booted",
        ],
        &["help"],
    )
    .map_err(|e| Failure::Usage(e.to_string()))?;
    if parsed.flag("help") {
        write_out(out, USAGE)?;
        return Ok(());
    }
    let mut roots = roots;
    if let Some(s) = parsed.value("sysfs") {
        roots = roots.with_sysfs(s);
    }
    if let Some(v) = parsed.value("efivars") {
        roots = roots.with_efivars(v);
    }
    let Some(command) = parsed.positional.first() else {
        return Err(Failure::Usage("missing command".into()));
    };
    let mgr = BootManager::new(EfiVars::new(&roots.efivars));
    match command.as_str() {
        "status" => status(&mgr, out)?,
        "add" => add(&mgr, &roots, &parsed, out)?,
        "next" => next(&mgr, &parsed, out)?,
        "bless" => bless(&mgr, &roots, out)?,
        "sync" => sync(&mgr, &roots, &parsed, out)?,
        other => return Err(Failure::Usage(format!("unknown command '{other}'"))),
    }
    Ok(())
}

fn write_out(out: &mut dyn Write, text: &str) -> Result<()> {
    out.write_all(text.as_bytes())
        .map_err(|e| Error::new(format!("cannot write output: {e}")))
}

fn hex4(n: u16) -> String {
    format!("{n:04X}")
}

fn describe(option: &LoadOption) -> String {
    let path = option.file_path().unwrap_or("-");
    let mut s = format!("{}  {}", option.description, path);
    if !option.optional_data.is_empty() {
        match option.optional_text() {
            Some(t) => s.push_str(&format!("  data=\"{t}\"")),
            None => {
                let hex: String = option
                    .optional_data
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                s.push_str(&format!("  data=0x{hex}"));
            }
        }
    }
    if let Some(h) = option.hard_drive() {
        s.push_str(&format!(
            "  (partition {}, {})",
            h.partition_number,
            format_guid(&h.signature)
        ));
    }
    s
}

fn status(mgr: &BootManager, out: &mut dyn Write) -> Result<()> {
    let vars = mgr.vars();
    let entries = mgr.entries()?;
    let name_of = |n: u16| -> String {
        entries
            .iter()
            .find(|(m, _)| *m == n)
            .and_then(|(_, r)| r.as_ref().ok())
            .map(|o| format!("{} ({})", hex4(n), o.description))
            .unwrap_or_else(|| hex4(n))
    };
    let current = vars.boot_current()?;
    let next = vars.boot_next()?;
    let order = vars.boot_order()?;
    let mut text = String::new();
    text.push_str(&format!(
        "BootCurrent: {}\n",
        current.map(name_of).unwrap_or_else(|| "not set".into())
    ));
    text.push_str(&format!(
        "BootNext:    {}\n",
        next.map(name_of).unwrap_or_else(|| "not set".into())
    ));
    text.push_str(&format!(
        "BootOrder:   {}\n",
        if order.is_empty() {
            "empty".to_string()
        } else {
            order.iter().map(|n| hex4(*n)).collect::<Vec<_>>().join(",")
        }
    ));
    for (n, r) in &entries {
        match r {
            Ok(o) => {
                let star = if o.is_active() { '*' } else { ' ' };
                let tag = if kind(o) == Kind::Other {
                    ""
                } else {
                    "  [silica]"
                };
                text.push_str(&format!(
                    "Boot{}{} {}{}\n",
                    hex4(*n),
                    star,
                    describe(o),
                    tag
                ));
            }
            Err(e) => text.push_str(&format!("Boot{}  <unreadable: {e}>\n", hex4(*n))),
        }
    }
    write_out(out, &text)
}

fn resolve_esp(roots: &Roots, parsed: &Parsed) -> Result<(EspInfo, PathBuf)> {
    let mount = parsed.value("esp").unwrap_or("/efi");
    let dir = roots.path(mount);
    let info = match parsed.value("esp-device") {
        Some(dev) => esp::from_device_name(roots, dev)?,
        None => esp::from_mountpoint(roots, mount)
            .context(|| format!("cannot identify the ESP mounted at {mount}"))?,
    };
    Ok((info, dir))
}

fn add(
    mgr: &BootManager,
    roots: &Roots,
    parsed: &Parsed,
    out: &mut dyn Write,
) -> std::result::Result<(), Failure> {
    let label = parsed
        .value("label")
        .ok_or_else(|| Failure::Usage("add needs --label".into()))?;
    let path = parsed
        .value("path")
        .ok_or_else(|| Failure::Usage("add needs --path".into()))?;
    if !path.starts_with('\\') {
        return Err(Failure::Usage(
            "--path must be a UEFI path starting with a backslash, like '\\EFI\\Linux\\x.efi'"
                .into(),
        ));
    }
    let (info, _) = resolve_esp(roots, parsed)?;
    let (n, created) = mgr.add(
        label,
        &info,
        path,
        parsed.value("options"),
        Position::Append,
    )?;
    write_out(
        out,
        &format!(
            "{} {}\n",
            hex4(n),
            if created { "created" } else { "already exists" }
        ),
    )?;
    Ok(())
}

fn next(
    mgr: &BootManager,
    parsed: &Parsed,
    out: &mut dyn Write,
) -> std::result::Result<(), Failure> {
    let arg = parsed
        .positional
        .get(1)
        .ok_or_else(|| Failure::Usage("next needs an entry number".into()))?;
    let digits = arg.strip_prefix("Boot").unwrap_or(arg);
    let n = u16::from_str_radix(digits, 16).map_err(|_| {
        Failure::Usage(format!("'{arg}' is not a boot entry number (4 hex digits)"))
    })?;
    if mgr.read_entry(n)?.is_none() {
        return Err(Error::new(format!("Boot{} does not exist", hex4(n))).into());
    }
    mgr.vars().set_boot_next(n)?;
    write_out(out, &format!("BootNext set to {}\n", hex4(n)))?;
    Ok(())
}

fn bless(mgr: &BootManager, roots: &Roots, out: &mut dyn Write) -> Result<()> {
    let running = os_release_version(roots);
    let msg = match mgr.bless(running.as_deref())? {
        Bless::Blessed(n) => format!("{} is now first in BootOrder\n", hex4(n)),
        Bless::AlreadyFirst(n) => format!("{} is already first in BootOrder\n", hex4(n)),
        Bless::NotApplicable(why) => format!("nothing to bless: {why}\n"),
    };
    write_out(out, &msg)
}

fn os_release_version(roots: &Roots) -> Option<String> {
    for file in ["/etc/os-release", "/usr/lib/os-release"] {
        let Ok(text) = fs::read_to_string(roots.path(file)) else {
            continue;
        };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("IMAGE_VERSION=") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn sync(mgr: &BootManager, roots: &Roots, parsed: &Parsed, out: &mut dyn Write) -> Result<()> {
    let (info, dir) = resolve_esp(roots, parsed)?;
    let opts = SyncOptions {
        booted: parsed.value("booted").map(str::to_string),
        booted_fallback: os_release_version(roots),
        tried: Some(roots.path("/var/lib/silica/boot/tried")),
    };
    let ukis = scan_ukis(&dir)?;
    let report = mgr.sync(&info, &dir, &opts)?;
    let mut text = format!("{} UKI(s) on the ESP\n", ukis.len());
    for m in report.messages {
        text.push_str(&m);
        text.push('\n');
    }
    write_out(out, &text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use silica_efi::fixture::{fake_system, tmpdir};

    fn run_ok(roots: &Roots, args: &[&str]) -> String {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        match run(&args, roots.clone(), &mut out) {
            Ok(()) => String::from_utf8(out).unwrap(),
            Err(e) => panic!("{e:?}"),
        }
    }

    fn run_err(roots: &Roots, args: &[&str]) -> String {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        match run(&args, roots.clone(), &mut out) {
            Ok(()) => panic!("expected failure"),
            Err(Failure::Usage(m)) => format!("usage: {m}"),
            Err(Failure::Error(e)) => e.to_string(),
        }
    }

    #[test]
    fn full_flow_on_a_fake_system() {
        let roots = fake_system("boot-cli", 512);
        fs::create_dir_all(&roots.efivars).unwrap();
        let uki = |v: &str| {
            fs::write(
                roots.root.join(format!("efi/EFI/Linux/silica_{v}.efi")),
                b"MZ",
            )
            .unwrap();
        };
        uki("1.0.0");
        let out = run_ok(&roots, &["sync"]);
        assert!(out.contains("1 UKI(s)"), "{out}");
        let out = run_ok(&roots, &["status"]);
        assert!(out.contains("Silica 1.0.0"), "{out}");
        assert!(
            out.contains("Silica Recovery") && out.contains("data=\"@1\""),
            "{out}"
        );
        assert!(out.contains("BootOrder:   0001,0000"), "{out}");
        assert!(out.contains("[silica]"));

        // the system says it runs 1.0.0 (BootCurrent = Boot0000)
        EfiVars::new(&roots.efivars)
            .set(
                "BootCurrent",
                "8be4df61-93ca-11d2-aa0d-00e098032b8c",
                6,
                &[1, 0],
            )
            .unwrap();
        uki("1.1.0");
        let out = run_ok(&roots, &["sync"]);
        assert!(out.contains("BootNext set to Boot0002"), "{out}");
        assert!(run_ok(&roots, &["status"]).contains("BootNext:    0002 (Silica 1.1.0)"));

        // 1.1.0 fails: the firmware consumed BootNext and fell back to 1.0.0,
        // so the next sync must not hand 1.1.0 to BootNext again
        EfiVars::new(&roots.efivars).clear_boot_next().unwrap();
        let out = run_ok(&roots, &["sync"]);
        assert!(out.contains("did not start last time"), "{out}");
        assert!(run_ok(&roots, &["status"]).contains("BootNext:    not set"));

        // firmware boots 1.1.0 and consumes BootNext
        EfiVars::new(&roots.efivars).clear_boot_next().unwrap();
        EfiVars::new(&roots.efivars)
            .set(
                "BootCurrent",
                "8be4df61-93ca-11d2-aa0d-00e098032b8c",
                6,
                &[2, 0],
            )
            .unwrap();
        assert!(run_ok(&roots, &["bless"]).contains("0002 is now first"));
        assert!(run_ok(&roots, &["bless"]).contains("already first"));
        assert!(run_ok(&roots, &["status"]).contains("BootOrder:   0002,0001,0000"));

        let out = run_ok(&roots, &["next", "0001"]);
        assert!(out.contains("BootNext set to 0001"));
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn add_and_next_and_errors() {
        let roots = fake_system("boot-cli2", 4096);
        fs::create_dir_all(&roots.efivars).unwrap();
        let args = [
            "add",
            "--label",
            "Test",
            "--path",
            "\\EFI\\Linux\\t.efi",
            "--options",
            "@1",
        ];
        assert_eq!(run_ok(&roots, &args), "0000 created\n");
        assert_eq!(run_ok(&roots, &args), "0000 already exists\n");
        let mut with_dev = args.to_vec();
        with_dev.extend(["--esp-device", "/dev/vda1"]);
        assert_eq!(run_ok(&roots, &with_dev), "0000 already exists\n");

        assert!(run_err(&roots, &["add", "--label", "x"]).starts_with("usage"));
        assert!(run_err(&roots, &["add", "--label", "x", "--path", "EFI/x"]).starts_with("usage"));
        assert!(run_err(&roots, &["next"]).starts_with("usage"));
        assert!(run_err(&roots, &["next", "zz"]).starts_with("usage"));
        assert!(run_err(&roots, &["next", "0042"]).contains("does not exist"));
        assert!(run_err(&roots, &[]).starts_with("usage"));
        assert!(run_err(&roots, &["frobnicate"]).starts_with("usage"));
        assert!(run_err(&roots, &["status", "--bogus"]).starts_with("usage"));
        assert!(run_err(&roots, &["sync", "--esp", "/nowhere"]).contains("cannot identify the ESP"));
        // an unmounted/empty ESP directory is refused
        let _ = fs::remove_dir_all(roots.root.join("efi/EFI"));
        assert!(run_err(&roots, &["sync"]).contains("is the ESP mounted"));
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn flags_override_roots_and_missing_efivars_fail_cleanly() {
        let roots = fake_system("boot-cli3", 512);
        let vars = tmpdir("boot-cli3-vars");
        let v = vars.to_str().unwrap();
        let sys = roots.sysfs.to_str().unwrap().to_string();
        let out = run_ok(
            &Roots::new("/nonexistent"),
            &["--efivars", v, "--sysfs", &sys, "status"],
        );
        assert!(out.contains("BootOrder:   empty"));
        let e = run_err(&Roots::new(&roots.root), &["status"]);
        assert!(e.contains("is this a UEFI boot"), "{e}");
        let _ = fs::remove_dir_all(&roots.root);
        let _ = fs::remove_dir_all(vars);
    }

    #[test]
    fn os_release_fallback() {
        let roots = fake_system("boot-osr", 512);
        fs::create_dir_all(roots.root.join("usr/lib")).unwrap();
        fs::write(
            roots.root.join("usr/lib/os-release"),
            "NAME=Silica\nIMAGE_VERSION=\"1.0.0\"\n",
        )
        .unwrap();
        assert_eq!(os_release_version(&roots).as_deref(), Some("1.0.0"));
        let _ = fs::remove_dir_all(&roots.root);
    }
}
