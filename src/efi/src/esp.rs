//! Finding the EFI System Partition and describing it for a boot entry.

use crate::error::{Context, Error, Result};
use crate::gpt;
use crate::guid::Guid;
use crate::loadopt::HardDrive;
use crate::roots::Roots;
use std::fs::{self, File};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspInfo {
    /// Kernel name of the parent disk, such as `vda` or `nvme0n1`.
    pub disk: String,
    pub partition_number: u32,
    /// In logical blocks of the disk.
    pub start: u64,
    pub size: u64,
    pub guid: Guid,
}

impl EspInfo {
    pub fn hard_drive(&self) -> HardDrive {
        HardDrive {
            partition_number: self.partition_number,
            start: self.start,
            size: self.size,
            signature: self.guid,
        }
    }
}

fn unescape_mountinfo(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            let v = (u32::from(b[i + 1] - b'0') << 6)
                | (u32::from(b[i + 2] - b'0') << 3)
                | u32::from(b[i + 3] - b'0');
            out.push((v & 0xff) as u8);
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn normalize(mp: &str) -> &str {
    if mp.len() > 1 {
        mp.trim_end_matches('/')
    } else {
        mp
    }
}

/// Every `(mount point, major, minor)` in a mountinfo file, in file order.
pub fn parse_mountinfo(text: &str) -> Vec<(String, u32, u32)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        if f.len() < 5 {
            continue;
        }
        let Some((maj, min)) = f[2].split_once(':') else {
            continue;
        };
        let (Ok(maj), Ok(min)) = (maj.parse(), min.parse()) else {
            continue;
        };
        out.push((unescape_mountinfo(f[4]), maj, min));
    }
    out
}

/// The device mounted at `mountpoint` (the last mount wins, as it is on top).
pub fn find_mount_dev(mountinfo: &str, mountpoint: &str) -> Option<(u32, u32)> {
    let want = normalize(mountpoint);
    parse_mountinfo(mountinfo)
        .into_iter()
        .rev()
        .find(|(mp, _, _)| normalize(mp) == want)
        .map(|(_, a, b)| (a, b))
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_num(path: &Path) -> Result<u64> {
    let text =
        read_trim(path).ok_or_else(|| Error::new(format!("cannot read {}", path.display())))?;
    text.parse()
        .map_err(|_| Error::new(format!("{} does not hold a number", path.display())))
}

/// Logical block size of a disk, 512 when sysfs does not say.
pub fn logical_block_size(roots: &Roots, disk: &str) -> u64 {
    read_trim(
        &roots
            .sysfs
            .join("block")
            .join(disk)
            .join("queue/logical_block_size"),
    )
    .and_then(|s| s.parse().ok())
    .filter(|n: &u64| *n >= 512)
    .unwrap_or(512)
}

/// Describes the partition with the given device numbers.
pub fn from_dev(roots: &Roots, major: u32, minor: u32) -> Result<EspInfo> {
    let base = roots
        .sysfs
        .join("dev/block")
        .join(format!("{major}:{minor}"));
    if !base.exists() {
        return Err(Error::new(format!(
            "device {major}:{minor} is not a block device"
        )));
    }
    let partition = read_num(&base.join("partition"))
        .context(|| format!("device {major}:{minor} is not a partition"))?;
    let start512 = read_num(&base.join("start"))?;
    let size512 = read_num(&base.join("size"))?;
    let real = fs::canonicalize(&base).context(|| format!("cannot resolve {}", base.display()))?;
    let disk = real
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::new("cannot find the parent disk of the partition"))?;
    let lbs = logical_block_size(roots, &disk);
    let start = start512 * 512 / lbs;
    let size = size512 * 512 / lbs;
    let partition_number =
        u32::try_from(partition).map_err(|_| Error::new("partition number out of range"))?;

    let dev = roots.dev.join(&disk);
    let mut file = File::open(&dev).context(|| format!("cannot open {}", dev.display()))?;
    let table = gpt::parse(&mut file, lbs)
        .context(|| format!("cannot read the partition table of {}", dev.display()))?;
    let entry = table.partition(partition_number).ok_or_else(|| {
        Error::new(format!(
            "partition {partition_number} of {disk} is not in its GPT"
        ))
    })?;
    if entry.first_lba != start {
        return Err(Error::new(format!(
            "the GPT of {disk} disagrees with the kernel about partition {partition_number} (start {} vs {start})",
            entry.first_lba
        )));
    }
    Ok(EspInfo {
        disk,
        partition_number,
        start,
        size,
        guid: entry.unique_guid,
    })
}

