//! Access to UEFI variables through efivarfs (or a directory that looks like it).
//!
//! Every variable is a file `<Name>-<GUID>` whose content is a 4-byte
//! little-endian attribute word followed by the data.

use crate::error::{Context, Error, Result};
use crate::loadopt::le_u16s;
use std::ffi::{c_int, c_long, c_ulong};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub const GLOBAL_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
/// NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS.
pub const ATTR_NV_BS_RT: u32 = 0x7;

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

const FS_IMMUTABLE_FL: c_int = 0x10;

/// `_IOR('f', 1, long)` and `_IOW('f', 2, long)`.
fn ioc(dir: c_ulong, nr: c_ulong) -> c_ulong {
    (dir << 30) | ((std::mem::size_of::<c_long>() as c_ulong) << 16) | (0x66 << 8) | nr
}

/// Clears the immutable attribute efivarfs puts on every variable file.
/// Errors are ignored: plain directories (tests) do not support it.
fn clear_immutable(path: &Path) {
    let Ok(file) = File::open(path) else { return };
    let fd = file.as_raw_fd();
    let mut flags: c_int = 0;
    // SAFETY: `fd` is a valid open descriptor owned by `file` for the whole
    // call, and `flags` is a live c_int the kernel reads or writes.
    unsafe {
        if ioctl(fd, ioc(2, 1), &mut flags as *mut c_int) != 0 {
            return;
        }
        if flags & FS_IMMUTABLE_FL != 0 {
            flags &= !FS_IMMUTABLE_FL;
            ioctl(fd, ioc(1, 2), &mut flags as *mut c_int);
        }
    }
}

#[derive(Debug, Clone)]
pub struct EfiVars {
    dir: PathBuf,
}

fn u16_list(data: &[u8], what: &str) -> Result<Vec<u16>> {
    if !data.len().is_multiple_of(2) {
        return Err(Error::new(format!("{what} has an odd length")));
    }
    Ok(le_u16s(data))
}

fn u16_bytes(list: &[u16]) -> Vec<u8> {
    list.iter().flat_map(|n| n.to_le_bytes()).collect()
}

/// `Boot0003` for 3.
pub fn boot_var_name(number: u16) -> String {
    format!("Boot{number:04X}")
}

impl EfiVars {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        EfiVars { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, name: &str, guid: &str) -> PathBuf {
        self.dir.join(format!("{name}-{guid}"))
    }

