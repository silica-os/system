//! Builders for fake disks and temporary directories. Used by the tests of
//! every crate in the workspace; never touches real devices.

use crate::crc32::crc32;
use crate::guid::{parse_guid, Guid, ESP_TYPE};
use crate::roots::Roots;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

pub struct FixturePart {
    pub number: u32,
    pub type_guid: Guid,
    pub unique_guid: Guid,
    pub first_lba: u64,
    pub last_lba: u64,
    pub name: String,
}

/// A fresh empty directory under /dev/shm.
pub fn tmpdir(tag: &str) -> PathBuf {
    let thread: String = format!("{:?}", std::thread::current().id())
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let p = PathBuf::from(format!(
        "/dev/shm/silica-test-{}-{}-{}",
        tag,
        std::process::id(),
        thread
    ));
    let _ = fs::remove_dir_all(&p);
    let _ = fs::create_dir_all(&p);
    p
}

/// The first sectors of a disk: protective MBR sector, primary GPT header and
/// a 128-entry array. Partition `number` lands in slot `number - 1`.
pub fn build_gpt(lbs: u64, parts: &[FixturePart]) -> Vec<u8> {
    let entry_size = 128usize;
    let count = 128usize;
    let mut entries = vec![0u8; entry_size * count];
    for p in parts {
        let slot = (p.number as usize).saturating_sub(1);
        if slot >= count {
            continue;
        }
        let e = &mut entries[slot * entry_size..(slot + 1) * entry_size];
        e[0..16].copy_from_slice(&p.type_guid);
        e[16..32].copy_from_slice(&p.unique_guid);
        e[32..40].copy_from_slice(&p.first_lba.to_le_bytes());
        e[40..48].copy_from_slice(&p.last_lba.to_le_bytes());
        for (i, u) in p.name.encode_utf16().take(36).enumerate() {
            e[56 + i * 2..58 + i * 2].copy_from_slice(&u.to_le_bytes());
        }
    }
    let mut header = vec![0u8; 92];
    header[0..8].copy_from_slice(b"EFI PART");
    header[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
    header[12..16].copy_from_slice(&92u32.to_le_bytes());
    header[24..32].copy_from_slice(&1u64.to_le_bytes());
    header[56..72].copy_from_slice(&[0xd1; 16]);
    header[72..80].copy_from_slice(&2u64.to_le_bytes());
    header[80..84].copy_from_slice(&(count as u32).to_le_bytes());
    header[84..88].copy_from_slice(&(entry_size as u32).to_le_bytes());
    header[88..92].copy_from_slice(&crc32(&entries).to_le_bytes());
    let crc = crc32(&header);
    header[16..20].copy_from_slice(&crc.to_le_bytes());

    let lbs = lbs as usize;
    let mut img = vec![0u8; lbs * 2];
    img[lbs..lbs + 92].copy_from_slice(&header);
    img.extend_from_slice(&entries);
    img
}

/// Adds a disk `name` to a fake system: `/sys/block/<name>`, `/dev/<name>`
/// (a file holding the start of the disk) and, when `esp_guid` is given, an
/// ESP as partition 1 (`<name>1`, device `major:minor+1`, LBA 2048, 1000 blocks).
pub fn add_disk(
    roots: &Roots,
    name: &str,
    (major, minor): (u32, u32),
    lbs: u64,
    model: &str,
    esp_guid: Option<Guid>,
) {
    let sys = &roots.sysfs;
    let disk_dir = sys.join("devices/pci/block").join(name);
    let _ = fs::create_dir_all(disk_dir.join("queue"));
    let _ = fs::create_dir_all(disk_dir.join("device"));
    let _ = fs::write(
        disk_dir.join("queue/logical_block_size"),
        format!("{lbs}\n"),
    );
    let _ = fs::write(disk_dir.join("device/model"), format!("{model:<16}\n"));
    let _ = fs::write(disk_dir.join("dev"), format!("{major}:{minor}\n"));
    let _ = fs::write(disk_dir.join("size"), "209715200\n");
    let _ = fs::write(disk_dir.join("ro"), "0\n");
    let _ = fs::create_dir_all(sys.join("dev/block"));
    let _ = fs::create_dir_all(sys.join("block"));
    let _ = fs::create_dir_all(sys.join("class/block"));
    let _ = symlink(&disk_dir, sys.join(format!("dev/block/{major}:{minor}")));
    let _ = symlink(&disk_dir, sys.join("block").join(name));
    let _ = symlink(&disk_dir, sys.join("class/block").join(name));
    let _ = fs::create_dir_all(&roots.dev);
    let mut parts = Vec::new();
    if let (Some(guid), Some(esp_type)) = (esp_guid, parse_guid(ESP_TYPE)) {
        let part = format!("{name}1");
        let part_dir = disk_dir.join(&part);
        let _ = fs::create_dir_all(&part_dir);
        let _ = fs::write(part_dir.join("dev"), format!("{major}:{}\n", minor + 1));
        let _ = fs::write(part_dir.join("partition"), "1\n");
        let _ = fs::write(part_dir.join("start"), format!("{}\n", 2048 * lbs / 512));
        let _ = fs::write(part_dir.join("size"), format!("{}\n", 1000 * lbs / 512));
        let _ = symlink(
            &part_dir,
            sys.join(format!("dev/block/{major}:{}", minor + 1)),
        );
        let _ = symlink(&part_dir, sys.join("class/block").join(&part));
        parts.push(FixturePart {
            number: 1,
            type_guid: esp_type,
            unique_guid: guid,
            first_lba: 2048,
            last_lba: 2048 + 999,
            name: "ESP".into(),
        });
        let _ = fs::write(roots.dev.join(&part), b"");
    }
    let _ = fs::write(roots.dev.join(name), build_gpt(lbs, &parts));
}

/// A fake system with disk `vda` (ESP `vda1`, GUID 42..42, mounted at /efi)
/// and an empty `/efi/EFI/Linux`.
pub fn fake_system(tag: &str, lbs: u64) -> Roots {
    let roots = Roots::new(tmpdir(tag));
    add_disk(
        &roots,
        "vda",
        (254, 0),
        lbs,
        "Virtio disk",
        Some([0x42; 16]),
    );
    let _ = fs::create_dir_all(roots.root.join("proc/self"));
    let _ = fs::write(
        roots.root.join("proc/self/mountinfo"),
        "22 1 254:1 / /efi rw,relatime - vfat /dev/vda1 rw\n23 1 0:5 / /efi/x rw - tmpfs tmpfs rw\n",
    );
    let _ = fs::create_dir_all(roots.root.join("efi/EFI/Linux"));
    roots
}
