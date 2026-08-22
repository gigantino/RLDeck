use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://lethamyr.com/api/v1/maps";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Network(String),
    Parse(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Network(e) => write!(f, "{e}"),
            Error::Parse(e) => write!(f, "the catalog did not look like catalog data: {e}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Counts {
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub views: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(default)]
    pub description_short: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub counts: Counts,
}

impl Entry {
    pub fn thumbnail(&self) -> Option<&str> {
        self.media.first().map(String::as_str)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub last_page: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub data: Vec<Entry>,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cache {
    pub fetched_unix: u64,
    pub entries: Vec<Entry>,
}

pub fn plain_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                let mut tag = String::new();
                for c in chars.by_ref() {
                    if c == '>' {
                        break;
                    }
                    tag.push(c);
                }
                let name = tag.trim_start_matches('/').trim().to_lowercase();
                if name.starts_with("br") || name.starts_with('p') && !name.starts_with("pre") {
                    out.push('\n');
                }
            }
            '&' => {
                let mut entity = String::new();
                let mut terminated = false;

                while let Some(&c) = chars.peek() {
                    if c == ';' {
                        chars.next();
                        terminated = true;
                        break;
                    }
                    if entity.len() >= 8 || !(c.is_ascii_alphanumeric() || c == '#') {
                        break;
                    }
                    entity.push(c);
                    chars.next();
                }

                match decode_entity(&entity).filter(|_| terminated) {
                    Some(text) => out.push_str(&text),
                    None => {
                        out.push('&');
                        out.push_str(&entity);
                        if terminated {
                            out.push(';');
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }

    collapse(&out)
}

fn decode_entity(entity: &str) -> Option<String> {
    let text = match entity.to_lowercase().as_str() {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        "hellip" => "\u{2026}",
        "mdash" => "\u{2014}",
        "ndash" => "\u{2013}",
        other => {
            let code = other.strip_prefix('#')?.parse::<u32>().ok()?;
            return char::from_u32(code).map(|c| c.to_string());
        }
    };

    Some(text.to_string())
}

fn meaningful_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|line| !line.is_empty())
}

fn collapse(text: &str) -> String {
    meaningful_lines(text).collect::<Vec<_>>().join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chunk<'a> {
    Text(&'a str),
    Link(&'a str),
}

pub fn linkify(text: &str) -> Vec<Chunk<'_>> {
    let mut chunks = Vec::new();
    let mut rest = text;

    while let Some(start) = find_link_start(rest) {
        if start > 0 {
            chunks.push(Chunk::Text(&rest[..start]));
        }

        let tail = &rest[start..];
        let end = tail.find(|c: char| c.is_whitespace()).unwrap_or(tail.len());

        let url = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '"', '\'']);

        if url.is_empty() {
            chunks.push(Chunk::Text(&tail[..end]));
        } else {
            chunks.push(Chunk::Link(url));
            if url.len() < end {
                chunks.push(Chunk::Text(&tail[url.len()..end]));
            }
        }

        rest = &tail[end..];
    }

    if !rest.is_empty() {
        chunks.push(Chunk::Text(rest));
    }

    chunks
}

fn find_link_start(text: &str) -> Option<usize> {
    ["https://", "http://", "www."].iter().filter_map(|prefix| text.find(prefix)).min()
}

pub fn absolute(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") { url.to_string() } else { format!("https://{url}") }
}

pub fn parse_page(json: &str) -> Result<Page> {
    serde_json::from_str(json).map_err(|e| Error::Parse(e.to_string()))
}

