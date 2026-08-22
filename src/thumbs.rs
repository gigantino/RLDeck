use std::path::{Path, PathBuf};

use crate::hash;

pub const MAX_IN_FLIGHT: usize = 8;
const MAX_EDGE: u32 = 640;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retry {
    Worth,
    Pointless,
}

#[derive(Debug, Clone)]
pub struct Failed {
    pub url: String,
    pub reason: String,
    pub retry: Retry,
}

impl Failed {
    fn network(url: String, reason: String) -> Self {
        Self { url, reason, retry: Retry::Worth }
    }

    fn hopeless(url: String, reason: &str) -> Self {
        Self { url, reason: reason.to_string(), retry: Retry::Pointless }
    }
}

/// The renderer drops a texture whenever the widget scrolls off-screen and
/// something else loads. Rebuilding it from encoded bytes decodes the image
/// again. Rebuilding from raw pixels copies them.
pub fn decode_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?;

    let scaled = if decoded.width() > MAX_EDGE { decoded.thumbnail(MAX_EDGE, MAX_EDGE * 2) } else { decoded };

    let rgba = scaled.to_rgba8();
    Some((rgba.width(), rgba.height(), rgba.into_raw()))
}

fn shrink(bytes: &[u8]) -> Option<Vec<u8>> {
    let decoded = image::load_from_memory(bytes).ok()?;

    if decoded.width() <= MAX_EDGE {
        return None;
    }

    let scaled = decoded.thumbnail(MAX_EDGE, MAX_EDGE * 2);
    let mut out = std::io::Cursor::new(Vec::new());
    scaled.write_to(&mut out, image::ImageFormat::Jpeg).ok().map(|_| out.into_inner())
}

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("RLDeck").join("thumbs")
}

fn key(url: &str) -> String {
    hash::short(url.as_bytes(), 16)
}

pub fn cached_path(dir: &Path, url: &str) -> PathBuf {
    dir.join(key(url))
}

pub fn looks_decodable(bytes: &[u8]) -> bool {
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF];
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G'];
    const GIF: &[u8] = b"GIF8";
    const BMP: &[u8] = b"BM";
    const RIFF: &[u8] = b"RIFF";
    const WEBP: &[u8] = b"WEBP";
    const WEBP_AT: usize = 8;
    const SHORTEST_HEADER: usize = WEBP_AT + WEBP.len();

    if bytes.len() < SHORTEST_HEADER {
        return false;
    }

    if bytes.starts_with(JPEG) || bytes.starts_with(PNG) || bytes.starts_with(GIF) || bytes.starts_with(BMP) {
        return true;
    }

    bytes.starts_with(RIFF) && bytes.get(WEBP_AT..SHORTEST_HEADER) == Some(WEBP)
}

#[derive(Clone)]
pub struct Ready {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ready({}x{}, {} bytes, {})", self.width, self.height, self.pixels.len(), self.url)
    }
}

pub async fn fetch(url: String) -> Result<Ready, Failed> {
    let dir = cache_dir();
    let path = cached_path(&dir, &url);

    if let Ok(bytes) = std::fs::read(&path) {
        if looks_decodable(&bytes) {
            return ready(url, &bytes);
        }
        let _ = std::fs::remove_file(&path);
    }

    let fetched =
        async { crate::http::client().get(&url).send().await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string()) }.await;

    let bytes = match fetched {
        Ok(bytes) => bytes.to_vec(),
        Err(reason) => return Err(Failed::network(url, reason)),
    };

    if !looks_decodable(&bytes) {
        return Err(Failed::hopeless(url, "not an image"));
    }

    let bytes = shrink(&bytes).filter(|small| looks_decodable(small)).unwrap_or(bytes);

    store(&dir, &path, &bytes);
    ready(url, &bytes)
}

fn ready(url: String, bytes: &[u8]) -> Result<Ready, Failed> {
    match decode_rgba(bytes) {
        Some((width, height, pixels)) => Ok(Ready { url, width, height, pixels }),
        None => Err(Failed::hopeless(url, "could not decode")),
    }
}

fn store(dir: &Path, path: &Path, bytes: &[u8]) {
    let _ = std::fs::create_dir_all(dir);
    let _ = crate::atomic::write(path, |part| std::fs::write(part, bytes));
}

#[cfg(test)]
#[path = "tests/thumbs.rs"]
mod tests;
