//! `silica-install`: the text-mode installer.
//!
//! Picks a disk, lets `systemd-repart` lay out and fill it from the image's
//! definitions, then writes the UEFI boot entries on the new ESP.

use silica_efi::args;
use silica_efi::cmd;
use silica_efi::efivars::EfiVars;
use silica_efi::error::{Context, Error, Failure, Result};
use silica_efi::esp;
use silica_efi::gpt;
use silica_efi::guid::{parse_guid, ESP_TYPE};
use silica_efi::manager::{kind, scan_ukis, BootManager, Kind, SyncOptions};
use silica_efi::roots::Roots;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const USAGE: &str = "\
usage: silica-install [--yes --disk /dev/NAME] [--definitions DIR]

Installs Silica on a whole disk. Everything on that disk is erased.

  --disk DEV          the target disk (needs --yes to skip the question)
  --yes               do not ask for confirmation
  --definitions DIR   systemd-repart definitions (default /usr/lib/silica/install.repart.d)
environment:
  SILICA_ROOT         treat this directory as the root of the system
";

pub const DEFAULT_DEFINITIONS: &str = "/usr/lib/silica/install.repart.d";

/// The external programs the installer runs.
#[derive(Debug, Clone)]
pub struct Tools {
    pub repart: String,
    pub mount: String,
    pub umount: String,
    pub udevadm: String,
    /// How long to wait for the new partitions to appear.
    pub wait: Duration,
}

