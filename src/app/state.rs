use super::tasks::{blocking, fetch_art, swap_in, tracing};
use super::*;

impl RlDeck {
    pub(super) fn download(&mut self, name: String) -> Task<Message> {
        if self.busy.contains(&name) {
            return Task::none();
        }

        let Some(entry) = self.catalog.iter().find(|e| e.name == name) else {
            return Task::none();
        };

        let Some(url) = entry.download_url.clone() else {
            self.notice = Some(format!("{name} has no download link"));
            return Task::none();
        };

        let extras = fetch::Extras {
            author: Some("Lethamyr".to_string()),
            blurb: entry.description_short.as_deref().map(catalog::plain_text),
            description: entry.description.as_deref().map(catalog::plain_text),
            settings: self.settings.get(&name).cloned(),
            checked: self.checked.contains(&name),
            source: self.pages.get(&name).cloned(),
            artwork: entry.thumbnail().and_then(|url| std::fs::read(thumbs::cached_path(&thumbs::cache_dir(), url)).ok()),
        };

        self.busy.insert(name.clone());
        self.refused.remove(&name);
        self.notice = None;

        Task::perform(fetch::get_map(name, url, self.library_dir.clone(), extras), Message::Fetched)
    }

    pub(super) fn load_into_game(&mut self, key: &str, confirmed: bool) -> Task<Message> {
        let Some(game_dir) = self.game_dir.clone() else {
            self.chooser = true;
            return Task::none();
        };

        let Some(map) = self.find_map(key).cloned() else {
            return Task::none();
        };

        if self.refuse_while_working() {
            return Task::none();
        }

        let record = self.record.clone();
        let backups = config::backup_slot(&game_dir);
        let (name, bytes) = (map.name.clone(), map.bytes);

        self.start_job(
            Some(name.clone()),
            format!("loading {name}"),
            bytes,
            move |progress| swap_in(record, map, &game_dir, &backups, confirmed, progress),
            Message::Swapped,
        )
    }

    pub(super) fn start_job<T, F, M>(&mut self, map: Option<String>, label: String, bytes: u64, work: F, done: M) -> Task<Message>
    where
        F: FnOnce(&progress::Progress) -> T + Send + 'static,
        T: Send + 'static,
        M: Fn(T) -> Message + Send + 'static,
    {
        self.working = Some(Working { map, label });
        self.notice = None;

        let progress = self.progress.clone();
        progress.start(bytes);

        Task::perform(blocking(move || work(&progress)), done)
    }

    pub(super) fn refuse_while_working(&mut self) -> bool {
        let Some(working) = &self.working else {
            return false;
        };

        self.notice = Some(format!("Still {}. One at a time", working.label));
        true
    }

    pub(super) fn find_map(&self, key: &str) -> Option<&library::Map> {
        self.library.iter().find(|map| map.key() == key)
    }

    pub(super) fn rescan(&mut self) {
        self.library = library::scan(&self.library_dir).maps;
        reconcile_catalog_metadata(&mut self.library, &self.catalog);
        self.load_local_art();
    }

    pub(super) fn use_game_dir(&mut self, root: PathBuf) -> Task<Message> {
        self.record = self.config.record(&root);
        self.config.game_dir = Some(root.clone());
        self.game_dir = Some(root.clone());
        self.loaded_map = None;
        self.save_config();

        self.notice = Some(format!("Loading maps into {}", root.display()));

        let record = self.record.clone();
        let dir = root.clone();

        Task::perform(blocking(move || install::state(&record, &dir).ok()), move |state| Message::GameState(root.clone(), state))
    }

    pub(super) fn spinning(&self) -> bool {
        self.working.is_some() || !self.busy.is_empty()
    }

    pub(super) fn busy_with(&self, name: &str) -> bool {
        self.working.as_ref().and_then(|working| working.map.as_deref()) == Some(name)
    }

    pub(super) fn finished(&mut self) {
        self.working = None;
        self.progress.clear();
    }

    pub(super) fn remember(&mut self, record: install::Record) {
        self.record = record.clone();

        if let Some(dir) = self.game_dir.clone() {
            self.config.set_record(&dir, record);
            self.save_config();
        }
    }

    pub(super) fn save_config(&mut self) {
        if let Err(err) = config::save(&self.config) {
            self.notice = Some(format!("Could not save settings: {err}"));
        }
    }

