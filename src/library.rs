use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::files;

pub const PERSISTENT_SUFFIX: &str = "_P";

#[derive(Debug)]
pub enum Error {
    LibraryUnreadable(std::io::Error),
    IsLibraryRoot,
    OutsideLibrary(String),
    Delete(String, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::LibraryUnreadable(e) => write!(f, "library folder unreadable: {e}"),
            Error::IsLibraryRoot => write!(f, "refusing to delete the library folder itself"),
            Error::OutsideLibrary(name) => write!(f, "{name} is outside the library"),
            Error::Delete(name, e) => write!(f, "{name}: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    pub name: String,
    pub folder: Option<PathBuf>,
    pub primary: PathBuf,
    pub extras: Vec<PathBuf>,
    pub image: Option<PathBuf>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub blurb: Option<String>,
    pub settings: Option<String>,
    pub settings_checked: bool,
    pub source: Option<String>,
    pub bytes: u64,
    pub saved: Option<SystemTime>,
}

impl Map {
    pub fn file_count(&self) -> usize {
        1 + self.extras.len()
    }

    pub fn home(&self) -> &Path {
        self.folder.as_deref().unwrap_or(&self.primary)
    }

    pub fn key(&self) -> String {
        self.home().to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub maps: Vec<Map>,
    pub skipped: Vec<Skipped>,
}

pub fn scan(root: &Path) -> Scan {
    let mut out = Scan::default();

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) => {
            out.skipped.push(Skipped { path: root.to_path_buf(), reason: err.to_string() });
            return out;
        }
    };

    let mut loose = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        match entry.file_type() {
            Ok(kind) if kind.is_dir() => match read_map_dir(&path) {
                Ok(Some(map)) => out.maps.push(map),
                Ok(None) => {}
                Err(reason) => out.skipped.push(Skipped { path, reason }),
            },
            Ok(_) if files::is_map(&path) => loose.push(path),
            _ => {}
        }
    }

    for path in loose {
        match read_loose_map(&path) {
            Ok(map) => out.maps.push(map),
            Err(reason) => out.skipped.push(Skipped { path, reason }),
        }
    }

    out.maps.sort_by_key(|map| map.name.to_lowercase());
    out
}

fn read_map_dir(dir: &Path) -> Result<Option<Map>, String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;

    let mut maps = Vec::new();
    let mut images = Vec::new();
    let mut info = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if files::is_map(&path) {
            maps.push(path);
        } else if files::is_image(&path) {
            images.push(path);
        } else if path.file_name().is_some_and(|n| n == INFO_FILE) {
            info = Some(path);
        }
    }

    if maps.is_empty() {
        return Ok(None);
    }

    let primary = pick_primary(&maps);
    let extras = maps.into_iter().filter(|p| *p != primary).collect::<Vec<_>>();

    images.sort();
    let info = info.as_deref().map(read).unwrap_or_default();

    let folder = files::name_of(dir);

    let bytes = files::total_bytes(std::iter::once(primary.as_path()).chain(extras.iter().map(PathBuf::as_path)));

    Ok(Some(Map {
        name: info.name.unwrap_or(folder),
        folder: Some(dir.to_path_buf()),
        primary,
        extras,
        image: images.into_iter().next(),
        author: info.author,
        description: info.description,
        blurb: info.blurb,
        settings: info.settings,
        settings_checked: info.settings_checked,
        source: info.source,
        bytes,
        saved: saved_at(dir),
    }))
}

fn read_loose_map(path: &Path) -> Result<Map, String> {
    let name = files::stem_of(path).ok_or_else(|| "unnamed file".to_string())?;

    Ok(Map { name, folder: None, bytes: files::bytes_at(path), primary: path.to_path_buf(), saved: saved_at(path), ..Map::default() })
}

pub fn strip_persistent_suffix(stem: &str) -> &str {
    let cut = stem.len().saturating_sub(PERSISTENT_SUFFIX.len());
    if stem.is_char_boundary(cut) && stem[cut..].eq_ignore_ascii_case(PERSISTENT_SUFFIX) { &stem[..cut] } else { stem }
}

pub fn is_persistent(path: &Path) -> bool {
    path.file_stem().is_some_and(|stem| {
        let stem = stem.to_string_lossy();
        strip_persistent_suffix(&stem).len() != stem.len()
    })
}

pub fn pick_level<'a>(maps: impl IntoIterator<Item = &'a PathBuf>) -> Option<&'a PathBuf> {
    let maps: Vec<&PathBuf> = maps.into_iter().collect();

    let persistent: Vec<&PathBuf> = maps.iter().copied().filter(|p| is_persistent(p)).collect();
    let pool = if persistent.is_empty() { maps } else { persistent };

    pool.into_iter().max_by_key(|p| files::bytes_at(p))
}

fn pick_primary(maps: &[PathBuf]) -> PathBuf {
    pick_level(maps).cloned().unwrap_or_else(|| maps[0].clone())
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Info {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    #[serde(default)]
    pub settings_checked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub const INFO_FILE: &str = "info.json";

pub fn write(folder: &Path, info: &Info) -> std::io::Result<()> {
    let raw = serde_json::to_vec_pretty(info).map_err(std::io::Error::other)?;
    fs::write(folder.join(INFO_FILE), raw)
}

fn read(path: &Path) -> Info {
    let Ok(raw) = fs::read_to_string(path) else {
        return Info::default();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Info::default();
    };

    let mut info: Info = serde_json::from_value(value.clone()).unwrap_or_default();
    let older = |key: &str| value.get(key).and_then(serde_json::Value::as_str).map(str::to_string);

    info.name = info.name.or_else(|| older("title"));
    info.description = info.description.or_else(|| older("desc"));
    info.blurb = info.blurb.or_else(|| older("desc"));
    info.source = info.source.or_else(|| older("url"));

    for field in [&mut info.name, &mut info.author, &mut info.description, &mut info.blurb, &mut info.settings, &mut info.source] {
        if field.as_deref().is_some_and(|text| text.trim().is_empty()) {
            *field = None;
        }
    }

    info
}

fn saved_at(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    meta.created().or_else(|_| meta.modified()).ok()
}

pub fn remove(map: &Map, library_root: &Path) -> Result<(), Error> {
    let root = library_root.canonicalize().map_err(Error::LibraryUnreadable)?;

    let target = map.home().canonicalize().map_err(|e| Error::Delete(map.name.clone(), e))?;

    if target == root {
        return Err(Error::IsLibraryRoot);
    }
    if !target.starts_with(&root) {
        return Err(Error::OutsideLibrary(map.name.clone()));
    }

    match map.folder {
        Some(_) => fs::remove_dir_all(&target),
        None => fs::remove_file(&target),
    }
    .map_err(|e| Error::Delete(map.name.clone(), e))
}

#[cfg(test)]
#[path = "tests/library.rs"]
mod tests;
