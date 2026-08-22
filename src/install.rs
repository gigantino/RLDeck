use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::library::Map;
use crate::progress::{self, Progress};
use crate::{atomic, files, hash};

pub const TARGET: &str = "Labs_Underpass_P.upk";

#[cfg(windows)]
const ERROR_SHARING_VIOLATION: i32 = 32;
pub const MAPS_SUBDIR: [&str; 2] = ["TAGame", "CookedPCConsole"];

#[derive(Debug)]
pub enum Error {
    TargetMissing(PathBuf),
    NoBackup,
    BackupLost(PathBuf),
    NeedsConfirmation { bytes: u64 },
    Io(io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TargetMissing(p) => write!(f, "no {TARGET} at {}", p.display()),
            Error::NoBackup => write!(f, "no original has been backed up yet"),
            Error::BackupLost(p) => write!(f, "backup missing from {}", p.display()),
            Error::NeedsConfirmation { bytes } => {
                write!(f, "{TARGET} is {:.1} MB, which does not match a known original", *bytes as f64 / 1_000_000.0)
            }
            #[cfg(windows)]
            Error::Io(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => {
                write!(f, "close Rocket League first, then try again")
            }
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Loaded {
    pub map_name: String,
    pub sha256: String,
    pub extras: Vec<String>,
    #[serde(default)]
    pub displaced: Vec<Displaced>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Displaced {
    pub name: String,
    pub backup: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Record {
    pub original_sha256: Option<String>,
    pub original_bytes: u64,
    pub backup: Option<PathBuf>,
    pub loaded: Option<Loaded>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Unprotected,
    Original,
    Loaded(String),
    Foreign,
    Missing,
}

pub fn maps_dir(game_dir: &Path) -> PathBuf {
    MAPS_SUBDIR.iter().fold(game_dir.to_path_buf(), |p, s| p.join(s))
}

pub fn state(record: &Record, game_dir: &Path) -> Result<State> {
    let target = maps_dir(game_dir).join(TARGET);
    if !target.exists() {
        return Ok(State::Missing);
    }

    let Some(original) = record.original_sha256.as_deref() else {
        return Ok(State::Unprotected);
    };

    let bytes = fs::metadata(&target)?.len();
    let could_be_original = bytes == record.original_bytes;

    if !could_be_original && record.loaded.is_none() {
        return Ok(State::Foreign);
    }

    let digest = hash::of_file(&target)?;

    if could_be_original && digest == original {
        return Ok(State::Original);
    }

    if let Some(loaded) = &record.loaded
        && digest == loaded.sha256
    {
        return Ok(State::Loaded(loaded.map_name.clone()));
    }

    Ok(State::Foreign)
}

pub fn protect(record: &mut Record, game_dir: &Path, backup_dir: &Path, confirmed: bool) -> Result<()> {
    if record.original_sha256.is_some() {
        return Ok(());
    }

    let target = maps_dir(game_dir).join(TARGET);
    if !target.exists() {
        return Err(Error::TargetMissing(target));
    }

    let bytes = fs::metadata(&target)?.len();

    if !confirmed {
        return Err(Error::NeedsConfirmation { bytes });
    }

    let digest = hash::of_file(&target)?;

    fs::create_dir_all(backup_dir)?;
    let backup = backup_dir.join(format!("{digest}.upk"));
    if !backup.exists() {
        fs::copy(&target, &backup)?;
    }

    record.original_sha256 = Some(digest);
    record.original_bytes = bytes;
    record.backup = Some(backup);
    Ok(())
}

pub fn install(record: &mut Record, map: &Map, game_dir: &Path, backup_dir: &Path, progress: &Progress) -> Result<()> {
    if record.original_sha256.is_none() {
        return Err(Error::NoBackup);
    }

    progress.start(map.bytes);

    let dir = maps_dir(game_dir);
    fs::create_dir_all(&dir)?;

    unload(record, &dir, progress)?;

    let mut placement = Placement::new(&dir, progress);

    let outcome = map.extras.iter().try_for_each(|extra| placement.add(extra, backup_dir)).and_then(|()| write_level(map, &dir, progress));

    match outcome {
        Ok(sha256) => {
            record.loaded = Some(placement.into_loaded(map.name.clone(), sha256));
            Ok(())
        }
        Err(err) => {
            let _ = placement.revert();
            Err(err)
        }
    }
}

fn write_level(map: &Map, dir: &Path, progress: &Progress) -> Result<String> {
    let sha256 = hash::of_file(&map.primary)?;
    copy_into_place(&map.primary, &dir.join(TARGET), progress)?;
    Ok(sha256)
}

pub fn restore(record: &mut Record, game_dir: &Path, progress: &Progress) -> Result<()> {
    let backup = record.backup.clone().ok_or(Error::NoBackup)?;
    if !backup.exists() {
        return Err(Error::BackupLost(backup));
    }

    progress.start(files::bytes_at(&backup));

    let dir = maps_dir(game_dir);
    fs::create_dir_all(&dir)?;

    unload(record, &dir, progress)?;

    copy_into_place(&backup, &dir.join(TARGET), progress)
}

fn unload(record: &mut Record, dir: &Path, progress: &Progress) -> Result<()> {
    let Some(loaded) = record.loaded.as_ref() else {
        return Ok(());
    };

    Placement::of(loaded, dir, progress).revert()?;
    record.loaded = None;
    Ok(())
}

struct Placement<'a> {
    dir: &'a Path,
    progress: &'a Progress,
    extras: Vec<String>,
    displaced: Vec<Displaced>,
}

impl<'a> Placement<'a> {
    fn new(dir: &'a Path, progress: &'a Progress) -> Self {
        Self { dir, progress, extras: Vec::new(), displaced: Vec::new() }
    }

    fn of(loaded: &Loaded, dir: &'a Path, progress: &'a Progress) -> Self {
        Self { dir, progress, extras: loaded.extras.clone(), displaced: loaded.displaced.clone() }
    }

    fn add(&mut self, package: &Path, backup_dir: &Path) -> Result<()> {
        let Some(name) = package.file_name() else {
            return Ok(());
        };
        let name = name.to_string_lossy().into_owned();
        let dest = self.dir.join(&name);

        if dest.exists() {
            fs::create_dir_all(backup_dir)?;
            let parked = backup_dir.join(format!("displaced-{name}"));

            self.progress.expect(files::bytes_at(&dest));
            copy_into_place(&dest, &parked, self.progress)?;

            self.displaced.push(Displaced { name: name.clone(), backup: parked });
        }

        copy_into_place(package, &dest, self.progress)?;
        self.extras.push(name);

        Ok(())
    }

    fn revert(self) -> Result<()> {
        self.take_out_packages()?;
        self.put_back_displaced()
    }

    fn take_out_packages(&self) -> Result<()> {
        for name in &self.extras {
            let path = self.dir.join(name);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    fn put_back_displaced(&self) -> Result<()> {
        for entry in &self.displaced {
            if !entry.backup.exists() {
                continue;
            }

            self.progress.expect(files::bytes_at(&entry.backup));
            copy_into_place(&entry.backup, &self.dir.join(&entry.name), self.progress)?;
            let _ = fs::remove_file(&entry.backup);
        }

        Ok(())
    }

    fn into_loaded(self, map_name: String, sha256: String) -> Loaded {
        Loaded { map_name, sha256, extras: self.extras, displaced: self.displaced }
    }
}

fn copy_into_place(from: &Path, to: &Path, progress: &Progress) -> Result<()> {
    atomic::write(to, |part| progress::copy(from, part, progress)).map_err(Error::from)
}

#[cfg(test)]
#[path = "tests/install.rs"]
mod tests;
