//! Silica's boot entries: one `Silica <version>` per installed UKI and one
//! `Silica Recovery`, kept in step with the ESP.

use crate::efivars::{boot_var_name, EfiVars, ATTR_NV_BS_RT, GLOBAL_GUID};
use crate::error::{Context, Error, Result};
use crate::esp::EspInfo;
use crate::loadopt::{utf16_bytes, LoadOption};
use crate::version;
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

pub const RECOVERY_LABEL: &str = "Silica Recovery";
pub const LABEL_PREFIX: &str = "Silica ";
/// Where UKIs live on the ESP, as a UEFI path.
pub const UKI_DIR: &str = "\\EFI\\Linux";
/// Makes systemd-stub select UKI profile 1, the recovery profile.
pub const RECOVERY_OPTIONS: &str = "@1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Version(String),
    Recovery,
    Other,
}

pub fn kind(option: &LoadOption) -> Kind {
    let d = option.description.as_str();
    if d == RECOVERY_LABEL {
        Kind::Recovery
    } else if let Some(v) = d.strip_prefix(LABEL_PREFIX) {
        if v.is_empty() {
            Kind::Other
        } else {
            Kind::Version(v.to_string())
        }
    } else {
        Kind::Other
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub number: u16,
    pub option: LoadOption,
}

impl Entry {
    /// The version this entry boots: from its UKI file name, else its label.
    pub fn version(&self) -> Option<String> {
        if let Some(v) =
            file_name_of(self.option.file_path()).and_then(|f| version::uki_version(&f))
        {
            return Some(v);
        }
        match kind(&self.option) {
            Kind::Version(v) => Some(v),
            _ => None,
        }
    }
}

fn file_name_of(path: Option<&str>) -> Option<String> {
    path.and_then(|p| p.rsplit('\\').next()).map(str::to_string)
}

/// Where a new entry goes in `BootOrder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Append,
    BeforeRecovery,
}

#[derive(Debug, Default, Clone)]
pub struct SyncOptions {
    /// Version that is running, when the caller knows it for sure.
    pub booted: Option<String>,
    /// Used when `BootCurrent` does not identify the running version.
    pub booted_fallback: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bless {
    Blessed(u16),
    AlreadyFirst(u16),
    /// `BootCurrent` is not a Silica entry (or is the recovery one).
    NotApplicable(String),
}

pub struct BootManager {
    vars: EfiVars,
}

fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// True when `uefi_path` (backslashes) names a file below `esp_dir`.
fn esp_file_exists(esp_dir: &Path, uefi_path: &str) -> bool {
    let mut p = esp_dir.to_path_buf();
    for part in uefi_path.split('\\').filter(|s| !s.is_empty()) {
        if part == ".." {
            return false;
        }
        p.push(part);
    }
    p.is_file()
}

impl BootManager {
    pub fn new(vars: EfiVars) -> Self {
        BootManager { vars }
    }

    pub fn vars(&self) -> &EfiVars {
        &self.vars
    }

    pub fn read_entry(&self, number: u16) -> Result<Option<LoadOption>> {
        match self.vars.get_global(&boot_var_name(number))? {
            None => Ok(None),
            Some(d) => LoadOption::decode(&d).map(Some),
        }
    }

    /// All `Boot####` variables, with their decoded form or why decoding failed.
    pub fn entries(&self) -> Result<Vec<(u16, Result<LoadOption>)>> {
        let mut out = Vec::new();
        for n in self.vars.boot_numbers()? {
            let r = match self.read_entry(n) {
                Ok(Some(o)) => Ok(o),
                Ok(None) => Err(Error::new("vanished while reading")),
                Err(e) => Err(e),
            };
            out.push((n, r));
        }
        Ok(out)
    }

    /// Entries that decode; broken ones are left alone.
    pub fn valid_entries(&self) -> Result<Vec<Entry>> {
        Ok(self
            .entries()?
            .into_iter()
            .filter_map(|(number, r)| r.ok().map(|option| Entry { number, option }))
            .collect())
    }

