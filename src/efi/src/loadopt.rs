//! EFI_LOAD_OPTION encoding and decoding (UEFI specification, section 3.1.3),
//! with the device path nodes Silica needs: Hard Drive and File Path.

use crate::error::{Error, Result};
use crate::guid::Guid;

pub const LOAD_OPTION_ACTIVE: u32 = 1;

const TYPE_MEDIA: u8 = 4;
const SUBTYPE_HARD_DRIVE: u8 = 1;
const SUBTYPE_FILE_PATH: u8 = 4;
const TYPE_END: u8 = 0x7f;
const SUBTYPE_END_ENTIRE: u8 = 0xff;
const HARD_DRIVE_LEN: usize = 42;

/// Hard Drive media device path node of a GPT partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardDrive {
    pub partition_number: u32,
    /// First logical block of the partition.
    pub start: u64,
    /// Size in logical blocks.
    pub size: u64,
    /// GPT unique partition GUID, in the EFI (mixed-endian) layout.
    pub signature: Guid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    HardDrive(HardDrive),
    FilePath(String),
    Other {
        kind: u8,
        subtype: u8,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOption {
    pub attributes: u32,
    pub description: String,
    /// Device path nodes, without the terminating End node.
    pub path: Vec<Node>,
    pub optional_data: Vec<u8>,
}

fn utf16z(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2 + 2);
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&[0, 0]);
    out
}

/// Little-endian 16-bit units of `data`; a trailing odd byte is ignored.
pub fn le_u16s(data: &[u8]) -> Vec<u16> {
    data.as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect()
}

/// Encodes text as UTF-16LE without a terminator (used for the optional data).
pub fn utf16_bytes(text: &str) -> Vec<u8> {
    let mut v = utf16z(text);
    v.truncate(v.len() - 2);
    v
}

/// Reads UTF-16LE up to a NUL; returns the text and the number of bytes
/// consumed, including the terminator.
fn read_utf16z(data: &[u8]) -> Result<(String, usize)> {
    let mut units = Vec::new();
    let mut i = 0;
    while i + 1 < data.len() {
        let unit = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        if unit == 0 {
            return Ok((String::from_utf16_lossy(&units), i));
        }
        units.push(unit);
    }
    Err(Error::new("unterminated UTF-16 string in load option"))
}

