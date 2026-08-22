use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launcher {
    Steam,
    #[cfg_attr(not(windows), allow(dead_code))]
    Epic,
}

impl Launcher {
    pub fn label(self) -> &'static str {
        match self {
            Launcher::Steam => "Steam",
            Launcher::Epic => "Epic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    pub launcher: Launcher,
    pub root: PathBuf,
}

pub fn looks_like_rocket_league(root: &Path) -> bool {
    root.join("TAGame").join("CookedPCConsole").is_dir()
}

pub fn resolve_root(picked: &Path) -> Option<PathBuf> {
    if looks_like_rocket_league(picked) {
        return Some(picked.to_path_buf());
    }

    let mut up = picked;
    for _ in 0..3 {
        let Some(parent) = up.parent() else { break };
        if looks_like_rocket_league(parent) {
            return Some(parent.to_path_buf());
        }
        up = parent;
    }

    let mut children: Vec<PathBuf> =
        std::fs::read_dir(picked).ok()?.flatten().map(|e| e.path()).filter(|p| looks_like_rocket_league(p)).collect();

    children.sort();
    children.into_iter().next()
}

fn unescape_vdf(value: &str) -> String {
    value.replace("\\\\", "\\")
}

pub fn parse_library_folders(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let line = line.trim();
            let mut parts = line.split('"').filter(|p| !p.trim().is_empty());
            let key = parts.next()?;
            if !key.eq_ignore_ascii_case("path") {
                return None;
            }
            let value = parts.next()?;
            Some(PathBuf::from(unescape_vdf(value)))
        })
        .collect()
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_epic_manifest(json: &str) -> Option<Install> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let name = value.get("DisplayName")?.as_str()?;

    if !name.to_lowercase().contains("rocket league") {
        return None;
    }

    let location = value.get("InstallLocation")?.as_str()?;
    Some(Install { launcher: Launcher::Epic, root: PathBuf::from(location) })
}

pub fn steam_installs_from_libraries(libraries: &[PathBuf]) -> Vec<Install> {
    libraries
        .iter()
        .map(|lib| lib.join("steamapps").join("common").join("rocketleague"))
        .filter(|root| looks_like_rocket_league(root))
        .map(|root| Install { launcher: Launcher::Steam, root })
        .collect()
}

pub fn find_installs() -> Vec<Install> {
    let mut found = steam_installs_from_libraries(&steam_libraries());

    for install in epic_installs() {
        if looks_like_rocket_league(&install.root) {
            found.push(install);
        }
    }

    found.sort_by_cached_key(|install| path_key(&install.root));
    found.dedup_by(|a, b| path_key(&a.root) == path_key(&b.root));
    found
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    windows_path_key(path)
}

#[cfg(not(windows))]
fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn dedup_paths(paths: &mut Vec<PathBuf>) {
    paths.sort_by_cached_key(|path| path_key(path));
    paths.dedup_by(|a, b| path_key(a) == path_key(b));
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn windows_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

#[cfg(windows)]
fn steam_libraries() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam") else {
        return Vec::new();
    };
    let Ok(steam_path) = key.get_value::<String, _>("SteamPath") else {
        return Vec::new();
    };

    let steam = PathBuf::from(steam_path.replace('/', "\\"));
    let vdf = steam.join("steamapps").join("libraryfolders.vdf");

    let mut libraries = vec![steam];
    if let Ok(raw) = std::fs::read_to_string(vdf) {
        libraries.extend(parse_library_folders(&raw));
    }

    dedup_paths(&mut libraries);
    libraries
}

#[cfg(windows)]
fn epic_installs() -> Vec<Install> {
    let dir = PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "item"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|raw| parse_epic_manifest(&raw))
        .collect()
}

#[cfg(not(windows))]
fn steam_libraries() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let roots = [
        home.join(".steam").join("steam"),
        home.join(".local").join("share").join("Steam"),
        home.join("Library").join("Application Support").join("Steam"),
    ];

    let mut libraries = Vec::new();

    for root in roots {
        if !root.is_dir() {
            continue;
        }

        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        libraries.push(root);

        if let Ok(raw) = std::fs::read_to_string(vdf) {
            libraries.extend(parse_library_folders(&raw));
        }
    }

    dedup_paths(&mut libraries);
    libraries
}

#[cfg(not(windows))]
fn epic_installs() -> Vec<Install> {
    Vec::new()
}

pub fn default_library_dir() -> PathBuf {
    let base = dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("."));

    base.join("Rocket League Maps")
}

pub fn ensure_library_dir() -> std::io::Result<PathBuf> {
    let dir = default_library_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
#[path = "tests/game.rs"]
mod tests;