/// The partition mounted at `mountpoint`.
pub fn from_mountpoint(roots: &Roots, mountpoint: &str) -> Result<EspInfo> {
    let text = fs::read_to_string(&roots.mountinfo)
        .context(|| format!("cannot read {}", roots.mountinfo.display()))?;
    let (major, minor) = find_mount_dev(&text, mountpoint)
        .ok_or_else(|| Error::new(format!("nothing is mounted at {mountpoint}")))?;
    from_dev(roots, major, minor)
}

/// The partition behind a device node such as `/dev/vda1`, found by name in
/// `/sys/class/block`.
pub fn from_device_name(roots: &Roots, device: &str) -> Result<EspInfo> {
    let name = Path::new(device)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::new(format!("bad device name {device}")))?;
    let path = roots.sysfs.join("class/block").join(&name).join("dev");
    let text =
        read_trim(&path).ok_or_else(|| Error::new(format!("no such block device: {device}")))?;
    let (maj, min) = text
        .split_once(':')
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .ok_or_else(|| Error::new(format!("{} is malformed", path.display())))?;
    from_dev(roots, maj, min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::fake_system;

    #[test]
    fn mountinfo_parsing() {
        let t = "36 35 98:0 /mnt1 /mnt/with\\040space rw - ext3 /dev/x rw\n37 1 8:1 / /efi rw - vfat /dev/sda1 rw\n38 1 9:9 / /efi rw - vfat /dev/sdb1 rw\nbroken\n";
        assert_eq!(find_mount_dev(t, "/mnt/with space"), Some((98, 0)));
        assert_eq!(find_mount_dev(t, "/efi/"), Some((9, 9)));
        assert_eq!(find_mount_dev(t, "/nope"), None);
    }

    #[test]
    fn esp_from_mountpoint_for_both_block_sizes() {
        for lbs in [512u64, 4096] {
            let roots = fake_system("esp", lbs);
            let info = from_mountpoint(&roots, "/efi").unwrap();
            assert_eq!(info.disk, "vda");
            assert_eq!(info.partition_number, 1);
            assert_eq!(info.start, 2048);
            assert_eq!(info.size, 1000);
            assert_eq!(info.guid, [0x42; 16]);
            let hd = info.hard_drive();
            assert_eq!(hd.signature, [0x42; 16]);
            let by_name = from_device_name(&roots, "/dev/vda1").unwrap();
            assert_eq!(by_name, info);
            let _ = fs::remove_dir_all(&roots.root);
        }
    }

    #[test]
    fn errors_are_clear() {
        let roots = fake_system("esperr", 512);
        assert!(from_mountpoint(&roots, "/nowhere")
            .unwrap_err()
            .to_string()
            .contains("nothing is mounted"));
        assert!(from_device_name(&roots, "/dev/zzz").is_err());
        // a whole disk is not a partition
        assert!(from_device_name(&roots, "/dev/vda")
            .unwrap_err()
            .to_string()
            .contains("not a partition"));
        // GPT disagrees with the kernel
        fs::write(
            roots.sysfs.join("devices/pci/block/vda/vda1/start"),
            "4096\n",
        )
        .unwrap();
        assert!(from_dev(&roots, 254, 1)
            .unwrap_err()
            .to_string()
            .contains("disagrees"));
        let _ = fs::remove_dir_all(&roots.root);
    }
}