fn node_header(kind: u8, subtype: u8, body_len: usize, out: &mut Vec<u8>) -> Result<()> {
    let total = body_len + 4;
    let len = u16::try_from(total).map_err(|_| Error::new("device path node too long"))?;
    out.push(kind);
    out.push(subtype);
    out.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

impl Node {
    fn encode(&self, out: &mut Vec<u8>) -> Result<()> {
        match self {
            Node::HardDrive(h) => {
                node_header(TYPE_MEDIA, SUBTYPE_HARD_DRIVE, HARD_DRIVE_LEN - 4, out)?;
                out.extend_from_slice(&h.partition_number.to_le_bytes());
                out.extend_from_slice(&h.start.to_le_bytes());
                out.extend_from_slice(&h.size.to_le_bytes());
                out.extend_from_slice(&h.signature);
                out.push(2); // partition format: GPT
                out.push(2); // signature type: GUID
            }
            Node::FilePath(p) => {
                let body = utf16z(p);
                node_header(TYPE_MEDIA, SUBTYPE_FILE_PATH, body.len(), out)?;
                out.extend_from_slice(&body);
            }
            Node::Other {
                kind,
                subtype,
                data,
            } => {
                node_header(*kind, *subtype, data.len(), out)?;
                out.extend_from_slice(data);
            }
        }
        Ok(())
    }
}

impl LoadOption {
    /// An active entry that boots `file` (a `\EFI\...` path) on the partition `hd`.
    pub fn new(description: &str, hd: Option<HardDrive>, file: &str, data: &[u8]) -> Self {
        let mut path = Vec::new();
        if let Some(h) = hd {
            path.push(Node::HardDrive(h));
        }
        path.push(Node::FilePath(file.to_string()));
        LoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            description: description.to_string(),
            path,
            optional_data: data.to_vec(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.attributes & LOAD_OPTION_ACTIVE != 0
    }

    /// The first File Path node, if any.
    pub fn file_path(&self) -> Option<&str> {
        self.path.iter().find_map(|n| match n {
            Node::FilePath(p) => Some(p.as_str()),
            _ => None,
        })
    }

    pub fn hard_drive(&self) -> Option<&HardDrive> {
        self.path.iter().find_map(|n| match n {
            Node::HardDrive(h) => Some(h),
            _ => None,
        })
    }

    /// The optional data as text when it is plausible UTF-16LE.
    pub fn optional_text(&self) -> Option<String> {
        let d = &self.optional_data;
        if d.is_empty() || !d.len().is_multiple_of(2) {
            return None;
        }
        let units = le_u16s(d);
        let text = String::from_utf16(&units).ok()?;
        let text = text.trim_end_matches('\0').to_string();
        if text.is_empty() || text.chars().any(|c| c.is_control()) {
            None
        } else {
            Some(text)
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut list = Vec::new();
        for node in &self.path {
            node.encode(&mut list)?;
        }
        node_header(TYPE_END, SUBTYPE_END_ENTIRE, 0, &mut list)?;
        let list_len = u16::try_from(list.len())
            .map_err(|_| Error::new("device path too long for a load option"))?;
        let mut out = Vec::new();
        out.extend_from_slice(&self.attributes.to_le_bytes());
        out.extend_from_slice(&list_len.to_le_bytes());
        out.extend_from_slice(&utf16z(&self.description));
        out.extend_from_slice(&list);
        out.extend_from_slice(&self.optional_data);
        Ok(out)
    }

    pub fn decode(data: &[u8]) -> Result<LoadOption> {
        if data.len() < 6 {
            return Err(Error::new("load option too short"));
        }
        let attributes = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let list_len = usize::from(u16::from_le_bytes([data[4], data[5]]));
        let (description, used) = read_utf16z(&data[6..])?;
        let list_start = 6 + used;
        let list_end = list_start
            .checked_add(list_len)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| Error::new("load option device path runs past the end"))?;
        let list = &data[list_start..list_end];
        let mut path = Vec::new();
        let mut pos = 0;
        let mut ended = false;
        while pos < list.len() {
            if list.len() - pos < 4 {
                return Err(Error::new("truncated device path node"));
            }
            let (kind, subtype) = (list[pos], list[pos + 1]);
            let len = usize::from(u16::from_le_bytes([list[pos + 2], list[pos + 3]]));
            if len < 4 || pos + len > list.len() {
                return Err(Error::new("bad device path node length"));
            }
            let body = &list[pos + 4..pos + len];
            pos += len;
            if kind == TYPE_END && subtype == SUBTYPE_END_ENTIRE {
                ended = true;
                break;
            }
            path.push(decode_node(kind, subtype, body)?);
        }
        if !ended {
            return Err(Error::new("device path has no end node"));
        }
        Ok(LoadOption {
            attributes,
            description,
            path,
            optional_data: data[list_end..].to_vec(),
        })
    }
}

fn decode_node(kind: u8, subtype: u8, body: &[u8]) -> Result<Node> {
    if kind == TYPE_MEDIA && subtype == SUBTYPE_HARD_DRIVE && body.len() == HARD_DRIVE_LEN - 4 {
        let u32_at =
            |o: usize| u32::from_le_bytes([body[o], body[o + 1], body[o + 2], body[o + 3]]);
        let u64_at = |o: usize| {
            let mut b = [0u8; 8];
            b.copy_from_slice(&body[o..o + 8]);
            u64::from_le_bytes(b)
        };
        let mut signature = [0u8; 16];
        signature.copy_from_slice(&body[20..36]);
        // Only GPT (format 2, GUID signature) is modelled; others stay opaque.
        if body[36] == 2 && body[37] == 2 {
            return Ok(Node::HardDrive(HardDrive {
                partition_number: u32_at(0),
                start: u64_at(4),
                size: u64_at(12),
                signature,
            }));
        }
    }
    if kind == TYPE_MEDIA && subtype == SUBTYPE_FILE_PATH {
        let (text, _) = read_utf16z(body)?;
        return Ok(Node::FilePath(text));
    }
    Ok(Node::Other {
        kind,
        subtype,
        data: body.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hd() -> HardDrive {
        HardDrive {
            partition_number: 1,
            start: 2048,
            size: 1_048_576,
            signature: [7; 16],
        }
    }

    #[test]
    fn encode_matches_spec_layout() {
        let opt = LoadOption::new("Hi", Some(hd()), "\\a", &utf16_bytes("@1"));
        let b = opt.encode().unwrap();
        assert_eq!(&b[0..4], &[1, 0, 0, 0]); // ACTIVE
                                             // device path: HD (42) + file (4 + 6) + end (4)
        assert_eq!(u16::from_le_bytes([b[4], b[5]]), 42 + 10 + 4);
        assert_eq!(&b[6..12], &[b'H', 0, b'i', 0, 0, 0]);
        assert_eq!(&b[12..16], &[4, 1, 42, 0]);
        assert_eq!(&b[16..20], &[1, 0, 0, 0]);
        assert_eq!(&b[20..28], &2048u64.to_le_bytes());
        assert_eq!(&b[28..36], &1_048_576u64.to_le_bytes());
        assert_eq!(&b[36..52], &[7; 16]);
        assert_eq!(&b[52..54], &[2, 2]);
        assert_eq!(&b[54..58], &[4, 4, 10, 0]);
        assert_eq!(&b[58..64], &[b'\\', 0, b'a', 0, 0, 0]);
        assert_eq!(&b[64..68], &[0x7f, 0xff, 4, 0]);
        assert_eq!(&b[68..], &[b'@', 0, b'1', 0]);
    }

    #[test]
    fn round_trip() {
        let opt = LoadOption::new("Silica 1.2.3", Some(hd()), "\\EFI\\Linux\\x.efi", &[]);
        assert_eq!(LoadOption::decode(&opt.encode().unwrap()).unwrap(), opt);
        let opt = LoadOption::new(
            "Silica Recovery",
            None,
            "\\EFI\\Linux\\x.efi",
            &utf16_bytes("@1"),
        );
        let back = LoadOption::decode(&opt.encode().unwrap()).unwrap();
        assert_eq!(back, opt);
        assert_eq!(back.optional_text().as_deref(), Some("@1"));
        assert_eq!(back.file_path(), Some("\\EFI\\Linux\\x.efi"));
    }

    #[test]
    fn unknown_nodes_survive() {
        let mut opt = LoadOption::new("x", None, "\\a", &[]);
        opt.path.insert(
            0,
            Node::Other {
                kind: 2,
                subtype: 1,
                data: vec![1, 2, 3],
            },
        );
        assert_eq!(LoadOption::decode(&opt.encode().unwrap()).unwrap(), opt);
    }

    #[test]
    fn rejects_malformed() {
        assert!(LoadOption::decode(&[]).is_err());
        assert!(LoadOption::decode(&[1, 0, 0, 0, 0, 0, b'a', 0]).is_err());
        let mut b = LoadOption::new("x", None, "\\a", &[]).encode().unwrap();
        b.truncate(b.len() - 2);
        assert!(LoadOption::decode(&b).is_err());
        let mut b = LoadOption::new("x", None, "\\a", &[]).encode().unwrap();
        b[4] = 0xff; // claim a huge device path
        assert!(LoadOption::decode(&b).is_err());
    }

    #[test]
    fn optional_text_rejects_binary() {
        let mut o = LoadOption::new("x", None, "\\a", &[1, 2, 3]);
        assert_eq!(o.optional_text(), None);
        o.optional_data = vec![1, 0];
        assert_eq!(o.optional_text(), None);
    }
}
