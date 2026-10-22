//! Facts about the running system and its ESP shared by the tools.

use crate::error::{Error, Result};
use crate::roots::Roots;
use std::fs;
use std::path::Path;

/// The running Silica version: `IMAGE_VERSION` from os-release.
pub fn os_release_version(roots: &Roots) -> Option<String> {
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

/// Where firmware without (or with lost) boot entries looks for a loader.
pub const FALLBACK_PATH: &str = "\\EFI\\BOOT\\BOOTX64.EFI";

/// Makes the firmware's fallback loader a copy of the blessed version's UKI,
/// so that a machine that loses its boot entries still starts a version
/// whose system partition exists. Returns whether the file changed.
pub fn refresh_fallback(esp_dir: &Path, version: &str) -> Result<bool> {
    let uki = esp_dir
        .join("EFI/Linux")
        .join(format!("silica_{version}.efi"));
    let target = esp_dir.join("EFI/BOOT/BOOTX64.EFI");
    let wanted =
        fs::read(&uki).map_err(|e| Error::new(format!("cannot read {}: {e}", uki.display())))?;
    if fs::read(&target).ok().as_deref() == Some(wanted.as_slice()) {
        return Ok(false);
    }
    let dir = esp_dir.join("EFI/BOOT");
    fs::create_dir_all(&dir)
        .map_err(|e| Error::new(format!("cannot create {}: {e}", dir.display())))?;
    // Write a new file and rename it over the old one, so a power cut never
    // leaves a half-written loader.
    let tmp = dir.join("BOOTX64.EFI.new");
    let write = || -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut f, &wanted)?;
        f.sync_all()?;
        fs::rename(&tmp, &target)?;
        fs::File::open(&dir)?.sync_all()
    };
    write().map_err(|e| Error::new(format!("cannot update {}: {e}", target.display())))?;
    Ok(true)
}