    fn free_number(&self) -> Result<u16> {
        let used = self.vars.boot_numbers()?;
        (0..=u16::MAX)
            .find(|n| !used.contains(n))
            .ok_or_else(|| Error::new("all 65536 boot entry numbers are in use"))
    }

    pub fn write_entry(&self, number: u16, option: &LoadOption) -> Result<()> {
        self.vars
            .set_global(&boot_var_name(number), &option.encode()?)
    }

    fn insert_in_order(&self, number: u16, position: Position) -> Result<()> {
        let mut order = self.vars.boot_order()?;
        if order.contains(&number) {
            return Ok(());
        }
        let recovery = if position == Position::BeforeRecovery {
            self.valid_entries()?
                .into_iter()
                .find(|e| kind(&e.option) == Kind::Recovery)
                .and_then(|e| order.iter().position(|n| *n == e.number))
        } else {
            None
        };
        match recovery {
            Some(i) => order.insert(i, number),
            None => order.push(number),
        }
        self.vars.set_boot_order(&order)
    }

    /// Creates the entry, or reuses one with the same label, file and data.
    /// Returns the number and whether it was created.
    pub fn add(
        &self,
        label: &str,
        esp: &EspInfo,
        uefi_path: &str,
        options: Option<&str>,
        position: Position,
    ) -> Result<(u16, bool)> {
        let data = options.map(utf16_bytes).unwrap_or_default();
        let wanted = LoadOption::new(label, Some(esp.hard_drive()), uefi_path, &data);
        let existing = self.valid_entries()?.into_iter().find(|e| {
            e.option.description == label
                && e.option.optional_data == data
                && e.option
                    .file_path()
                    .is_some_and(|p| same_path(p, uefi_path))
        });
        if let Some(e) = existing {
            if e.option != wanted {
                self.write_entry(e.number, &wanted)?;
            }
            self.insert_in_order(e.number, position)?;
            return Ok((e.number, false));
        }
        let n = self.free_number()?;
        self.write_entry(n, &wanted)?;
        self.insert_in_order(n, position)?;
        Ok((n, true))
    }

    /// Deletes an entry and every reference to it.
    pub fn remove_entry(&self, number: u16) -> Result<()> {
        let mut order = self.vars.boot_order()?;
        if order.contains(&number) {
            order.retain(|n| *n != number);
            self.vars.set_boot_order(&order)?;
        }
        if self.vars.boot_next()? == Some(number) {
            self.vars.clear_boot_next()?;
        }
        self.vars.remove(&boot_var_name(number), GLOBAL_GUID)
    }

    pub fn move_to_front(&self, number: u16) -> Result<()> {
        let mut order = self.vars.boot_order()?;
        order.retain(|n| *n != number);
        order.insert(0, number);
        self.vars.set_boot_order(&order)
    }

    /// The version of the Silica entry firmware booted from, if any.
    pub fn booted_version(&self) -> Result<Option<String>> {
        let Some(current) = self.vars.boot_current()? else {
            return Ok(None);
        };
        let Ok(Some(option)) = self.read_entry(current) else {
            return Ok(None);
        };
        if kind(&option) == Kind::Other {
            return Ok(None);
        }
        Ok(Entry {
            number: current,
            option,
        }
        .version())
    }

