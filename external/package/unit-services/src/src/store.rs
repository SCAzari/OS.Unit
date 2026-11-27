//! Where the services keep their state, and how it is written.
//!
//! Settings are per profile, so they live in the profile's own folders (each profile is its own
//! Linux user, Software Architecture §3): `$XDG_CONFIG_HOME/unit` for settings and
//! `$XDG_STATE_HOME/unit` for service state. The files hold one `key=value` per line. They are
//! replaced in one step (written beside, flushed to disk, then renamed), so a power cut leaves
//! either the old file or the new one, never half of each.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn xdg_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .unwrap_or_else(|| PathBuf::from("/").join(fallback))
        .join("unit")
}

/// The profile's Unit settings folder.
pub fn config_dir() -> PathBuf {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// The profile's Unit service state folder.
pub fn state_dir() -> PathBuf {
    xdg_dir("XDG_STATE_HOME", ".local/state")
}

/// Reads `key=value` lines; blank lines and lines starting with `#` are skipped. A missing file
/// reads as empty.
pub fn read_map(path: &Path) -> io::Result<BTreeMap<String, String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e),
    }
}

pub fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub fn format(header: &str, map: &BTreeMap<String, String>) -> String {
    let mut text = format!("# {}\n", header);
    for (k, v) in map {
        text.push_str(&format!("{}={}\n", k, v));
    }
    text
}

/// Replaces a file in one step.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension("new");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    // Make the rename itself durable.
    fs::File::open(dir)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TestDir;

    #[test]
    fn reads_what_it_writes() {
        let dir = TestDir::new();
        let path = dir.path().join("sub/settings");
        let mut map = BTreeMap::new();
        map.insert("a.b".to_string(), "Inter".to_string());
        map.insert("c".to_string(), "#11756B".to_string());
        write_atomic(&path, &format("test", &map)).unwrap();
        assert_eq!(read_map(&path).unwrap(), map);
        assert!(!path.with_extension("new").exists());
    }

    #[test]
    fn missing_file_is_empty_and_junk_is_skipped() {
        let dir = TestDir::new();
        assert!(read_map(&dir.path().join("none")).unwrap().is_empty());
        let map = parse("# comment\n\nno equals sign\n k = v \n");
        assert_eq!(map.len(), 1);
        assert_eq!(map["k"], "v");
    }
}
