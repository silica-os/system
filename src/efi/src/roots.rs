//! Where the system lives: the real `/` or a fake tree for tests.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Roots {
    pub root: PathBuf,
    pub sysfs: PathBuf,
    pub efivars: PathBuf,
    pub dev: PathBuf,
    pub mountinfo: PathBuf,
}

impl Roots {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Roots {
            sysfs: root.join("sys"),
            efivars: root.join("sys/firmware/efi/efivars"),
            dev: root.join("dev"),
            mountinfo: root.join("proc/self/mountinfo"),
            root,
        }
    }

    /// Uses `SILICA_ROOT` when set and non-empty, `/` otherwise.
    pub fn from_env() -> Self {
        match std::env::var_os("SILICA_ROOT") {
            Some(v) if !v.is_empty() => Roots::new(PathBuf::from(v)),
            _ => Roots::new("/"),
        }
    }

    pub fn with_sysfs(mut self, dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        self.efivars = dir.join("firmware/efi/efivars");
        self.sysfs = dir;
        self
    }

    pub fn with_efivars(mut self, dir: impl Into<PathBuf>) -> Self {
        self.efivars = dir.into();
        self
    }

    /// Maps an absolute path of the running system (like `/efi`) into the root.
    pub fn path(&self, absolute: impl AsRef<Path>) -> PathBuf {
        let p = absolute.as_ref();
        self.root.join(p.strip_prefix("/").unwrap_or(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_paths() {
        let r = Roots::new("/x");
        assert_eq!(r.sysfs, Path::new("/x/sys"));
        assert_eq!(r.efivars, Path::new("/x/sys/firmware/efi/efivars"));
        assert_eq!(r.path("/efi"), Path::new("/x/efi"));
        let r = r.with_sysfs("/s");
        assert_eq!(r.efivars, Path::new("/s/firmware/efi/efivars"));
        let r = r.with_efivars("/v");
        assert_eq!(r.efivars, Path::new("/v"));
    }
}