    pub fn bless(&self) -> Result<Bless> {
        let Some(current) = self.vars.boot_current()? else {
            return Ok(Bless::NotApplicable("BootCurrent is not set".into()));
        };
        let option = match self.read_entry(current) {
            Ok(Some(o)) => o,
            _ => {
                return Ok(Bless::NotApplicable(format!(
                    "{} cannot be read",
                    boot_var_name(current)
                )))
            }
        };
        match kind(&option) {
            Kind::Other => Ok(Bless::NotApplicable(format!(
                "{} ({}) is not a Silica entry",
                boot_var_name(current),
                option.description
            ))),
            Kind::Recovery => Ok(Bless::NotApplicable(
                "booted into Silica Recovery, nothing to bless".into(),
            )),
            Kind::Version(_) => {
                if self.vars.boot_order()?.first() == Some(&current) {
                    Ok(Bless::AlreadyFirst(current))
                } else {
                    self.move_to_front(current)?;
                    Ok(Bless::Blessed(current))
                }
            }
        }
    }

    /// The other Silica version to boot: the newest one older than the
    /// current, else the newest other one.
    pub fn rollback_target(&self) -> Result<u16> {
        let mut versions: Vec<(String, u16)> = self
            .valid_entries()?
            .into_iter()
            .filter(|e| kind(&e.option) != Kind::Other)
            .filter(|e| kind(&e.option) != Kind::Recovery)
            .filter_map(|e| e.version().map(|v| (v, e.number)))
            .collect();
        versions.sort_by(|a, b| version::compare(&a.0, &b.0));
        let current = match self.booted_version()? {
            Some(v) => v,
            None => versions
                .last()
                .map(|(v, _)| v.clone())
                .ok_or_else(|| Error::new("no Silica version is installed"))?,
        };
        let older = versions
            .iter()
            .rev()
            .find(|(v, _)| version::compare(v, &current) == Ordering::Less);
        let other = versions
            .iter()
            .rev()
            .find(|(v, _)| version::compare(v, &current) != Ordering::Equal);
        older
            .or(other)
            .map(|(_, n)| *n)
            .ok_or_else(|| Error::new("there is no other Silica version to roll back to"))
    }

    /// Reconciles the boot entries with `EFI/Linux/silica_*.efi` on the ESP.
    pub fn sync(&self, esp: &EspInfo, esp_dir: &Path, opts: &SyncOptions) -> Result<SyncReport> {
        let mut report = SyncReport::default();
        if !esp_dir.join("EFI").is_dir() {
            return Err(Error::new(format!(
                "{}/EFI does not exist; is the ESP mounted there?",
                esp_dir.display()
            )));
        }
        let ukis = scan_ukis(esp_dir)?;

        let booted = match &opts.booted {
            Some(v) => Some(v.clone()),
            None => self
                .booted_version()?
                .or_else(|| opts.booted_fallback.clone()),
        };

        // Entries whose file is gone.
        for e in self.valid_entries()? {
            if kind(&e.option) == Kind::Other {
                continue;
            }
            // Entries of another ESP (a second install) are not ours to judge.
            if e.option
                .hard_drive()
                .is_some_and(|h| h.signature != esp.guid)
            {
                continue;
            }
            let present = e
                .option
                .file_path()
                .is_some_and(|p| esp_file_exists(esp_dir, p));
            if !present {
                self.remove_entry(e.number)?;
                report.messages.push(format!(
                    "removed {} ({}): its file is gone",
                    boot_var_name(e.number),
                    e.option.description
                ));
            }
        }

        // The recovery entry first, so that new versions land before it.
        let newest = ukis.last().map(|(v, _)| v.clone());
        let recovery_version = booted
            .as_ref()
            .filter(|b| ukis.iter().any(|(v, _)| v == *b))
            .cloned()
            .or_else(|| newest.clone());
        if let Some(rv) = &recovery_version {
            if let Some((_, file)) = ukis.iter().find(|(v, _)| v == rv) {
                let path = format!("{UKI_DIR}\\{file}");
                self.upsert_recovery(esp, &path, &mut report)?;
            }
        }

        // One entry per UKI.
        let mut numbers: Vec<(String, u16)> = Vec::new();
        for (v, file) in &ukis {
            let path = format!("{UKI_DIR}\\{file}");
            let label = format!("{LABEL_PREFIX}{v}");
            let (n, created) = self.add(&label, esp, &path, None, Position::BeforeRecovery)?;
            if created {
                report
                    .messages
                    .push(format!("created {} ({label})", boot_var_name(n)));
            }
            numbers.push((v.clone(), n));
        }

        // A newer version than the running one boots next, once.
        if let (Some(b), Some(newest)) = (&booted, &newest) {
            if version::compare(newest, b) == Ordering::Greater {
                if let Some((_, n)) = numbers.iter().find(|(v, _)| v == newest) {
                    self.vars.set_boot_next(*n)?;
                    report.messages.push(format!(
                        "BootNext set to {} (Silica {newest}, running {b})",
                        boot_var_name(*n)
                    ));
                }
            }
        }
        if report.messages.is_empty() {
            report.messages.push("boot entries are up to date".into());
        }
        Ok(report)
    }

