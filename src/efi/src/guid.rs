//! GUIDs in the EFI in-memory layout (the same bytes GPT stores on disk).

pub type Guid = [u8; 16];

/// GPT partition type of the EFI System Partition.
pub const ESP_TYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

/// Parses `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` into the mixed-endian EFI layout.
pub fn parse_guid(text: &str) -> Option<Guid> {
    let hex: Vec<u8> = text.bytes().filter(|b| *b != b'-').collect();
    if hex.len() != 32 || text.len() != 36 {
        return None;
    }
    let mut raw = [0u8; 16];
    for (i, byte) in raw.iter_mut().enumerate() {
        let pair = std::str::from_utf8(&hex[i * 2..i * 2 + 2]).ok()?;
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    // `raw` holds the textual (big-endian) order; swap the first three fields.
    let mut out = raw;
    out[0..4].copy_from_slice(&[raw[3], raw[2], raw[1], raw[0]]);
    out[4..6].copy_from_slice(&[raw[5], raw[4]]);
    out[6..8].copy_from_slice(&[raw[7], raw[6]]);
    Some(out)
}

/// Formats a GUID in the usual lowercase textual form.
pub fn format_guid(g: &Guid) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g[3], g[2], g[1], g[0], g[5], g[4], g[7], g[6], g[8], g[9], g[10], g[11], g[12], g[13],
        g[14], g[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_layout() {
        let text = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
        let g = parse_guid(text).unwrap();
        assert_eq!(&g[0..4], &[0x28, 0x73, 0x2a, 0xc1]);
        assert_eq!(&g[4..6], &[0x1f, 0xf8]);
        assert_eq!(&g[8..10], &[0xba, 0x4b]);
        assert_eq!(format_guid(&g), text);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_guid("nope").is_none());
        assert!(parse_guid("zzzzzzzz-f81f-11d2-ba4b-00a0c93ec93b").is_none());
    }
}
