use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::progress::{self, Counting, Progress};
use crate::{files, hash, library};

const MAX_UNPACKED: u64 = 4 * 1024 * 1024 * 1024;
const STAGING: &str = ".staging";

#[derive(Debug, Clone)]
pub enum Error {
    NeedsBrowser,
    Network(String),
    Archive(String),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NeedsBrowser => {
                write!(f, "Google Drive wants confirmation, so open it in your browser")
            }
            Error::Network(e) | Error::Archive(e) | Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn download(url: String, into: PathBuf) -> Result<PathBuf> {
    let response = crate::http::client().get(&url).send().await.map_err(|e| Error::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(Error::Network(format!("HTTP {}", response.status())));
    }

    let is_html =
        response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.contains("text/html"));

    if is_html {
        return Err(Error::NeedsBrowser);
    }

    fs::create_dir_all(&into)?;
    let path = into.join("download.part");
    let mut file = File::create(&path)?;

    let mut response = response;
    let mut written: u64 = 0;

    while let Some(chunk) = response.chunk().await.map_err(|e| Error::Network(e.to_string()))? {
        written += chunk.len() as u64;
        if written > MAX_UNPACKED {
            let _ = fs::remove_file(&path);
            return Err(Error::Archive("download exceeded the size cap".into()));
        }
        file.write_all(&chunk)?;
    }

    file.flush()?;
    Ok(path)
}

pub fn install_into_library(file: &Path, name: &str, library: &Path, progress: &Progress) -> Result<PathBuf> {
    let folder = library.join(sanitize(name));
    let folder_predates_us = folder.exists();
    fs::create_dir_all(&folder)?;

    let filled = if is_zip(file)? {
        unpack(file, &folder, progress).inspect(|()| {
            let _ = fs::remove_file(file);
        })
    } else {
        move_file(file, &folder.join(format!("{}.upk", sanitize(name)))).map_err(Error::from)
    };

    match filled {
        Ok(()) => Ok(folder),
        Err(err) => {
            if !folder_predates_us {
                let _ = fs::remove_dir_all(&folder);
            }
            Err(err)
        }
    }
}

fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }

    fs::copy(from, to)?;
    let _ = fs::remove_file(from);
    Ok(())
}

fn is_zip(path: &Path) -> Result<bool> {
    let mut magic = [0u8; 4];
    let read = File::open(path)?.read(&mut magic)?;
    Ok(read == 4 && &magic[..2] == b"PK")
}

fn unpack(archive: &Path, into: &Path, progress: &Progress) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| Error::Archive(e.to_string()))?;

    let planned = plan(&mut zip);

    if planned.is_empty() {
        return Err(Error::Archive("no map files in the archive".into()));
    }

    let declared: u64 = planned.iter().map(|entry| entry.bytes).sum();
    if declared > MAX_UNPACKED {
        return Err(Error::Archive("archive unpacks to too much".into()));
    }

    progress.start(declared);

    let mut budget = MAX_UNPACKED;

    for Planned { index, name, .. } in planned {
        let mut entry = zip.by_index(index).map_err(|e| Error::Archive(e.to_string()))?;
        let out = File::create(into.join(name))?;

        let wrote = io::copy(&mut entry.by_ref().take(budget.saturating_add(1)), &mut Counting { inner: out, progress })?;

        if wrote > budget {
            return Err(Error::Archive("archive unpacks to too much".into()));
        }
        budget -= wrote;
    }

    Ok(())
}

struct Planned {
    index: usize,
    name: String,
    bytes: u64,
}

fn plan<R: Read + io::Seek>(zip: &mut zip::ZipArchive<R>) -> Vec<Planned> {
    let mut planned = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();

    for index in 0..zip.len() {
        let Ok(entry) = zip.by_index_raw(index) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }

        let Some(name) = entry.enclosed_name().and_then(|path| path.file_name().map(|n| n.to_string_lossy().into_owned())) else {
            continue;
        };

        if name.starts_with('.') || !files::worth_extracting(&name) {
            continue;
        }

        let name = sanitize(&name);
        if !taken.insert(name.clone()) {
            continue;
        }

        planned.push(Planned { index, name, bytes: entry.size() });
    }

    planned
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() { "map".to_string() } else { trimmed.to_string() }
}

fn fill_new_folder<F>(library: &Path, name: &str, fill: F) -> Result<PathBuf>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let folder = unique_folder(library, name)?;

    match fill(&folder) {
        Ok(()) => Ok(folder),
        Err(err) => {
            let _ = fs::remove_dir_all(&folder);
            Err(err)
        }
    }
}