    fn upsert_recovery(&self, esp: &EspInfo, path: &str, report: &mut SyncReport) -> Result<()> {
        let wanted = LoadOption::new(
            RECOVERY_LABEL,
            Some(esp.hard_drive()),
            path,
            &utf16_bytes(RECOVERY_OPTIONS),
        );
        let existing = self
            .valid_entries()?
            .into_iter()
            .find(|e| kind(&e.option) == Kind::Recovery);
        match existing {
            Some(e) => {
                if e.option != wanted {
                    self.write_entry(e.number, &wanted)?;
                    report.messages.push(format!(
                        "{} ({RECOVERY_LABEL}) now boots {path}",
                        boot_var_name(e.number)
                    ));
                }
                self.insert_in_order(e.number, Position::Append)
            }
            None => {
                let (n, _) = self.add(
                    RECOVERY_LABEL,
                    esp,
                    path,
                    Some(RECOVERY_OPTIONS),
                    Position::Append,
                )?;
                report
                    .messages
                    .push(format!("created {} ({RECOVERY_LABEL})", boot_var_name(n)));
                Ok(())
            }
        }
    }
}

/// `(version, file name)` of every `silica_*.efi` in `EFI/Linux`, oldest first.
pub fn scan_ukis(esp_dir: &Path) -> Result<Vec<(String, String)>> {
    let dir = esp_dir.join("EFI/Linux");
    let mut out = Vec::new();
    let read = match fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(Error::new(format!("cannot list {}: {e}", dir.display()))),
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(v) = version::uki_version(&name) {
            if entry.path().is_file() {
                out.push((v, name));
            }
        }
    }
    out.sort_by(|a, b| version::compare(&a.0, &b.0));
    Ok(out)
}

