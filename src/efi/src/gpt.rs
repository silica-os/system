//! Reading the primary GUID Partition Table of a disk.

use crate::crc32::crc32;
use crate::error::{Context, Error, Result};
use crate::guid::Guid;
use crate::loadopt::le_u16s;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartEntry {
    /// 1-based partition number.
    pub number: u32,
    pub type_guid: Guid,
    pub unique_guid: Guid,
    pub first_lba: u64,
    pub last_lba: u64,
    pub name: String,
}

impl PartEntry {
    pub fn size_lba(&self) -> u64 {
        self.last_lba
            .saturating_sub(self.first_lba)
            .saturating_add(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpt {
    pub disk_guid: Guid,
    /// Used entries only.
    pub entries: Vec<PartEntry>,
}

impl Gpt {
    pub fn partition(&self, number: u32) -> Option<&PartEntry> {
        self.entries.iter().find(|e| e.number == number)
    }
}

const MAX_ENTRIES_BYTES: u64 = 4 * 1024 * 1024;

fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn le64(b: &[u8], o: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(a)
}

/// Parses the primary GPT. `lbs` is the logical block size of the device.
pub fn parse<R: Read + Seek>(disk: &mut R, lbs: u64) -> Result<Gpt> {
    if lbs < 512 || !lbs.is_power_of_two() {
        return Err(Error::new(format!("unsupported logical block size {lbs}")));
    }
    let mut header = vec![0u8; 92];
    disk.seek(SeekFrom::Start(lbs))
        .and_then(|_| disk.read_exact(&mut header))
        .context(|| "cannot read the GPT header".to_string())?;
    if &header[0..8] != b"EFI PART" {
        return Err(Error::new("no GPT found on the disk (bad signature)"));
    }
    let header_size = le32(&header, 12) as usize;
    if !(92..=lbs as usize).contains(&header_size) {
        return Err(Error::new("GPT header has a bad size"));
    }
    let mut full = vec![0u8; header_size];
    disk.seek(SeekFrom::Start(lbs))
        .and_then(|_| disk.read_exact(&mut full))
        .context(|| "cannot read the GPT header".to_string())?;
    let stored_crc = le32(&full, 16);
    full[16..20].copy_from_slice(&[0; 4]);
    if crc32(&full) != stored_crc {
        return Err(Error::new("GPT header checksum mismatch"));
    }
    let mut disk_guid = [0u8; 16];
    disk_guid.copy_from_slice(&full[56..72]);
    let entries_lba = le64(&full, 72);
    let count = u64::from(le32(&full, 80));
    let entry_size = u64::from(le32(&full, 84));
    let entries_crc = le32(&full, 88);
    if entry_size < 128
        || entry_size % 8 != 0
        || count == 0
        || count * entry_size > MAX_ENTRIES_BYTES
    {
        return Err(Error::new("GPT entry array has implausible dimensions"));
    }
    let offset = entries_lba
        .checked_mul(lbs)
        .ok_or_else(|| Error::new("GPT entry array offset overflows"))?;
    let mut raw = vec![0u8; (count * entry_size) as usize];
    disk.seek(SeekFrom::Start(offset))
        .and_then(|_| disk.read_exact(&mut raw))
        .context(|| "cannot read the GPT entries".to_string())?;
    if crc32(&raw) != entries_crc {
        return Err(Error::new("GPT entry array checksum mismatch"));
    }
    let mut entries = Vec::new();
    for (i, e) in raw.chunks_exact(entry_size as usize).enumerate() {
        if e[0..16].iter().all(|b| *b == 0) {
            continue;
        }
        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&e[0..16]);
        let mut unique_guid = [0u8; 16];
        unique_guid.copy_from_slice(&e[16..32]);
        let units: Vec<u16> = le_u16s(&e[56..128])
            .into_iter()
            .take_while(|u| *u != 0)
            .collect();
        entries.push(PartEntry {
            number: i as u32 + 1,
            type_guid,
            unique_guid,
            first_lba: le64(e, 32),
            last_lba: le64(e, 40),
            name: String::from_utf16_lossy(&units),
        });
    }
    Ok(Gpt { disk_guid, entries })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{build_gpt, FixturePart};
    use crate::guid::{parse_guid, ESP_TYPE};
    use std::io::Cursor;

    fn parts() -> Vec<FixturePart> {
        vec![
            FixturePart {
                number: 1,
                type_guid: parse_guid(ESP_TYPE).unwrap(),
                unique_guid: [0xaa; 16],
                first_lba: 2048,
                last_lba: 2048 + 1000 - 1,
                name: "ESP".into(),
            },
            FixturePart {
                number: 3,
                type_guid: [1; 16],
                unique_guid: [0xbb; 16],
                first_lba: 4096,
                last_lba: 8191,
                name: "silica_usr_1.0".into(),
            },
        ]
    }

    #[test]
    fn parses_512_and_4096() {
        for lbs in [512u64, 4096] {
            let img = build_gpt(lbs, &parts());
            let gpt = parse(&mut Cursor::new(img), lbs).unwrap();
            assert_eq!(gpt.entries.len(), 2);
            let esp = gpt.partition(1).unwrap();
            assert_eq!(esp.unique_guid, [0xaa; 16]);
            assert_eq!(esp.first_lba, 2048);
            assert_eq!(esp.size_lba(), 1000);
            assert_eq!(esp.name, "ESP");
            assert_eq!(gpt.partition(3).unwrap().name, "silica_usr_1.0");
            assert!(gpt.partition(2).is_none());
        }
    }

    #[test]
    fn wrong_block_size_fails() {
        let img = build_gpt(4096, &parts());
        assert!(parse(&mut Cursor::new(img), 512).is_err());
    }

    #[test]
    fn detects_corruption() {
        let mut img = build_gpt(512, &parts());
        img[512 + 40] ^= 1; // header field
        assert!(parse(&mut Cursor::new(img), 512).is_err());
        let mut img = build_gpt(512, &parts());
        img[1024 + 32] ^= 1; // entry data
        let e = parse(&mut Cursor::new(img), 512).unwrap_err();
        assert!(e.to_string().contains("checksum"));
        assert!(parse(&mut Cursor::new(vec![0u8; 4096]), 512).is_err());
        assert!(parse(&mut Cursor::new(vec![0u8; 100]), 512).is_err());
        assert!(parse(&mut Cursor::new(vec![0u8; 4096]), 100).is_err());
    }
}
