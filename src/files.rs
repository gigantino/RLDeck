use std::fs;
use std::path::Path;

pub const MAP_EXTS: [&str; 2] = ["upk", "udk"];
pub const IMAGE_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
const INFO_EXTS: [&str; 1] = ["json"];

pub fn has_ext(path: impl AsRef<Path>, exts: &[&str]) -> bool {
    path.as_ref().extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| exts.contains(&e.as_str()))
}

pub fn is_map(path: impl AsRef<Path>) -> bool {
    has_ext(path, &MAP_EXTS)
}

pub fn is_image(path: impl AsRef<Path>) -> bool {
    has_ext(path, &IMAGE_EXTS)
}

pub fn worth_extracting(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    is_map(path) || is_image(path) || has_ext(path, &INFO_EXTS)
}

pub fn name_of(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

pub fn stem_of(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref().file_stem().map(|n| n.to_string_lossy().into_owned())
}

pub fn bytes_at(path: impl AsRef<Path>) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn total_bytes<'a>(paths: impl IntoIterator<Item = &'a Path>) -> u64 {
    paths.into_iter().map(bytes_at).sum()
}

pub fn reveal(path: &Path) {
    let target = path.to_string_lossy().into_owned();

    #[cfg(target_os = "windows")]
    spawn("explorer", &[&target]);
    #[cfg(not(target_os = "windows"))]
    spawn(OPENER, &[&target]);
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    spawn("cmd", &["/C", "start", "", url]);
    #[cfg(not(target_os = "windows"))]
    spawn(OPENER, &[url]);
}

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(all(unix, not(target_os = "macos")))]
const OPENER: &str = "xdg-open";

fn spawn(program: &str, args: &[&str]) {
    let _ = std::process::Command::new(program).args(args).spawn();
}

#[cfg(test)]
#[path = "tests/files.rs"]
mod tests;
