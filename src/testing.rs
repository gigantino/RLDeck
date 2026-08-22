use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

pub fn scratch(label: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);

    let dir = std::env::temp_dir().join("rldeck-tests").join(format!("{}-{}-{label}", std::process::id(), NEXT.fetch_add(1, Relaxed),));

    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
#[path = "tests/testing.rs"]
mod tests;