impl Default for Tools {
    fn default() -> Self {
        Tools {
            repart: "systemd-repart".into(),
            mount: "mount".into(),
            umount: "umount".into(),
            udevadm: "udevadm".into(),
            wait: Duration::from_secs(20),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub name: String,
    pub path: PathBuf,
    pub model: String,
    pub bytes: u64,
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Disks that hold the running system, found through the mounts of `/efi`,
/// `/usr` and `/` and through device-mapper/RAID slaves.
pub fn running_disks(roots: &Roots) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(text) = fs::read_to_string(&roots.mountinfo) else {
        return out;
    };
    let mounts = esp::parse_mountinfo(&text);
    for want in ["/efi", "/usr", "/"] {
        if let Some((_, maj, min)) = mounts.iter().rev().find(|(mp, _, _)| mp == want) {
            collect_disks(roots, *maj, *min, 0, &mut out);
        }
    }
    out
}

fn collect_disks(roots: &Roots, major: u32, minor: u32, depth: u32, out: &mut BTreeSet<String>) {
    if depth > 8 {
        return;
    }
    let base = roots
        .sysfs
        .join("dev/block")
        .join(format!("{major}:{minor}"));
    let Ok(real) = fs::canonicalize(&base) else {
        return;
    };
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    if real.join("partition").exists() {
        if let Some(disk) = real.parent().and_then(name_of) {
            out.insert(disk);
        }
        return;
    }
    let Some(name) = name_of(&real) else { return };
    let slaves = roots.sysfs.join("block").join(&name).join("slaves");
    let mut found = false;
    if let Ok(read) = fs::read_dir(&slaves) {
        for slave in read.flatten() {
            if let Some((a, b)) = read_trim(&slave.path().join("dev")).and_then(|t| {
                let (a, b) = t.split_once(':')?;
                Some((a.parse().ok()?, b.parse().ok()?))
            }) {
                found = true;
                collect_disks(roots, a, b, depth + 1, out);
            }
        }
    }
    if !found {
        out.insert(name);
    }
}

fn skipped_by_name(name: &str) -> bool {
    ["loop", "zram", "sr", "ram", "dm-", "md", "nbd", "fd"]
        .iter()
        .any(|p| name.starts_with(p))
}

/// Whole disks the installer may erase.
pub fn list_disks(roots: &Roots) -> Result<Vec<Disk>> {
    let running = running_disks(roots);
    let dir = roots.sysfs.join("block");
    let read = fs::read_dir(&dir).context(|| format!("cannot list {}", dir.display()))?;
    let mut names: Vec<String> = read
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut disks = Vec::new();
    for name in names {
        if skipped_by_name(&name) || running.contains(&name) {
            continue;
        }
        let base = dir.join(&name);
        let sectors: u64 = read_trim(&base.join("size"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if sectors == 0 || read_trim(&base.join("ro")).as_deref() == Some("1") {
            continue;
        }
        let vendor = read_trim(&base.join("device/vendor")).unwrap_or_default();
        let model = read_trim(&base.join("device/model")).unwrap_or_default();
        let model = format!("{vendor} {model}").trim().to_string();
        disks.push(Disk {
            path: roots.dev.join(&name),
            model: if model.is_empty() {
                "unknown model".into()
            } else {
                model
            },
            bytes: sectors.saturating_mul(512),
            name,
        });
    }
    Ok(disks)
}

fn normalize_disk_name(text: &str) -> String {
    let t = text.trim();
    t.strip_prefix("/dev/").unwrap_or(t).to_string()
}

fn print_disks(disks: &[Disk], out: &mut dyn Write) -> Result<()> {
    for d in disks {
        writeln!(
            out,
            "  {:<12} {:>9}  {}",
            d.name,
            human_size(d.bytes),
            d.model
        )
        .map_err(|e| Error::new(format!("cannot write output: {e}")))?;
    }
    Ok(())
}

fn say(out: &mut dyn Write, text: &str) -> Result<()> {
    writeln!(out, "{text}").map_err(|e| Error::new(format!("cannot write output: {e}")))
}

/// Asks which disk to erase. The user must type its name.
fn ask_disk(disks: &[Disk], input: &mut dyn BufRead, out: &mut dyn Write) -> Result<Disk> {
    say(out, "Disks that Silica can be installed on:")?;
    print_disks(disks, out)?;
    for _ in 0..3 {
        write!(
            out,
            "\nEverything on the disk you choose will be erased.\nType the name of the disk to confirm (empty to cancel): "
        )
        .and_then(|_| out.flush())
        .map_err(|e| Error::new(format!("cannot write output: {e}")))?;
        let mut line = String::new();
        let n = input
            .read_line(&mut line)
            .map_err(|e| Error::new(format!("cannot read input: {e}")))?;
        let name = normalize_disk_name(&line);
        if n == 0 || name.is_empty() {
            return Err(Error::new("installation cancelled; nothing was changed"));
        }
        if let Some(d) = disks.iter().find(|d| d.name == name) {
            return Ok(d.clone());
        }
        say(out, &format!("'{name}' is not one of the disks listed."))?;
    }
    Err(Error::new("too many wrong answers; nothing was changed"))
}

fn partition_device_name(roots: &Roots, disk: &str, number: u32) -> Option<String> {
    let dir = roots.sysfs.join("block").join(disk);
    for e in fs::read_dir(dir).ok()?.flatten() {
        if read_trim(&e.path().join("partition")).and_then(|s| s.parse::<u32>().ok())
            == Some(number)
        {
            return Some(e.file_name().to_string_lossy().into_owned());
        }
    }
    None
}

/// Waits until the new ESP is in the GPT, in sysfs and in `/dev`.
fn wait_for_esp(roots: &Roots, disk: &Disk, wait: Duration) -> Result<String> {
    let esp_type =
        parse_guid(ESP_TYPE).ok_or_else(|| Error::new("internal error: bad ESP GUID"))?;
    let deadline = Instant::now() + wait;
    loop {
        let found = (|| {
            let lbs = esp::logical_block_size(roots, &disk.name);
            let mut f = File::open(&disk.path).ok()?;
            let table = gpt::parse(&mut f, lbs).ok()?;
            let number = table
                .entries
                .iter()
                .find(|e| e.type_guid == esp_type)?
                .number;
            let name = partition_device_name(roots, &disk.name, number)?;
            roots.dev.join(&name).exists().then_some(name)
        })();
        if let Some(name) = found {
            return Ok(name);
        }
        if Instant::now() >= deadline {
            return Err(Error::new(format!(
                "no EFI System Partition appeared on {} after partitioning",
                disk.path.display()
            )));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// A mounted ESP that is unmounted and removed on drop.
struct Mounted<'a> {
    dir: PathBuf,
    tools: &'a Tools,
}

impl Drop for Mounted<'_> {
    fn drop(&mut self) {
        let _ = cmd::run(&self.tools.umount, &[&self.dir.to_string_lossy()]);
        let _ = fs::remove_dir(&self.dir);
    }
}

pub fn run(
    args: &[String],
    roots: &Roots,
    tools: &Tools,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> std::result::Result<(), Failure> {
    let parsed = args::parse(args, &["disk", "definitions"], &["yes", "help"])
        .map_err(|e| Failure::Usage(e.to_string()))?;
    if parsed.flag("help") {
        say(out, USAGE)?;
        return Ok(());
    }
    if let Some(extra) = parsed.positional.first() {
        return Err(Failure::Usage(format!("unexpected argument '{extra}'")));
    }
    if parsed.flag("yes") && parsed.value("disk").is_none() {
        return Err(Failure::Usage("--yes needs --disk".into()));
    }
    let definitions = parsed.value("definitions").unwrap_or(DEFAULT_DEFINITIONS);

    if !roots.efivars.is_dir() {
        return Err(Error::new(
            "this computer did not start in UEFI mode (no UEFI variables); restart the installer in UEFI mode",
        )
        .into());
    }
    let disks = list_disks(roots)?;
    if disks.is_empty() {
        return Err(Error::new("no disk found that Silica can be installed on").into());
    }

    let disk = match parsed.value("disk") {
        Some(given) => {
            let name = normalize_disk_name(given);
            let disk = disks.iter().find(|d| d.name == name).cloned().ok_or_else(|| {
                Error::new(format!(
                    "{given} is not a disk Silica can be installed on (it may hold the running system); candidates: {}",
                    disks.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", ")
                ))
            })?;
            if !parsed.flag("yes") {
                ask_disk(std::slice::from_ref(&disk), input, out)?;
            }
            disk
        }
        None => ask_disk(&disks, input, out)?,
    };

    say(
        out,
        &format!(
            "\nInstalling Silica on {} ({}, {})...",
            disk.path.display(),
            human_size(disk.bytes),
            disk.model
        ),
    )?;
    say(out, "Partitioning and copying the system (systemd-repart)")?;
    let path = disk.path.to_string_lossy().into_owned();
    cmd::run(
        &tools.repart,
        &[
            "--dry-run=no",
            "--empty=force",
            &format!("--definitions={definitions}"),
            &path,
        ],
    )
    .map_err(|e| Error::new(format!("partitioning {path} failed: {e}")))?;

    // Let udev create the device nodes; a missing udevadm is not fatal.
    let _ = cmd::run(&tools.udevadm, &["settle", "--timeout=10"]);
    let part = wait_for_esp(roots, &disk, tools.wait)?;
    say(out, &format!("Writing the boot entries (ESP is {part})"))?;

    let mount_dir = roots
        .path("/run")
        .join(format!("silica-install-{}", std::process::id()));
    fs::create_dir_all(&mount_dir)
        .map_err(|e| Error::new(format!("cannot create {}: {e}", mount_dir.display())))?;
    let dev = roots.dev.join(&part);
    let mount_arg = mount_dir.to_string_lossy().into_owned();
    let mounted = Mounted {
        dir: mount_dir.clone(),
        tools,
    };
    cmd::run(
        &tools.mount,
        &["-t", "vfat", &dev.to_string_lossy(), &mount_arg],
    )
    .map_err(|e| Error::new(format!("cannot mount the new ESP: {e}")))?;

    let info = esp::from_device_name(roots, &part)?;
    let ukis = scan_ukis(&mount_dir)?;
    let newest = ukis.last().map(|(v, _)| v.clone()).ok_or_else(|| {
        Error::new("the new ESP holds no EFI/Linux/silica_*.efi; the repart definitions are wrong")
    })?;
    let mgr = BootManager::new(EfiVars::new(&roots.efivars));
    // The new system is not "running", so make the newest UKI the reference:
    // no BootNext, and the recovery entry points at it.
    let report = mgr.sync(
        &info,
        &mount_dir,
        &SyncOptions {
            booted: Some(newest.clone()),
            booted_fallback: None,
        },
    )?;
    for m in &report.messages {
        say(out, &format!("  {m}"))?;
    }
    let entry = mgr
        .valid_entries()?
        .into_iter()
        .find(|e| {
            kind(&e.option) != Kind::Recovery
                && kind(&e.option) != Kind::Other
                && e.version().as_deref() == Some(newest.as_str())
                && e.option
                    .hard_drive()
                    .is_some_and(|h| h.signature == info.guid)
        })
        .ok_or_else(|| Error::new("the boot entry for the new system was not created"))?;
    mgr.move_to_front(entry.number)?;
    drop(mounted);

    say(
        out,
        &format!("\nSilica {newest} is installed on {}.", disk.path.display()),
    )?;
    say(
        out,
        "Remove the installation medium and restart the computer.",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use silica_efi::fixture::{add_disk, fake_system, tmpdir};
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &Path, name: &str, body: &str) -> String {
        let p = dir.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p.to_string_lossy().into_owned()
    }

    /// Running system on vda; a USB-like vdb, a loop device and an empty one.
    fn system(tag: &str) -> Roots {
        let roots = fake_system(tag, 512);
        // the target disk already carries the table repart would write
        add_disk(
            &roots,
            "vdb",
            (254, 16),
            512,
            "Target SSD",
            Some([0x77; 16]),
        );
        add_disk(&roots, "loop0", (7, 0), 512, "", None);
        add_disk(&roots, "sr0", (11, 0), 512, "", None);
        add_disk(&roots, "vdc", (254, 32), 512, "", None);
        fs::write(roots.sysfs.join("block/vdc/size"), "0\n").unwrap();
        fs::create_dir_all(&roots.efivars).unwrap();
        roots
    }

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1_500_000_000), "1.5 GB");
        assert_eq!(human_size(512_110_190_592), "512.1 GB");
    }

    #[test]
    fn lists_only_eligible_disks() {
        let roots = system("inst-list");
        let disks = list_disks(&roots).unwrap();
        assert_eq!(disks.len(), 1, "{disks:?}");
        assert_eq!(disks[0].name, "vdb");
        assert_eq!(disks[0].model, "Target SSD");
        assert_eq!(disks[0].bytes, 209_715_200 * 512);
        assert_eq!(
            running_disks(&roots).into_iter().collect::<Vec<_>>(),
            vec!["vda".to_string()]
        );
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn running_disk_found_through_device_mapper() {
        let roots = system("inst-dm");
        // /usr is dm-0 on top of vdb1
        let dm = roots.sysfs.join("devices/virtual/block/dm-0");
        fs::create_dir_all(dm.join("slaves")).unwrap();
        fs::write(dm.join("dev"), "253:0\n").unwrap();
        std::os::unix::fs::symlink(&dm, roots.sysfs.join("dev/block/253:0")).unwrap();
        std::os::unix::fs::symlink(&dm, roots.sysfs.join("block/dm-0")).unwrap();
        let slave = roots.sysfs.join("devices/pci/block/vdb/vdb1");
        std::os::unix::fs::symlink(&slave, dm.join("slaves/vdb1")).unwrap();
        fs::write(
            &roots.mountinfo,
            "1 1 253:0 / /usr ro - erofs /dev/mapper/usr ro\n",
        )
        .unwrap();
        assert_eq!(
            running_disks(&roots).into_iter().collect::<Vec<_>>(),
            vec!["vdb".to_string()]
        );
        let disks = list_disks(&roots).unwrap();
        assert_eq!(
            disks.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            vec!["vda"]
        );
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn asks_for_the_name() {
        let roots = system("inst-ask");
        let disks = list_disks(&roots).unwrap();
        let mut out = Vec::new();
        let d = ask_disk(&disks, &mut Cursor::new("sdz\n/dev/vdb\n"), &mut out).unwrap();
        assert_eq!(d.name, "vdb");
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Target SSD") && text.contains("not one of the disks"));
        let e = ask_disk(&disks, &mut Cursor::new("\n"), &mut Vec::new()).unwrap_err();
        assert!(e.to_string().contains("cancelled"));
        let e = ask_disk(&disks, &mut Cursor::new(""), &mut Vec::new()).unwrap_err();
        assert!(e.to_string().contains("cancelled"));
        let e = ask_disk(&disks, &mut Cursor::new("a\nb\nc\nvdb\n"), &mut Vec::new()).unwrap_err();
        assert!(e.to_string().contains("wrong answers"));
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn argument_errors() {
        let roots = system("inst-args");
        let tools = Tools::default();
        let go = |a: &[&str], input: &str| -> String {
            let mut out = Vec::new();
            match run(
                &args(a),
                &roots,
                &tools,
                &mut Cursor::new(input.to_string()),
                &mut out,
            ) {
                Ok(()) => "ok".into(),
                Err(Failure::Usage(m)) => format!("usage: {m}"),
                Err(Failure::Error(e)) => e.to_string(),
            }
        };
        assert!(go(&["--yes"], "").starts_with("usage"));
        assert!(go(&["x"], "").starts_with("usage"));
        assert!(go(&["--disk", "/dev/vda", "--yes"], "")
            .contains("not a disk Silica can be installed on"));
        assert!(go(&["--disk", "/dev/vdb"], "\n").contains("cancelled"));
        let _ = fs::remove_dir_all(&roots.efivars);
        assert!(go(&["--disk", "/dev/vdb", "--yes"], "").contains("UEFI mode"));
        let _ = fs::remove_dir_all(&roots.root);
    }

    #[test]
    fn failing_repart_is_explained() {
        let roots = system("inst-fail");
        let bin = tmpdir("inst-fail-bin");
        let tools = Tools {
            repart: script(&bin, "repart", "echo boom >&2; exit 3"),
            ..Tools::default()
        };
        let mut out = Vec::new();
        let r = run(
            &args(&["--disk", "vdb", "--yes"]),
            &roots,
            &tools,
            &mut Cursor::new(""),
            &mut out,
        );
        match r {
            Err(Failure::Error(e)) => {
                assert!(
                    e.to_string().contains("partitioning") && e.to_string().contains("exit code 3"),
                    "{e}"
                )
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&roots.root);
        let _ = fs::remove_dir_all(bin);
    }

    #[test]
    fn end_to_end_with_fake_tools() {
        let roots = system("inst-e2e");
        let bin = tmpdir("inst-e2e-bin");
        let payload = bin.join("payload");
        fs::create_dir_all(payload.join("EFI/Linux")).unwrap();
        fs::write(payload.join("EFI/Linux/silica_1.0.0.efi"), b"MZ").unwrap();
        fs::write(payload.join("EFI/Linux/silica_1.2.0.efi"), b"MZ").unwrap();
        let log = bin.join("log");
        let tools = Tools {
            repart: script(
                &bin,
                "repart",
                &format!("echo repart \"$@\" >> {}", log.display()),
            ),
            mount: script(
                &bin,
                "mount",
                &format!(
                    "echo mount \"$@\" >> {l}; cp -r {p}/. \"$4\"",
                    l = log.display(),
                    p = payload.display()
                ),
            ),
            umount: script(
                &bin,
                "umount",
                &format!("echo umount \"$@\" >> {}; rm -rf \"$1\"/*", log.display()),
            ),
            udevadm: "/nonexistent/udevadm".into(),
            wait: Duration::from_secs(2),
        };
        // an existing foreign entry that must be kept
        let mgr = BootManager::new(EfiVars::new(&roots.efivars));
        let info = esp::from_device_name(&roots, "vda1").unwrap();
        let (w, _) = mgr
            .add(
                "Other OS",
                &info,
                "\\EFI\\other.efi",
                None,
                silica_efi::manager::Position::Append,
            )
            .unwrap();

        let mut out = Vec::new();
        run(
            &args(&["--disk", "/dev/vdb", "--yes"]),
            &roots,
            &tools,
            &mut Cursor::new(""),
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Silica 1.2.0 is installed"), "{text}");

        let logged = fs::read_to_string(&log).unwrap();
        assert!(
            logged.contains(&format!(
                "repart --dry-run=no --empty=force --definitions={DEFAULT_DEFINITIONS} {}",
                roots.dev.join("vdb").display()
            )),
            "{logged}"
        );
        assert!(
            logged.contains("mount -t vfat") && logged.contains("umount"),
            "{logged}"
        );
        assert!(!roots
            .root
            .join(format!("run/silica-install-{}", std::process::id()))
            .exists());

        let labels: Vec<_> = mgr
            .valid_entries()
            .unwrap()
            .into_iter()
            .map(|e| {
                (
                    e.number,
                    e.option.description.clone(),
                    e.option.hard_drive().map(|h| h.signature),
                )
            })
            .collect();
        let order = mgr.vars().boot_order().unwrap();
        let new = labels.iter().find(|(_, l, _)| l == "Silica 1.2.0").unwrap();
        assert_eq!(new.2, Some([0x77; 16]));
        assert_eq!(order[0], new.0, "newest version first");
        assert!(order.contains(&w), "foreign entry is kept");
        assert!(labels.iter().any(|(_, l, _)| l == "Silica 1.0.0"));
        assert!(labels.iter().any(|(_, l, _)| l == "Silica Recovery"));
        assert_eq!(mgr.vars().boot_next().unwrap(), None);
        let rec = mgr
            .valid_entries()
            .unwrap()
            .into_iter()
            .find(|e| e.option.description == "Silica Recovery")
            .unwrap();
        assert_eq!(
            rec.option.file_path(),
            Some("\\EFI\\Linux\\silica_1.2.0.efi")
        );
        let _ = fs::remove_dir_all(&roots.root);
        let _ = fs::remove_dir_all(bin);
    }
}
