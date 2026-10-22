//! Dotted numeric version comparison and UKI file names.

use std::cmp::Ordering;

fn split_component(c: &str) -> (u64, &str) {
    let digits = c.bytes().take_while(u8::is_ascii_digit).count();
    let n = c[..digits].parse().unwrap_or(0);
    (n, &c[digits..])
}

/// Compares `1.10.0` > `1.9` numerically per component. A component with a
/// trailing suffix (`0-rc1`) sorts before the bare number; missing components
/// count as 0.
pub fn compare(a: &str, b: &str) -> Ordering {
    let ca: Vec<&str> = a.split('.').collect();
    let cb: Vec<&str> = b.split('.').collect();
    for i in 0..ca.len().max(cb.len()) {
        let (na, sa) = split_component(ca.get(i).copied().unwrap_or("0"));
        let (nb, sb) = split_component(cb.get(i).copied().unwrap_or("0"));
        let ord = na
            .cmp(&nb)
            .then_with(|| match (sa.is_empty(), sb.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => sa.cmp(sb),
            });
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.cmp(b)
}

/// `silica_1.2.3.efi` gives `1.2.3`. The match is case-insensitive because
/// the ESP is FAT.
pub fn uki_version(file_name: &str) -> Option<String> {
    let lower = file_name.to_ascii_lowercase();
    if !lower.starts_with("silica_") || !lower.ends_with(".efi") || file_name.len() <= 11 {
        return None;
    }
    let v = &file_name[7..file_name.len() - 4];
    let ok = v
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'~'));
    if ok {
        Some(v.to_string())
    } else {
        None
    }
}

pub fn uki_file_name(version: &str) -> String {
    format!("silica_{version}.efi")
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::*;

    #[test]
    fn numeric_order() {
        assert_eq!(compare("1.10", "1.9"), Greater);
        assert_eq!(compare("1.2.3", "1.2.3"), Equal);
        assert_eq!(compare("1.2", "1.2.0"), Less); // tie broken by text, stable
        assert_eq!(compare("26.08.1", "26.08"), Greater);
        assert_eq!(compare("2", "10"), Less);
        assert_eq!(compare("1.0-rc1", "1.0"), Less);
        assert_eq!(compare("1.0-rc2", "1.0-rc1"), Greater);
        assert_eq!(compare("1.0.1-rc1", "1.0"), Greater);
    }

    #[test]
    fn file_names() {
        assert_eq!(uki_version("silica_1.2.3.efi").as_deref(), Some("1.2.3"));
        assert_eq!(uki_version("SILICA_1.2.efi").as_deref(), Some("1.2"));
        assert_eq!(uki_version("silica_.efi"), None);
        assert_eq!(uki_version("silica_1.2.txt"), None);
        assert_eq!(uki_version("other_1.2.efi"), None);
        assert_eq!(uki_version("silica_a b.efi"), None);
        assert_eq!(uki_file_name("1.2"), "silica_1.2.efi");
    }
}
