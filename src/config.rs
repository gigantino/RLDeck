use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic;
use crate::hash;
use crate::install::Record;

#[derive(Debug)]
pub enum Error {
    Serialize(serde_json::Error),
    Write(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Serialize(e) => write!(f, "{e}"),
            Error::Write(path, e) => write!(f, "{}: {e}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub game_dir: Option<PathBuf>,
    #[serde(default)]
    pub library_dir: Option<PathBuf>,
    #[serde(default)]
    records: BTreeMap<String, Record>,
    #[serde(default)]
    starred: BTreeSet<String>,
}

impl Config {
    pub fn record(&self, game_dir: &Path) -> Record {
        self.records.get(&key(game_dir)).cloned().unwrap_or_default()
    }

    pub fn set_record(&mut self, game_dir: &Path, record: Record) {
        self.records.insert(key(game_dir), record);
    }

    pub fn is_starred(&self, map_key: &str) -> bool {
        self.starred.contains(map_key)
    }

    pub fn toggle_star(&mut self, map_key: &str) -> bool {
        let starred = !self.is_starred(map_key);
        self.set_star(map_key, starred);
        starred
    }

    pub fn set_star(&mut self, map_key: &str, starred: bool) {
        if starred {
            self.starred.insert(map_key.to_string());
        } else {
            self.starred.remove(map_key);
        }
    }
}

#[cfg(not(windows))]
fn key(game_dir: &Path) -> String {
    game_dir.to_string_lossy().into_owned()
}

#[cfg(windows)]
fn key(game_dir: &Path) -> String {
    game_dir.to_string_lossy().replace('/', "\\").to_lowercase()
}

fn dir() -> PathBuf {
    dirs::config_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from(".")).join("RLDeck")
}

pub fn path() -> PathBuf {
    dir().join("config.json")
}

fn backups_root() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RLDeck")
        .join("originals")
}

pub fn backup_slot(game_dir: &Path) -> PathBuf {
    backups_root().join(hash::short(game_dir.to_string_lossy().as_bytes(), 6))
}

pub fn load() -> Config {
    fs::read_to_string(path()).ok().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

pub fn save(config: &Config) -> Result<(), Error> {
    let raw = serde_json::to_string_pretty(config).map_err(Error::Serialize)?;

    let dest = path();
    atomic::write(&dest, |part| fs::write(part, &raw)).map_err(|e| Error::Write(dest, e))
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
