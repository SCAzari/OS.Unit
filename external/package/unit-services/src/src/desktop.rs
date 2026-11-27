//! Installed applications, from their desktop entries (freedesktop desktop entry
//! specification 1.5). The launcher lists them and the Session service starts them. An
//! application's ID is its entry's file name without `.desktop`, which is also the application ID
//! its windows carry, so the window switcher and the dock can match a window to its application.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub comment: String,
    /// The program and its arguments, with the field codes removed.
    pub exec: Vec<String>,
}

/// Reads one desktop entry. Returns None for anything the launcher should not show: not an
/// application, hidden, or missing its name or command.
pub fn parse_entry(id: &str, text: &str) -> Option<App> {
    let mut in_main = false;
    let mut keys = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
        } else if in_main && !line.starts_with('#') {
            if let Some((k, v)) = line.split_once('=') {
                keys.entry(k.trim()).or_insert_with(|| unescape(v.trim()));
            }
        }
    }
    let get = |k: &str| keys.get(k).cloned().unwrap_or_default();
    if get("Type") != "Application" || get("NoDisplay") == "true" || get("Hidden") == "true" {
        return None;
    }
    let name = get("Name");
    let icon = get("Icon");
    let exec = split_exec(keys.get("Exec")?, &name, &icon)?;
    if name.is_empty() || exec.is_empty() {
        return None;
    }
    Some(App { id: id.to_string(), name, icon, comment: get("Comment"), exec })
}

/// The string escapes of desktop entry values: \s \n \t \r \\.
fn unescape(v: &str) -> String {
    let mut out = String::new();
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Splits an Exec value into arguments: double quotes group an argument, in which \" \` \$ and \\
/// stand for the character itself. Field codes for files and URLs are dropped (nothing is opened
/// with a file yet); %i, %c and %% are expanded. Returns None if the quoting is broken.
pub fn split_exec(exec: &str, name: &str, icon: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => current.push(chars.next()?),
            ' ' | '\t' if !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                current.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if started {
        args.push(current);
    }
    let mut out = Vec::new();
    for arg in args {
        match arg.as_str() {
            "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" | "%k" => {}
            "%i" => {
                if !icon.is_empty() {
                    out.push("--icon".to_string());
                    out.push(icon.to_string());
                }
            }
            "%c" => out.push(name.to_string()),
            _ => out.push(arg.replace("%%", "%")),
        }
    }
    Some(out)
}

/// The folders searched for desktop entries, most important first: the profile's own, then
/// `$XDG_DATA_DIRS`.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        dirs.push(home);
    }
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let data = if data.is_empty() { "/usr/local/share:/usr/share".to_string() } else { data };
    dirs.extend(data.split(':').filter(|d| d.starts_with('/')).map(PathBuf::from));
    dirs.into_iter().map(|d| d.join("applications")).collect()
}

/// Every application in the folders, by ID; an entry in an earlier folder hides one with the
/// same ID in a later folder, even if the earlier one is hidden.
pub fn scan(dirs: &[PathBuf]) -> Vec<App> {
    let mut found: BTreeMap<String, Option<App>> = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = desktop_id(&path) else { continue };
            if found.contains_key(&id) {
                continue;
            }
            let app = std::fs::read_to_string(&path).ok().and_then(|t| parse_entry(&id, &t));
            found.insert(id, app);
        }
    }
    let mut apps: Vec<App> = found.into_values().flatten().collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn desktop_id(path: &Path) -> Option<String> {
    if path.extension()? != "desktop" {
        return None;
    }
    Some(path.file_stem()?.to_str()?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TestDir;

    const FOOT: &str = "[Desktop Entry]\nType=Application\nName=Terminal\nName[de]=Terminal DE\nComment=A terminal\nIcon=utilities-terminal\nExec=foot\n\n[Desktop Action new]\nName=New window\nExec=foot --other\n";

    #[test]
    fn reads_an_application() {
        let app = parse_entry("foot", FOOT).unwrap();
        assert_eq!(app.name, "Terminal");
        assert_eq!(app.icon, "utilities-terminal");
        assert_eq!(app.comment, "A terminal");
        assert_eq!(app.exec, vec!["foot"]);
    }

    #[test]
    fn skips_hidden_entries_and_non_applications() {
        assert!(parse_entry("x", &FOOT.replace("Exec=foot\n", "Exec=foot\nNoDisplay=true\n")).is_none());
        assert!(parse_entry("x", &FOOT.replace("Exec=foot\n", "Exec=foot\nHidden=true\n")).is_none());
        assert!(parse_entry("x", &FOOT.replace("Type=Application", "Type=Link")).is_none());
        assert!(parse_entry("x", &FOOT.replace("Exec=foot\n", "")).is_none());
    }

    #[test]
    fn splits_commands() {
        let split = |e| split_exec(e, "Editor", "edit");
        assert_eq!(split("ed %F").unwrap(), vec!["ed"]);
        assert_eq!(split("ed --title \"My \\\"Ed\\\"\" %u").unwrap(), vec!["ed", "--title", "My \"Ed\""]);
        assert_eq!(split("ed %i %c 100%%").unwrap(), vec!["ed", "--icon", "edit", "Editor", "100%"]);
        assert_eq!(split("ed \"\"").unwrap(), vec!["ed", ""]);
        assert!(split("ed \"open").is_none());
    }

    #[test]
    fn earlier_folders_win() {
        let a = TestDir::new();
        let b = TestDir::new();
        std::fs::write(a.path().join("foot.desktop"), FOOT.replace("Name=Terminal", "Name=Mine")).unwrap();
        std::fs::write(b.path().join("foot.desktop"), FOOT).unwrap();
        std::fs::write(b.path().join("editor.desktop"), FOOT.replace("Name=Terminal", "Name=Editor")).unwrap();
        std::fs::write(b.path().join("notes.txt"), "x").unwrap();
        // Hiding an entry in an earlier folder hides it everywhere.
        std::fs::write(a.path().join("gone.desktop"), "[Desktop Entry]\nType=Application\nHidden=true\n").unwrap();
        std::fs::write(b.path().join("gone.desktop"), FOOT).unwrap();
        let apps = scan(&[a.path().to_path_buf(), b.path().to_path_buf()]);
        let names: Vec<_> = apps.iter().map(|a| (a.id.as_str(), a.name.as_str())).collect();
        assert_eq!(names, [("editor", "Editor"), ("foot", "Mine")]);
    }
}
