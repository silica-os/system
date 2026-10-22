//! A tiny command line parser: `--flag value`, `--flag=value`, booleans and
//! positional arguments.

use crate::error::{Error, Result};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone)]
pub struct Parsed {
    pub positional: Vec<String>,
    values: HashMap<String, Vec<String>>,
    flags: HashSet<String>,
}

impl Parsed {
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .get(name)
            .and_then(|v| v.last())
            .map(String::as_str)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.contains(name)
    }
}

/// `value_flags` take an argument, `bool_flags` do not. Names are given
/// without the leading dashes.
pub fn parse(args: &[String], value_flags: &[&str], bool_flags: &[&str]) -> Result<Parsed> {
    let mut out = Parsed::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == "--" {
            out.positional.extend(args[i..].iter().cloned());
            break;
        }
        let Some(rest) = arg.strip_prefix("--") else {
            out.positional.push(arg.clone());
            continue;
        };
        let (name, inline) = match rest.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (rest, None),
        };
        if value_flags.contains(&name) {
            let value = match inline {
                Some(v) => v,
                None => {
                    let v = args
                        .get(i)
                        .ok_or_else(|| Error::new(format!("--{name} needs a value")))?
                        .clone();
                    i += 1;
                    v
                }
            };
            out.values.entry(name.to_string()).or_default().push(value);
        } else if bool_flags.contains(&name) {
            if inline.is_some() {
                return Err(Error::new(format!("--{name} does not take a value")));
            }
            out.flags.insert(name.to_string());
        } else {
            return Err(Error::new(format!("unknown option --{name}")));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_all_forms() {
        let p = parse(
            &s(&["sync", "--esp", "/efi", "--label=L", "--yes", "x"]),
            &["esp", "label"],
            &["yes"],
        )
        .unwrap();
        assert_eq!(p.positional, vec!["sync", "x"]);
        assert_eq!(p.value("esp"), Some("/efi"));
        assert_eq!(p.value("label"), Some("L"));
        assert!(p.flag("yes"));
        assert!(!p.flag("no"));
    }

    #[test]
    fn errors() {
        assert!(parse(&s(&["--esp"]), &["esp"], &[]).is_err());
        assert!(parse(&s(&["--zzz"]), &[], &[]).is_err());
        assert!(parse(&s(&["--yes=1"]), &[], &["yes"]).is_err());
        let p = parse(&s(&["--", "--esp"]), &["esp"], &[]).unwrap();
        assert_eq!(p.positional, vec!["--esp"]);
    }
}