/// Writes a variable that is not a boot variable, with the standard attributes.
pub fn set_vendor_var(vars: &EfiVars, name: &str, guid: &str, data: &[u8]) -> Result<()> {
    vars.set(name, guid, ATTR_NV_BS_RT, data)
        .context(|| format!("cannot set {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::tmpdir;
    use std::path::PathBuf;

    struct Env {
        dir: PathBuf,
        esp_dir: PathBuf,
        mgr: BootManager,
        esp: EspInfo,
    }

    fn env(tag: &str) -> Env {
        let dir = tmpdir(tag);
        let vars_dir = dir.join("efivars");
        let esp_dir = dir.join("efi");
        fs::create_dir_all(&vars_dir).unwrap();
        fs::create_dir_all(esp_dir.join("EFI/Linux")).unwrap();
        Env {
            esp_dir,
            dir,
            mgr: BootManager::new(EfiVars::new(vars_dir)),
            esp: EspInfo {
                disk: "vda".into(),
                partition_number: 1,
                start: 2048,
                size: 1000,
                guid: [9; 16],
            },
        }
    }

    impl Env {
        fn uki(&self, v: &str) {
            fs::write(
                self.esp_dir.join(format!("EFI/Linux/silica_{v}.efi")),
                b"MZ",
            )
            .unwrap();
        }
        fn rm_uki(&self, v: &str) {
            fs::remove_file(self.esp_dir.join(format!("EFI/Linux/silica_{v}.efi"))).unwrap();
        }
        fn set_current(&self, n: u16) {
            self.mgr
                .vars()
                .set("BootCurrent", GLOBAL_GUID, 6, &n.to_le_bytes())
                .unwrap();
        }
        fn labels(&self) -> Vec<(u16, String)> {
            self.mgr
                .valid_entries()
                .unwrap()
                .into_iter()
                .map(|e| (e.number, e.option.description))
                .collect()
        }
        fn number_of(&self, label: &str) -> u16 {
            self.labels()
                .into_iter()
                .find(|(_, l)| l == label)
                .unwrap()
                .0
        }
        fn sync(&self) -> SyncReport {
            self.mgr
                .sync(&self.esp, &self.esp_dir, &SyncOptions::default())
                .unwrap()
        }
        fn cleanup(self) {
            let _ = fs::remove_dir_all(self.dir);
        }
    }

    #[test]
    fn add_creates_reuses_and_orders() {
        let e = env("mgr-add");
        let p = "\\EFI\\Linux\\silica_1.0.efi";
        let (a, created) = e
            .mgr
            .add("Silica 1.0", &e.esp, p, None, Position::Append)
            .unwrap();
        assert!(created);
        assert_eq!(a, 0);
        let (b, created) = e
            .mgr
            .add(
                "Silica 1.0",
                &e.esp,
                &p.to_lowercase(),
                None,
                Position::Append,
            )
            .unwrap();
        assert!(!created);
        assert_eq!(a, b);
        let (r, _) = e
            .mgr
            .add("Silica Recovery", &e.esp, p, Some("@1"), Position::Append)
            .unwrap();
        assert_eq!(r, 1);
        assert_eq!(e.mgr.vars().boot_order().unwrap(), vec![0, 1]);
        let opt = e.mgr.read_entry(r).unwrap().unwrap();
        assert_eq!(opt.optional_data, vec![b'@', 0, b'1', 0]);
        assert_eq!(opt.hard_drive().unwrap().signature, [9; 16]);
        e.cleanup();
    }

    #[test]
    fn first_free_number_skips_used() {
        let e = env("mgr-free");
        e.mgr.vars().set_global("Boot0000", b"junk").unwrap();
        e.mgr.vars().set_global("Boot0001", b"junk").unwrap();
        let (n, _) = e
            .mgr
            .add("x", &e.esp, "\\a", None, Position::Append)
            .unwrap();
        assert_eq!(n, 2);
        // undecodable entries are ignored, not fatal
        assert_eq!(e.mgr.valid_entries().unwrap().len(), 1);
        e.cleanup();
    }

    #[test]
    fn remove_cleans_order_and_next() {
        let e = env("mgr-rm");
        let (a, _) = e
            .mgr
            .add("a", &e.esp, "\\a", None, Position::Append)
            .unwrap();
        let (b, _) = e
            .mgr
            .add("b", &e.esp, "\\b", None, Position::Append)
            .unwrap();
        e.mgr.vars().set_boot_next(a).unwrap();
        e.mgr.remove_entry(a).unwrap();
        assert_eq!(e.mgr.vars().boot_order().unwrap(), vec![b]);
        assert_eq!(e.mgr.vars().boot_next().unwrap(), None);
        assert!(e.mgr.read_entry(a).unwrap().is_none());
        e.cleanup();
    }

    #[test]
    fn first_sync_creates_everything() {
        let e = env("mgr-sync1");
        e.uki("1.0.0");
        e.uki("1.1.0");
        let r = e.sync();
        assert!(r.messages.iter().any(|m| m.contains("Silica 1.1.0")));
        let labels: Vec<String> = e.labels().into_iter().map(|(_, l)| l).collect();
        assert!(labels.contains(&"Silica 1.0.0".to_string()));
        assert!(labels.contains(&"Silica 1.1.0".to_string()));
        assert!(labels.contains(&RECOVERY_LABEL.to_string()));
        // versions come before recovery in BootOrder
        let order = e.mgr.vars().boot_order().unwrap();
        assert_eq!(*order.last().unwrap(), e.number_of(RECOVERY_LABEL));
        // nothing is booted, so the newest is assumed: recovery uses it, no BootNext
        let rec = e
            .mgr
            .read_entry(e.number_of(RECOVERY_LABEL))
            .unwrap()
            .unwrap();
        assert_eq!(rec.file_path(), Some("\\EFI\\Linux\\silica_1.1.0.efi"));
        assert_eq!(e.mgr.vars().boot_next().unwrap(), None);
        // idempotent
        let r = e.sync();
        assert_eq!(r.messages, vec!["boot entries are up to date".to_string()]);
        assert_eq!(e.mgr.vars().boot_order().unwrap(), order);
        e.cleanup();
    }

    #[test]
    fn update_flow_sets_next_then_bless_and_rollback() {
        let e = env("mgr-flow");
        e.uki("1.0.0");
        e.sync();
        let old = e.number_of("Silica 1.0.0");
        e.mgr.move_to_front(old).unwrap();
        e.set_current(old);
        // sysupdate drops a new UKI
        e.uki("1.1.0");
        e.sync();
        let new = e.number_of("Silica 1.1.0");
        assert_eq!(e.mgr.vars().boot_next().unwrap(), Some(new));
        let order = e.mgr.vars().boot_order().unwrap();
        assert_eq!(order[0], old, "old stays first until blessed");
        let rec = e
            .mgr
            .read_entry(e.number_of(RECOVERY_LABEL))
            .unwrap()
            .unwrap();
        assert_eq!(rec.file_path(), Some("\\EFI\\Linux\\silica_1.0.0.efi"));

        // reboot: firmware consumed BootNext and booted the new entry
        e.mgr.vars().clear_boot_next().unwrap();
        e.set_current(new);
        assert_eq!(e.mgr.bless().unwrap(), Bless::Blessed(new));
        assert_eq!(e.mgr.vars().boot_order().unwrap()[0], new);
        assert_eq!(e.mgr.bless().unwrap(), Bless::AlreadyFirst(new));
        // sync now points recovery at the new version and sets nothing
        e.sync();
        assert_eq!(e.mgr.vars().boot_next().unwrap(), None);
        let rec = e
            .mgr
            .read_entry(e.number_of(RECOVERY_LABEL))
            .unwrap()
            .unwrap();
        assert_eq!(rec.file_path(), Some("\\EFI\\Linux\\silica_1.1.0.efi"));
        assert_eq!(e.mgr.rollback_target().unwrap(), old);
        e.cleanup();
    }

    #[test]
    fn sync_removes_dangling_entries() {
        let e = env("mgr-dangling");
        e.uki("1.0.0");
        e.uki("1.1.0");
        e.sync();
        let n = e.number_of("Silica 1.0.0");
        e.mgr.vars().set_boot_next(n).unwrap();
        e.rm_uki("1.0.0");
        let r = e.sync();
        assert!(r.messages.iter().any(|m| m.starts_with("removed")));
        assert!(!e.labels().iter().any(|(_, l)| l == "Silica 1.0.0"));
        assert!(!e.mgr.vars().boot_order().unwrap().contains(&n));
        assert_eq!(e.mgr.vars().boot_next().unwrap(), None);
        // a foreign entry is untouched
        e.mgr
            .add(
                "Windows",
                &e.esp,
                "\\EFI\\Microsoft\\bootmgfw.efi",
                None,
                Position::Append,
            )
            .unwrap();
        e.sync();
        assert!(e.labels().iter().any(|(_, l)| l == "Windows"));
        e.cleanup();
    }

    #[test]
    fn sync_leaves_entries_of_other_disks() {
        let e = env("mgr-other");
        e.uki("1.0.0");
        let other = EspInfo {
            guid: [5; 16],
            ..e.esp.clone()
        };
        let (n, _) = e
            .mgr
            .add(
                "Silica 0.9.0",
                &other,
                "\\EFI\\Linux\\silica_0.9.0.efi",
                None,
                Position::Append,
            )
            .unwrap();
        e.sync();
        assert!(e.labels().iter().any(|(m, _)| *m == n));
        e.cleanup();
    }

    #[test]
    fn sync_refuses_unmounted_esp() {
        let e = env("mgr-nomount");
        fs::remove_dir_all(e.esp_dir.join("EFI")).unwrap();
        let err = e
            .mgr
            .sync(&e.esp, &e.esp_dir, &SyncOptions::default())
            .unwrap_err();
        assert!(err.to_string().contains("is the ESP mounted"));
        e.cleanup();
    }

    #[test]
    fn booted_override_and_fallback() {
        let e = env("mgr-booted");
        e.uki("1.0.0");
        e.uki("2.0.0");
        let o = SyncOptions {
            booted: None,
            booted_fallback: Some("1.0.0".into()),
        };
        e.mgr.sync(&e.esp, &e.esp_dir, &o).unwrap();
        let new = e.number_of("Silica 2.0.0");
        assert_eq!(e.mgr.vars().boot_next().unwrap(), Some(new));
        let o = SyncOptions {
            booted: Some("2.0.0".into()),
            booted_fallback: None,
        };
        e.mgr.vars().clear_boot_next().unwrap();
        e.mgr.sync(&e.esp, &e.esp_dir, &o).unwrap();
        assert_eq!(e.mgr.vars().boot_next().unwrap(), None);
        e.cleanup();
    }

    #[test]
    fn bless_ignores_foreign_and_recovery() {
        let e = env("mgr-bless");
        e.uki("1.0.0");
        e.sync();
        assert!(matches!(e.mgr.bless().unwrap(), Bless::NotApplicable(_)));
        let (w, _) = e
            .mgr
            .add("Windows", &e.esp, "\\w.efi", None, Position::Append)
            .unwrap();
        e.set_current(w);
        assert!(matches!(e.mgr.bless().unwrap(), Bless::NotApplicable(_)));
        e.set_current(e.number_of(RECOVERY_LABEL));
        assert!(matches!(e.mgr.bless().unwrap(), Bless::NotApplicable(_)));
        e.set_current(99);
        assert!(matches!(e.mgr.bless().unwrap(), Bless::NotApplicable(_)));
        e.cleanup();
    }

    #[test]
    fn rollback_needs_two_versions() {
        let e = env("mgr-rb");
        e.uki("1.0.0");
        e.sync();
        assert!(e.mgr.rollback_target().is_err());
        e.uki("1.2.0");
        e.uki("1.1.0");
        e.sync();
        // no BootCurrent: current is the newest, rollback is the one below
        assert_eq!(
            e.mgr.rollback_target().unwrap(),
            e.number_of("Silica 1.1.0")
        );
        // running the oldest: roll "back" to the newest other one
        e.set_current(e.number_of("Silica 1.0.0"));
        assert_eq!(
            e.mgr.rollback_target().unwrap(),
            e.number_of("Silica 1.2.0")
        );
        e.cleanup();
    }

    #[test]
    fn kinds() {
        let o = |d: &str| LoadOption::new(d, None, "\\a", &[]);
        assert_eq!(kind(&o("Silica 1.2")), Kind::Version("1.2".into()));
        assert_eq!(kind(&o("Silica Recovery")), Kind::Recovery);
        assert_eq!(kind(&o("Silica ")), Kind::Other);
        assert_eq!(kind(&o("Windows")), Kind::Other);
    }
}