pub async fn fetch_page(page: u32) -> Result<Page> {
    let body = crate::http::client()
        .get(format!("{ENDPOINT}?page={page}"))
        .send()
        .await
        .map_err(|e| Error::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;

    parse_page(&body)
}

const MAX_PAGES: u32 = 100;

pub async fn fetch_all() -> Result<Vec<Entry>> {
    let first = fetch_page(1).await?;
    let last = first.meta.last_page.min(MAX_PAGES);
    let mut entries = first.data;

    for page in 2..=last {
        entries.extend(fetch_page(page).await?.data);
    }

    Ok(entries)
}

const SITE: &str = "https://lethamyr.com";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageDetails {
    pub url: Option<String>,
    pub settings: Option<String>,
}

pub fn parse_settings(html: &str) -> Option<String> {
    const LABEL: &str = "Recommended Settings";
    const TAIL_BYTES: usize = 800;
    const MAX_LINES: usize = 4;
    const SCRIPT_MARKERS: [&str; 2] = ["window.", "function"];

    let start = html.find(LABEL)? + LABEL.len();
    let tail = &html[start..floor_boundary(html, start + TAIL_BYTES)];

    let text = text_with_links(tail);

    let lines: Vec<&str> =
        meaningful_lines(&text).take_while(|line| !SCRIPT_MARKERS.iter().any(|marker| line.contains(marker))).take(MAX_LINES).collect();

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn text_with_links(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        out.push('\n');

        let after = &rest[at..];
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];

        if let Some(href) = href_in(tag) {
            out.push(' ');
            out.push_str(&href);
            out.push(' ');
        }

        rest = &after[close + 1..];
    }

    out.push_str(rest);
    out
}

fn href_in(tag: &str) -> Option<String> {
    const KEY: &str = "href=";

    let name = tag.trim_start_matches('<').trim_start().split(|c: char| c.is_ascii_whitespace() || c == '>').next()?;
    if !name.eq_ignore_ascii_case("a") {
        return None;
    }

    let at = find_ascii_case_insensitive(tag, KEY)? + KEY.len();
    let rest = tag[at..].trim_start();
    let quote = rest.chars().next()?;

    let value = if quote == '"' || quote == '\'' { rest[1..].split(quote).next()? } else { rest.split_whitespace().next()? };

    (!value.is_empty()).then(|| value.to_string())
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    debug_assert!(needle.is_ascii() && !needle.is_empty());

    haystack.as_bytes().windows(needle.len()).position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

pub async fn page_url(name: String, index: usize) -> String {
    details(name, index).await.url.unwrap_or_else(|| format!("{SITE}/maps"))
}

const PROBE_OFFSETS: [i64; 7] = [0, 1, -1, 2, -2, 3, -3];

/// The API exposes no id. The site numbers maps in roughly the same oldest-first
/// order the API returns them, and that numbering drifts near the newest end, so
/// arithmetic alone picks the wrong map. This walks outward from the guess and
/// compares each page's title against the name.
pub async fn details(name: String, index: usize) -> PageDetails {
    let guess = index as i64 + 1;

    for offset in PROBE_OFFSETS {
        let candidate = guess + offset;
        if candidate < 1 {
            continue;
        }

        let url = format!("{SITE}/maps/{candidate}");
        let Some(body) = body_of(&url).await else {
            continue;
        };

        if title_in(&body).is_some_and(|title| title_matches(&title, &name)) {
            return PageDetails { settings: parse_settings(&body), url: Some(url) };
        }
    }

    PageDetails::default()
}

async fn body_of(url: &str) -> Option<String> {
    crate::http::client().get(url).send().await.ok()?.text().await.ok()
}

fn title_in(body: &str) -> Option<String> {
    let start = body.find("<title>")? + "<title>".len();
    let end = body[start..].find("</title>")? + start;
    Some(body[start..end].to_string())
}

fn title_matches(title: &str, name: &str) -> bool {
    let title = title.split(" - ").next().unwrap_or(title).trim();
    title.eq_ignore_ascii_case(name.trim())
}

pub fn cache_path(dir: &Path) -> PathBuf {
    dir.join("lethamyr-catalog.json")
}

pub fn save(dir: &Path, entries: &[Entry], now_unix: u64) -> std::io::Result<()> {
    let cache = Cache { fetched_unix: now_unix, entries: entries.to_vec() };

    let raw = serde_json::to_vec_pretty(&cache).map_err(std::io::Error::other)?;
    crate::atomic::write(&cache_path(dir), |part| std::fs::write(part, &raw))
}

pub fn load(dir: &Path) -> Option<Cache> {
    let raw = std::fs::read_to_string(cache_path(dir)).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
#[path = "tests/catalog.rs"]
mod tests;
