use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use iced::widget::text_editor;
use iced::{Size, Subscription, Task, Theme, keyboard};

use crate::gallery::Gallery;
use crate::model::{Art, Card, Shelf, Sort, Tab};
use crate::{catalog, config, fetch, files, game, install, library, model, progress, theme, thumbs};

use crate::matching::{catalog_star_key, reconcile_catalog_metadata};
use tasks::{blocking, catalog_with_cache, survey};

mod card;
mod state;
mod tasks;
mod update;
mod view;

const CARD_W: f32 = 280.0;
const ART_H: f32 = 132.0;
const CATALOG_CARD_W: f32 = 304.0;
const CATALOG_ART_H: f32 = 172.0;
const SHOT_H: f32 = 328.0;
const SHEET_W: f32 = 660.0;
const LOCAL_SHEET_H: f32 = 540.0;
const MENU_TOP: f32 = 56.0;
pub const WINDOW: Size = Size::new(980.0, 660.0);
pub const MIN_WINDOW: Size = Size::new(760.0, 480.0);
const TURN_RATE: f32 = 7.0;
const CATALOG_TTL: u64 = 60 * 60 * 24;

#[derive(Debug, Clone)]
pub enum Loading {
    Busy,
    Ready,
    Failed(String),
}

pub struct RlDeck {
    tab: Tab,
    query: String,
    menu_open: bool,
    hovered: Option<String>,

    config: config::Config,
    installs: Vec<game::Install>,
    game_dir: Option<PathBuf>,
    record: install::Record,
    chooser: bool,
    pending: Option<Pending>,
    working: Option<Working>,
    progress: Arc<progress::Progress>,
    spin: f32,
    last_frame: Option<std::time::Instant>,

    library_dir: PathBuf,
    library: Vec<library::Map>,
    loaded_map: Option<String>,

    catalog: Vec<catalog::Entry>,
    catalog_state: Loading,
    sort: Sort,
    shelf: Shelf,
    detail: Option<(usize, usize)>,
    description: text_editor::Content,
    local_detail: Option<String>,
    scrolled: HashMap<Tab, f32>,
    window: Size,

    settings: HashMap<String, String>,
    pages: HashMap<String, String>,
    checked: HashSet<String>,

    busy: HashSet<String>,
    armed: Option<String>,
    refused: HashSet<String>,
    notice: Option<String>,

    gallery: Gallery,
}

#[derive(Debug, Clone)]
pub enum Message {
    Booted(Boot),
    CatalogLoaded(Result<Vec<catalog::Entry>, catalog::Error>),
    RetryCatalog,
    ArtLoaded(Result<thumbs::Ready, thumbs::Failed>),
    Resized(Size),
    Scrolled(f32),
    Tick,
    TabSelected(Tab),
    SortSelected(Sort),
    ShelfSelected(Shelf),
    QueryChanged(String),
    Escape,
    HoverStart(String),
    HoverEnd(String),
    Act(String),
    StarToggled(String),
    Arm(Option<String>),
    DeleteConfirmed(String),
    Fetched(Result<String, (String, fetch::Error)>),
    OpenBrowser(String),
    OpenMapPage(String, usize),
    DismissNotice,
    Absorb,
    Description(text_editor::Action),
    OpenDetail(usize),
    OpenLocal(String),
    PageDetails(String, catalog::PageDetails),
    CloseDetail,
    ShowImage(usize),
    StepImage(i32),
    Repair,
    MenuToggled,
    Imported(String),
    OpenGameFolder,
    OpenMapFolder,
    Swapped(Swap),
    Restored(Result<install::Record, String>),
    BackupConfirmed,
    BackupDeclined,
    ChooseGame,
    GameChosen(PathBuf),
    GameState(PathBuf, Option<install::State>),
    Browse(Pick),
    Picked(Pick, Option<Vec<PathBuf>>),
    CloseChooser,
    Ticked,
    Framed(std::time::Instant),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    GameFolder,
    LibraryFolder,
    MapFiles,
    MapFolder,
}

impl Pick {
    fn prompt(self) -> &'static str {
        match self {
            Pick::GameFolder => "Where is Rocket League installed?",
            Pick::LibraryFolder => "Where should maps be kept?",
            Pick::MapFiles => "Add maps to your library",
            Pick::MapFolder => "Pick a folder holding a map",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Boot {
    config: config::Config,
    installs: Vec<game::Install>,
    game_dir: Option<PathBuf>,
    record: install::Record,
    state: Option<install::State>,
    library_dir: PathBuf,
    library: Vec<library::Map>,
}

#[derive(Debug, Clone)]
pub struct Working {
    map: Option<String>,
    label: String,
}

#[derive(Debug, Clone)]
pub struct Pending {
    key: String,
    name: String,
    bytes: u64,
}

#[derive(Debug, Clone)]
pub enum Swap {
    Done { name: String, record: install::Record },
    Confirm { key: String, name: String, bytes: u64 },
    Failed(String),
}

impl RlDeck {
    pub fn boot() -> (Self, Task<Message>) {
        let deck = Self {
            tab: Tab::Library,
            query: String::new(),
            menu_open: false,
            hovered: None,
            config: config::Config::default(),
            installs: Vec::new(),
            game_dir: None,
            record: install::Record::default(),
            chooser: false,
            pending: None,
            working: None,
            progress: Arc::new(progress::Progress::default()),
            spin: 0.0,
            last_frame: None,
            library_dir: game::default_library_dir(),
            library: Vec::new(),
            loaded_map: None,
            catalog: Vec::new(),
            catalog_state: Loading::Busy,
            sort: Sort::default(),
            shelf: Shelf::default(),
            detail: None,
            description: text_editor::Content::new(),
            local_detail: None,
            scrolled: HashMap::new(),
            window: WINDOW,
            settings: HashMap::new(),
            pages: HashMap::new(),
            checked: HashSet::new(),
            busy: HashSet::new(),
            armed: None,
            refused: HashSet::new(),
            notice: None,
            gallery: Gallery::default(),
        };

        let startup =
            Task::batch([Task::perform(blocking(survey), Message::Booted), Task::perform(catalog_with_cache(), Message::CatalogLoaded)]);

        (deck, startup)
    }

    pub fn theme(&self) -> Theme {
        theme::base()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let ticking = match self.working {
            Some(_) => iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Ticked),
            None => Subscription::none(),
        };

        let animating = if self.spinning() { iced::window::frames().map(Message::Framed) } else { Subscription::none() };

        let escape = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. } => Some(Message::Escape),
            _ => None,
        });

        Subscription::batch([
            ticking,
            animating,
            escape,
            iced::window::resize_events().map(|(_, size)| Message::Resized(size)),
            iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::Tick),
        ])
    }
}

#[cfg(test)]
#[path = "tests/app.rs"]
mod tests;