    /// Returns the variable data (without the attribute word), or `None`.
    pub fn get(&self, name: &str, guid: &str) -> Result<Option<Vec<u8>>> {
        let path = self.path(name, guid);
        match fs::read(&path) {
            Ok(raw) => {
                if raw.len() < 4 {
                    return Err(Error::new(format!(
                        "variable {name} is shorter than 4 bytes"
                    )));
                }
                Ok(Some(raw[4..].to_vec()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::new(format!("cannot read {}: {e}", path.display()))),
        }
    }

    pub fn set(&self, name: &str, guid: &str, attributes: u32, data: &[u8]) -> Result<()> {
        let path = self.path(name, guid);
        if path.exists() {
            clear_immutable(&path);
        }
        let mut buf = attributes.to_le_bytes().to_vec();
        buf.extend_from_slice(data);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .context(|| format!("cannot open {} for writing", path.display()))?;
        // efivarfs needs the whole variable in one write.
        file.write_all(&buf)
            .context(|| format!("cannot write {}", path.display()))?;
        // On plain files a shorter rewrite leaves a tail; efivarfs reports the
        // exact size already.
        let len = u64::try_from(buf.len()).unwrap_or(u64::MAX);
        if file.metadata().map(|m| m.len() > len).unwrap_or(false) {
            file.set_len(len)
                .context(|| format!("cannot truncate {}", path.display()))?;
        }
        Ok(())
    }

    /// Deletes a variable; a missing one is not an error.
    pub fn remove(&self, name: &str, guid: &str) -> Result<()> {
        let path = self.path(name, guid);
        if !path.exists() {
            return Ok(());
        }
        clear_immutable(&path);
        fs::remove_file(&path).context(|| format!("cannot delete {}", path.display()))
    }

    pub fn get_global(&self, name: &str) -> Result<Option<Vec<u8>>> {
        self.get(name, GLOBAL_GUID)
    }

    pub fn set_global(&self, name: &str, data: &[u8]) -> Result<()> {
        self.set(name, GLOBAL_GUID, ATTR_NV_BS_RT, data)
    }

    fn get_u16(&self, name: &str) -> Result<Option<u16>> {
        match self.get_global(name)? {
            None => Ok(None),
            Some(d) if d.len() == 2 => Ok(Some(u16::from_le_bytes([d[0], d[1]]))),
            Some(_) => Err(Error::new(format!("{name} is not a 16-bit value"))),
        }
    }

    pub fn boot_current(&self) -> Result<Option<u16>> {
        self.get_u16("BootCurrent")
    }

    pub fn boot_next(&self) -> Result<Option<u16>> {
        self.get_u16("BootNext")
    }

    pub fn set_boot_next(&self, number: u16) -> Result<()> {
        self.set_global("BootNext", &number.to_le_bytes())
    }

    pub fn clear_boot_next(&self) -> Result<()> {
        self.remove("BootNext", GLOBAL_GUID)
    }

    pub fn boot_order(&self) -> Result<Vec<u16>> {
        match self.get_global("BootOrder")? {
            None => Ok(Vec::new()),
            Some(d) => u16_list(&d, "BootOrder"),
        }
    }

    pub fn set_boot_order(&self, order: &[u16]) -> Result<()> {
        if order.is_empty() {
            return self.remove("BootOrder", GLOBAL_GUID);
        }
        self.set_global("BootOrder", &u16_bytes(order))
    }

    /// Numbers of all `Boot####` variables, ascending.
    pub fn boot_numbers(&self) -> Result<Vec<u16>> {
        let suffix = format!("-{GLOBAL_GUID}");
        let mut out = Vec::new();
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(format!(
                    "cannot list {}: {e} (is this a UEFI boot?)",
                    self.dir.display()
                )))
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(&suffix) else {
                continue;
            };
            let Some(hex) = stem.strip_prefix("Boot") else {
                continue;
            };
            if hex.len() == 4
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
            {
                if let Ok(n) = u16::from_str_radix(hex, 16) {
                    out.push(n);
                }
            }
        }
        out.sort_unstable();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::tmpdir;

    #[test]
    fn set_get_remove() {
        let d = tmpdir("vars");
        let v = EfiVars::new(&d);
        assert_eq!(v.get_global("BootNext").unwrap(), None);
        v.set_boot_next(0x1a).unwrap();
        let raw = fs::read(d.join(format!("BootNext-{GLOBAL_GUID}"))).unwrap();
        assert_eq!(raw, vec![7, 0, 0, 0, 0x1a, 0]);
        assert_eq!(v.boot_next().unwrap(), Some(0x1a));
        v.clear_boot_next().unwrap();
        assert_eq!(v.boot_next().unwrap(), None);
        v.clear_boot_next().unwrap();
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn rewrite_shorter_truncates() {
        let d = tmpdir("short");
        let v = EfiVars::new(&d);
        v.set_boot_order(&[1, 2, 3]).unwrap();
        v.set_boot_order(&[9]).unwrap();
        assert_eq!(v.boot_order().unwrap(), vec![9]);
        v.set_boot_order(&[]).unwrap();
        assert_eq!(v.boot_order().unwrap(), Vec::<u16>::new());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn lists_boot_entries_only() {
        let d = tmpdir("list");
        let v = EfiVars::new(&d);
        v.set_global("Boot0003", b"x").unwrap();
        v.set_global("Boot000A", b"x").unwrap();
        v.set_global("Boot000a", b"x").unwrap();
        v.set_global("BootOrder", b"xx").unwrap();
        v.set("Boot0001", "11111111-2222-3333-4444-555555555555", 7, b"x")
            .unwrap();
        assert_eq!(v.boot_numbers().unwrap(), vec![3, 10]);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn bad_values_are_errors() {
        let d = tmpdir("bad");
        let v = EfiVars::new(&d);
        v.set_global("BootOrder", &[1, 2, 3]).unwrap();
        assert!(v.boot_order().is_err());
        v.set_global("BootNext", &[1]).unwrap();
        assert!(v.boot_next().is_err());
        fs::write(d.join(format!("BootCurrent-{GLOBAL_GUID}")), [1, 2]).unwrap();
        assert!(v.boot_current().is_err());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn missing_dir_is_an_error() {
        let v = EfiVars::new("/dev/shm/silica-efi-test-does-not-exist");
        assert!(v.boot_numbers().is_err());
    }

    #[test]
    fn names() {
        assert_eq!(boot_var_name(0x2b), "Boot002B");
    }
}