    pub(super) fn prefetch_around(&mut self, index: usize, shown: usize) -> Task<Message> {
        let Some(entry) = self.catalog.get(index) else {
            return Task::none();
        };

        let count = entry.media.len();
        if count == 0 {
            return Task::none();
        }

        let wanted: Vec<String> =
            [0, 1, count - 1].into_iter().map(|offset| (shown + offset) % count).filter_map(|i| entry.media.get(i).cloned()).collect();

        let starting: Vec<String> = wanted.iter().filter_map(|url| self.gallery.prioritise(url)).collect();

        fetch_art(starting)
    }

    pub(super) fn load_local_art(&mut self) {
        let images: Vec<PathBuf> = self.library.iter().filter_map(|map| map.image.clone()).collect();

        self.gallery.add_local(images);
    }

    pub(super) fn art_missing(&self) -> bool {
        let keys: Vec<String> = self.cards().iter().filter_map(Card::art_key).collect();

        !keys.is_empty() && self.gallery.any_missing(&keys, self.scroll_of(self.tab))
    }

    pub(super) fn in_library(&self, name: &str) -> bool {
        self.library.iter().any(|map| map.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn scroll_of(&self, tab: Tab) -> f32 {
        self.scrolled.get(&tab).copied().unwrap_or(0.0)
    }

    pub(super) fn trace(&self, what: &str) {
        if !tracing() {
            return;
        }

        let art = self.gallery.tally();

        eprintln!(
            "[{what}] tab={:?} cards={} catalog={} art={} queue={} running={} unavail={} scroll={:.2} state={:?} query={:?}",
            self.tab,
            self.cards().len(),
            self.catalog.len(),
            art.held,
            art.queued,
            art.running,
            art.unavailable,
            self.scroll_of(self.tab),
            self.catalog_state,
            self.query,
        );
    }

    pub(super) fn refocus(&mut self) -> Task<Message> {
        let remote: Vec<String> = self
            .cards()
            .into_iter()
            .filter_map(|card| match card.art {
                Art::Remote(url) => Some(url),
                _ => None,
            })
            .collect();

        let open: HashSet<String> = self
            .detail
            .and_then(|(index, _)| self.catalog.get(index))
            .map(|entry| entry.media.iter().cloned().collect())
            .unwrap_or_default();

        self.gallery.focus(&remote, self.scroll_of(self.tab), &open);

        self.trace("refocus");
        self.pump()
    }

    pub(super) fn pump(&mut self) -> Task<Message> {
        fetch_art(self.gallery.next_batch())
    }

    pub(super) fn cards(&self) -> Vec<Card> {
        let query = model::Query::new(&self.query);

        let all: Vec<Card> = match self.tab {
            Tab::Library => {
                let mut maps: Vec<&library::Map> = self.library.iter().collect();

                if self.shelf == Shelf::Starred {
                    maps.retain(|map| self.config.is_starred(&map.key()));
                }

                match self.shelf {
                    Shelf::Newest | Shelf::Starred => maps.sort_by_key(|m| (m.saved.is_none(), Reverse(m.saved), m.name.to_lowercase())),
                    Shelf::Oldest => maps.sort_by_key(|m| (m.saved.is_none(), m.saved, m.name.to_lowercase())),
                }

                maps.sort_by_key(|map| self.loaded_map.as_deref() != Some(map.name.as_str()));

                maps.into_iter().map(|m| model::from_library(m, self.loaded_map.as_deref())).collect()
            }
            Tab::Explore => {
                let mut order: Vec<usize> = (0..self.catalog.len()).collect();

                if self.sort == Sort::Starred {
                    order.retain(|&i| self.config.is_starred(&catalog_star_key(&self.catalog[i].name)));
                }

                match self.sort {
                    Sort::MostLiked => order.sort_by_key(|&i| Reverse(self.catalog[i].counts.likes)),
                    Sort::MostDownloaded => order.sort_by_key(|&i| Reverse(self.catalog[i].counts.downloads)),
                    Sort::Newest | Sort::Starred => order.sort_by_key(|&i| Reverse(i)),
                }

                order
                    .into_iter()
                    .filter(|i| !self.in_library(&self.catalog[*i].name))
                    .map(|i| model::from_catalog(&self.catalog[i], i))
                    .collect()
            }
        };

        all.into_iter().filter(|card| card.matches(&query)).collect()
    }
}
