use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn part_of(dest: &Path) -> PathBuf {
    let parent = dest.parent().unwrap_or(Path::new("."));
    let name = dest.file_name().unwrap_or_default().to_string_lossy();
    parent.join(format!("{name}.part"))
}

pub fn write(dest: &Path, fill: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let part = part_of(dest);

    if let Err(err) = fill(&part) {
        let _ = fs::remove_file(&part);
        return Err(err);
    }

    if let Err(err) = fs::rename(&part, dest) {
        let _ = fs::remove_file(&part);
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/atomic.rs"]
mod tests;