fn copy_all(taken: &[(PathBuf, String)], into: &Path, progress: &Progress) -> Result<()> {
    progress.start(files::total_bytes(taken.iter().map(|(path, _)| path.as_path())));

    for (path, base) in taken {
        progress::copy(path, &into.join(sanitize(base)), progress)?;
    }

    Ok(())
}

fn worth_taking(dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut taken = Vec::new();

    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let base = entry.file_name().to_string_lossy().into_owned();
        if base.starts_with('.') || !files::worth_extracting(&base) {
            continue;
        }

        taken.push((path, base));
    }

    Ok(taken)
}

pub fn import_file(file: &Path, library: &Path, progress: &Progress) -> Result<PathBuf> {
    let stem = files::stem_of(file).unwrap_or_else(|| "Map".to_string());

    fill_new_folder(library, &stem, |folder| {
        if is_zip(file)? {
            unpack(file, folder, progress)
        } else {
            copy_all(&[(file.to_path_buf(), files::name_of(file))], folder, progress)
        }
    })
}

pub fn import_group(picked: &[PathBuf], library: &Path, progress: &Progress) -> Result<PathBuf> {
    let level = pick_level(picked).ok_or_else(|| Error::Archive("no map file selected".into()))?;

    let stem = files::stem_of(level).unwrap_or_else(|| "Map".to_string());
    let name = library::strip_persistent_suffix(&stem).to_string();

    let taken: Vec<(PathBuf, String)> = picked.iter().map(|path| (path.clone(), files::name_of(path))).collect();

    fill_new_folder(library, &name, |folder| copy_all(&taken, folder, progress))
}

pub fn import_folder(dir: &Path, library: &Path, progress: &Progress) -> Result<PathBuf> {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| Error::Archive("unnamed folder".into()))?;

    let taken = worth_taking(dir)?;

    if !taken.iter().any(|(_, base)| files::is_map(base)) {
        return Err(Error::Archive(format!("no map file in {name}")));
    }

    fill_new_folder(library, &name, |folder| copy_all(&taken, folder, progress))
}

fn pick_level(picked: &[PathBuf]) -> Option<&PathBuf> {
    library::pick_level(picked.iter().filter(|path| files::is_map(path)))
}

fn unique_folder(library: &Path, name: &str) -> Result<PathBuf> {
    let base = sanitize(name);

    for suffix in 0..100 {
        let folder = match suffix {
            0 => library.join(&base),
            n => library.join(format!("{base} ({})", n + 1)),
        };

        if !folder.exists() {
            fs::create_dir_all(&folder)?;
            return Ok(folder);
        }
    }

    Err(Error::Archive(format!("too many copies of {base} already")))
}

#[derive(Debug, Clone, Default)]
pub struct Extras {
    pub author: Option<String>,
    pub blurb: Option<String>,
    pub description: Option<String>,
    pub settings: Option<String>,
    pub checked: bool,
    pub source: Option<String>,
    pub artwork: Option<Vec<u8>>,
}

impl Extras {
    fn info(&self, name: &str) -> library::Info {
        library::Info {
            name: Some(name.to_string()),
            author: self.author.clone(),
            description: self.description.clone(),
            blurb: self.blurb.clone(),
            settings: self.settings.clone(),
            settings_checked: self.checked,
            source: self.source.clone(),
        }
    }
}

fn remove_if_empty(dir: &Path) {
    let _ = fs::remove_dir(dir);
}

fn staging_for(library: &Path, name: &str, url: &str) -> PathBuf {
    library.join(STAGING).join(hash::short(format!("{name}\n{url}").as_bytes(), 8))
}

pub async fn get_map(name: String, url: String, library: PathBuf, extras: Extras) -> Result<String, (String, Error)> {
    let staging = staging_for(&library, &name, &url);

    let file = match download(url, staging.clone()).await {
        Ok(file) => file,
        Err(err) => return Err((name, err)),
    };

    let result = install_into_library(&file, &name, &library, &Progress::default());

    let _ = fs::remove_dir_all(&staging);
    remove_if_empty(&library.join(STAGING));

    match result {
        Ok(folder) => {
            write_extras(&folder, &name, &extras);
            Ok(name)
        }
        Err(err) => Err((name, err)),
    }
}

fn write_extras(folder: &Path, name: &str, extras: &Extras) {
    let has_art = fs::read_dir(folder).map(|entries| entries.flatten().any(|e| files::is_image(e.path()))).unwrap_or(false);

    if !has_art && let Some(bytes) = &extras.artwork {
        let _ = fs::write(folder.join("preview.jpg"), bytes);
    }

    if !folder.join(library::INFO_FILE).exists() {
        let _ = library::write(folder, &extras.info(name));
    }
}

#[cfg(test)]
#[path = "tests/fetch.rs"]
mod tests;
