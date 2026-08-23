use std::borrow::Cow;
use std::path::PathBuf;

use crate::{catalog, library};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Local,
    Catalog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Library,
    Explore,
}

impl Tab {
    pub const ALL: [Tab; 2] = [Tab::Library, Tab::Explore];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Library => "Library",
            Tab::Explore => "Explore",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    MostLiked,
    MostDownloaded,
    Newest,
    Starred,
}

impl Sort {
    pub const ALL: [Sort; 4] = [Sort::MostLiked, Sort::MostDownloaded, Sort::Newest, Sort::Starred];
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Sort::MostLiked => "Most liked",
            Sort::MostDownloaded => "Most downloaded",
            Sort::Newest => "Newest",
            Sort::Starred => "Starred",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shelf {
    #[default]
    Newest,
    Oldest,
    Starred,
}

impl Shelf {
    pub const ALL: [Shelf; 3] = [Shelf::Newest, Shelf::Oldest, Shelf::Starred];
}

impl std::fmt::Display for Shelf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Shelf::Newest => "Newest",
            Shelf::Oldest => "Oldest",
            Shelf::Starred => "Starred",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Art {
    File(PathBuf),
    Remote(String),
    None,
}

impl Art {
    pub fn key(&self) -> Option<Cow<'_, str>> {
        match self {
            Art::File(path) => Some(path.to_string_lossy()),
            Art::Remote(url) => Some(Cow::Borrowed(url.as_str())),
            Art::None => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub key: String,
    pub name: String,
    pub author: Option<String>,
    pub blurb: Option<String>,
    pub source: Source,
    pub art: Art,
    pub loaded: bool,
    pub catalog_index: Option<usize>,
}

pub struct Query(String);

impl Query {
    pub fn new(raw: &str) -> Self {
        Query(raw.trim().to_lowercase())
    }
}

impl Card {
    pub fn art_key(&self) -> Option<String> {
        self.art.key().map(Cow::into_owned)
    }

    pub fn matches(&self, query: &Query) -> bool {
        let needle = &query.0;

        needle.is_empty()
            || self.name.to_lowercase().contains(needle)
            || self.author.as_deref().is_some_and(|author| author.to_lowercase().contains(needle))
    }
}

pub fn from_library(map: &library::Map, loaded_name: Option<&str>) -> Card {
    Card {
        key: map.key(),
        name: map.name.clone(),
        author: map.author.clone(),
        blurb: None,
        source: Source::Local,
        art: map.image.clone().map_or(Art::None, Art::File),
        loaded: loaded_name == Some(map.name.as_str()),
        catalog_index: None,
    }
}

pub fn from_catalog(entry: &catalog::Entry, index: usize) -> Card {
    Card {
        key: entry.name.clone(),
        name: entry.name.clone(),
        author: Some("Lethamyr".to_string()),
        blurb: entry.description_short.as_deref().map(catalog::plain_text),
        source: Source::Catalog,
        art: entry.thumbnail().map_or(Art::None, |url| Art::Remote(url.to_string())),
        loaded: false,
        catalog_index: Some(index),
    }
}

#[cfg(test)]
#[path = "tests/model.rs"]
mod tests;
