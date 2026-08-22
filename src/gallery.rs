use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use iced::widget::image::Handle;

use crate::thumbs;

const FETCH_AHEAD: usize = 36;
const MAX_HANDLES: usize = 96;
const MAX_RGBA_DECODES: usize = 48;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub held: usize,
    pub queued: usize,
    pub running: usize,
    pub unavailable: usize,
}

#[derive(Default)]
pub struct Gallery {
    handles: HashMap<String, Handle>,
    pinned: HashSet<String>,
    queue: VecDeque<String>,
    queued: HashSet<String>,
    running: HashSet<String>,
    attempts: HashMap<String, u8>,
    unavailable: HashSet<String>,
}

impl Gallery {
    pub fn get(&self, key: &str) -> Option<&Handle> {
        self.handles.get(key)
    }

    pub fn is_unavailable(&self, key: &str) -> bool {
        self.unavailable.contains(key)
    }

    fn settled(&self, key: &str) -> bool {
        self.handles.contains_key(key) || self.unavailable.contains(key)
    }

    fn wanted(&self, url: &str) -> bool {
        !self.settled(url) && !self.running.contains(url)
    }

    pub fn arrived(&mut self, url: String, handle: Handle) {
        self.running.remove(&url);
        self.attempts.remove(&url);
        self.handles.insert(url, handle);
    }

    pub fn failed(&mut self, url: String, retry: thumbs::Retry) {
        self.running.remove(&url);

        if retry == thumbs::Retry::Pointless {
            self.unavailable.insert(url);
            return;
        }

        let attempts = self.attempts.entry(url.clone()).or_insert(0);
        *attempts += 1;

        if *attempts < MAX_ATTEMPTS {
            self.enqueue(&url);
        }
    }

    fn enqueue(&mut self, url: &str) {
        if self.wanted(url) && self.queued.insert(url.to_string()) {
            self.queue.push_back(url.to_string());
        }
    }

    pub fn prioritise(&mut self, url: &str) -> Option<String> {
        if !self.wanted(url) {
            return None;
        }

        if self.queued.remove(url) {
            self.queue.retain(|queued| queued != url);
        }

        self.running.insert(url.to_string());
        Some(url.to_string())
    }

    pub fn next_batch(&mut self) -> Vec<String> {
        let mut starting = Vec::new();

        while self.running.len() < thumbs::MAX_IN_FLIGHT {
            let Some(url) = self.queue.pop_front() else {
                break;
            };
            self.queued.remove(&url);

            if !self.wanted(&url) {
                continue;
            }

            self.running.insert(url.clone());
            starting.push(url);
        }

        starting
    }

    pub fn focus(&mut self, remote: &[String], scrolled: f32, open: &HashSet<String>) {
        self.queue.clear();
        self.queued.clear();

        for i in focus_order(remote.len(), scrolled, FETCH_AHEAD) {
            self.attempts.remove(&remote[i]);
            self.enqueue(&remote[i]);
        }

        if remote.is_empty() || self.handles.len() <= MAX_HANDLES + self.pinned.len() {
            return;
        }

        let window: HashSet<&str> = focus_order(remote.len(), scrolled, MAX_HANDLES).into_iter().map(|i| remote[i].as_str()).collect();

        let pinned = std::mem::take(&mut self.pinned);
        self.handles.retain(|url, _| survives(url, &pinned, open, &window));
        self.pinned = pinned;
    }

    pub fn any_missing(&self, keys: &[String], scrolled: f32) -> bool {
        focus_order(keys.len(), scrolled, FETCH_AHEAD).into_iter().any(|i| !self.settled(&keys[i]))
    }

    pub fn idle(&self) -> bool {
        self.queue.is_empty() && self.running.is_empty()
    }

    pub fn add_local(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut decoded = 0usize;

        for path in paths {
            let key = path.to_string_lossy().into_owned();
            if self.handles.contains_key(&key) {
                continue;
            }

            let Ok(bytes) = std::fs::read(&path) else {
                self.unavailable.insert(key);
                continue;
            };

            if !thumbs::looks_decodable(&bytes) {
                self.unavailable.insert(key);
                continue;
            }

            let handle = if decoded < MAX_RGBA_DECODES {
                match thumbs::decode_rgba(&bytes) {
                    Some((width, height, pixels)) => {
                        decoded += 1;
                        Handle::from_rgba(width, height, pixels)
                    }
                    None => Handle::from_bytes(bytes),
                }
            } else {
                Handle::from_bytes(bytes)
            };

            self.pinned.insert(key.clone());
            self.handles.insert(key, handle);
        }
    }

    pub fn tally(&self) -> Tally {
        Tally { held: self.handles.len(), queued: self.queue.len(), running: self.running.len(), unavailable: self.unavailable.len() }
    }
}

fn survives(url: &str, pinned: &HashSet<String>, open: &HashSet<String>, window: &HashSet<&str>) -> bool {
    pinned.contains(url) || open.contains(url) || window.contains(url)
}

pub fn focus_order(len: usize, scrolled: f32, take: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }

    let focus = ((len.saturating_sub(1)) as f32 * scrolled.clamp(0.0, 1.0)).round() as usize;
    let start = focus.saturating_sub(take / 3);

    (start..len).chain((0..start).rev()).take(take.min(len)).collect()
}

#[cfg(test)]
#[path = "tests/gallery.rs"]
mod tests;
