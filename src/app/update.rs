use super::tasks::{ask, catalog_with_cache, import, tracing};
use super::*;

impl RlDeck {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Booted(boot) => self.booted(boot),
            Message::CatalogLoaded(Ok(entries)) => {
                self.catalog = entries;
                reconcile_catalog_metadata(&mut self.library, &self.catalog);
                self.catalog_state = Loading::Ready;
                return self.refocus();
            }
            Message::CatalogLoaded(Err(err)) => self.catalog_state = Loading::Failed(err.to_string()),
            Message::RetryCatalog => {
                self.catalog_state = Loading::Busy;
                return Task::perform(catalog_with_cache(), Message::CatalogLoaded);
            }
            Message::ArtLoaded(result) => return self.art_loaded(result),
            Message::TabSelected(tab) => {
                self.armed = None;
                self.trace("tab:before");
                self.tab = tab;
                self.menu_open = false;
                self.detail = None;
                return self.refocus();
            }
            Message::Resized(size) => self.window = size,
            Message::Tick => {
                if self.gallery.idle() && self.art_missing() {
                    self.trace("tick:recover");
                    return self.refocus();
                }
            }
            Message::Scrolled(offset) => {
                let moved = (offset - self.scroll_of(self.tab)).abs() > 0.01;
                self.scrolled.insert(self.tab, offset);

                if moved {
                    return self.refocus();
                }
            }
            Message::ShelfSelected(shelf) => {
                self.scrolled.insert(self.tab, 0.0);
                self.shelf = shelf;
                return self.refocus();
            }
            Message::SortSelected(sort) => {
                self.scrolled.insert(self.tab, 0.0);
                self.sort = sort;
                return self.refocus();
            }
            Message::QueryChanged(query) => {
                self.query = query;
                return self.refocus();
            }
            Message::Escape => self.escape(),
            Message::HoverStart(key) => self.hovered = Some(key),
            Message::HoverEnd(key) => {
                if self.hovered.as_deref() == Some(key.as_str()) {
                    self.hovered = None;
                }
            }
            Message::Act(key) => {
                if self.catalog.iter().any(|e| e.name == key) {
                    return self.download(key);
                }

                return self.load_into_game(&key, false);
            }
            Message::StarToggled(key) => {
                self.config.toggle_star(&key);
                self.save_config();
                return self.refocus();
            }
            Message::Arm(key) => self.armed = key,
            Message::DeleteConfirmed(key) => self.delete(key),
            Message::Fetched(Ok(name)) => self.fetched(name),
            Message::Fetched(Err((name, err))) => self.fetch_failed(name, err),
            Message::OpenBrowser(url) => files::open_url(&url),
            Message::OpenMapPage(name, index) => {
                return Task::perform(catalog::page_url(name, index), Message::OpenBrowser);
            }
            Message::DismissNotice => self.notice = None,
            Message::Absorb => {}
            Message::Description(action) => {
                if !action.is_edit() {
                    self.description.perform(action);
                }
            }
            Message::OpenDetail(index) => return self.open_detail(index),
            Message::OpenLocal(key) => {
                self.detail = None;
                self.description =
                    text_editor::Content::with_text(self.find_map(&key).and_then(|m| m.description.as_deref()).unwrap_or_default());
                self.local_detail = Some(key);
            }
            Message::PageDetails(name, details) => {
                if let Some(url) = details.url {
                    self.pages.insert(name.clone(), url);
                }
                if let Some(settings) = details.settings {
                    self.settings.insert(name.clone(), settings);
                }
                self.checked.insert(name);
            }
            Message::CloseDetail => {
                self.detail = None;
                self.local_detail = None;
                self.trace("detail:close");
            }
            Message::ShowImage(shown) => {
                if let Some((index, current)) = &mut self.detail {
                    *current = shown;
                    let index = *index;
                    return self.prefetch_around(index, shown);
                }
            }
            Message::StepImage(delta) => return self.step_image(delta),
            Message::Repair => return self.repair(),
            Message::Restored(Ok(record)) => {
                self.finished();
                self.loaded_map = None;
                self.remember(record);
                self.notice = Some("Underpass is back to normal".to_string());
            }
            Message::Restored(Err(err)) => {
                self.finished();
                self.notice = Some(format!("Could not restore Underpass: {err}"));
            }
            Message::Swapped(Swap::Done { name, record }) => {
                self.finished();
                self.loaded_map = Some(name.clone());
                self.remember(record);
                self.notice = Some(format!("{name} is loaded. Start Rocket League and play Underpass in a private match"));
            }
            Message::Swapped(Swap::Confirm { key, name, bytes }) => {
                self.finished();
                self.pending = Some(Pending { key, name, bytes });
            }
            Message::Swapped(Swap::Failed(err)) => {
                self.finished();
                self.notice = Some(err);
            }
            Message::BackupConfirmed => {
                let Some(pending) = self.pending.take() else {
                    return Task::none();
                };
                return self.load_into_game(&pending.key, true);
            }
            Message::BackupDeclined => self.pending = None,
            Message::ChooseGame => {
                self.menu_open = false;
                self.chooser = true;
            }
            Message::CloseChooser => self.chooser = false,
            Message::GameChosen(root) => {
                self.chooser = false;
                return self.use_game_dir(root);
            }
            Message::GameState(dir, state) => {
                if self.game_dir.as_deref() != Some(dir.as_path()) {
                    return Task::none();
                }

                self.loaded_map = match state {
                    Some(install::State::Loaded(name)) => Some(name),
                    _ => None,
                };
            }
            Message::Browse(pick) => {
                self.menu_open = false;
                return Task::perform(ask(pick), move |found| Message::Picked(pick, found));
            }
            Message::Picked(_, None) => {}
            Message::Picked(Pick::GameFolder, Some(paths)) => {
                return self.game_folder_picked(paths);
            }
            Message::Picked(Pick::LibraryFolder, Some(paths)) => {
                return self.library_folder_picked(paths);
            }
            Message::Picked(Pick::MapFiles, Some(paths)) => return self.import_files(paths),
            Message::Picked(Pick::MapFolder, Some(paths)) => {
                return self.import_folder_picked(paths);
            }
            Message::Imported(notice) => {
                self.finished();
                self.rescan();
                self.tab = Tab::Library;
                self.notice = Some(notice);
                return self.refocus();
            }
            Message::Ticked => {}
            Message::Framed(now) => self.framed(now),
            Message::MenuToggled => self.menu_open = !self.menu_open,
            Message::OpenMapFolder => {
                self.menu_open = false;
                files::reveal(&self.library_dir);
            }
            Message::OpenGameFolder => {
                self.menu_open = false;
                if let Some(dir) = &self.game_dir {
                    files::reveal(dir);
                }
            }
        }

        Task::none()
    }

    fn art_loaded(&mut self, result: Result<thumbs::Ready, thumbs::Failed>) -> Task<Message> {
        match result {
            Ok(ready) => {
                let handle = iced::widget::image::Handle::from_rgba(ready.width, ready.height, ready.pixels);
                self.gallery.arrived(ready.url, handle);
            }
            Err(failed) => {
                if tracing() {
                    eprintln!("[art:fail] {}  {}", failed.reason, failed.url);
                }
                self.gallery.failed(failed.url, failed.retry);
            }
        }

        self.pump()
    }

    fn booted(&mut self, boot: Boot) {
        self.config = boot.config;
        self.installs = boot.installs;
        self.game_dir = boot.game_dir;
        self.record = boot.record;
        self.library_dir = boot.library_dir;

        if let Some(install::State::Loaded(name)) = boot.state {
            self.loaded_map = Some(name);
        }

        if self.game_dir.is_none() && self.installs.len() > 1 {
            self.chooser = true;
        }

        if boot.library.is_empty() {
            self.tab = Tab::Explore;
        }
        self.library = boot.library;
        reconcile_catalog_metadata(&mut self.library, &self.catalog);
        self.load_local_art();
    }

    fn escape(&mut self) {
        if self.pending.take().is_some() {
            return;
        }
        if self.chooser {
            self.chooser = false;
            return;
        }
        if self.detail.take().is_some() || self.local_detail.take().is_some() {
            self.trace("detail:close");
            return;
        }
        if self.menu_open {
            self.menu_open = false;
            return;
        }
        self.armed = None;
    }

    fn delete(&mut self, key: String) {
        self.armed = None;

        let Some(map) = self.find_map(&key) else {
            return;
        };

        let name = map.name.clone();

        if self.loaded_map.as_deref() == Some(name.as_str()) {
            self.notice = Some(format!("{name} is in the game. Press Repair first, then delete it"));
            return;
        }

        match library::remove(map, &self.library_dir) {
            Ok(()) => {
                self.config.set_star(&key, false);
                self.save_config();
                self.rescan();
                self.notice = Some(format!("Deleted {name}"));
            }
            Err(err) => self.notice = Some(err.to_string()),
        }
    }

    fn fetched(&mut self, name: String) {
        self.busy.remove(&name);
        self.refused.remove(&name);
        let catalog_key = catalog_star_key(&name);
        let transfer_star = self.config.is_starred(&catalog_key);
        self.rescan();
        if transfer_star {
            self.config.set_star(&catalog_key, false);
            if let Some(local_key) = self.library.iter().find(|map| map.name.eq_ignore_ascii_case(&name)).map(library::Map::key) {
                self.config.set_star(&local_key, true);
            }
            self.save_config();
        }
        self.notice = Some(format!("{name} added to your library"));
    }

    fn fetch_failed(&mut self, name: String, err: fetch::Error) {
        self.busy.remove(&name);

        if matches!(err, fetch::Error::NeedsBrowser) {
            self.refused.insert(name.clone());

            if let Some(url) = self.catalog.iter().find(|e| e.name == name).and_then(|e| e.download_url.clone()) {
                files::open_url(&url);
                self.notice = Some(format!("{name}: opened in your browser to download"));
                return;
            }
        }

        self.notice = Some(format!("{name}: {err}"));
    }

    fn open_detail(&mut self, index: usize) -> Task<Message> {
        self.trace("detail:open");
        self.local_detail = None;
        self.detail = Some((index, 0));
        self.description = text_editor::Content::with_text(
            &self.catalog.get(index).and_then(|e| e.description.as_deref()).map(catalog::plain_text).unwrap_or_default(),
        );

        let mut jobs = vec![self.prefetch_around(index, 0), self.pump()];

        if let Some(entry) = self.catalog.get(index) {
            let name = entry.name.clone();
            if !self.pages.contains_key(&name) {
                jobs.push(Task::perform(
                    async move {
                        let details = catalog::details(name.clone(), index).await;
                        (name, details)
                    },
                    |(name, details)| Message::PageDetails(name, details),
                ));
            }
        }

        Task::batch(jobs)
    }

    fn framed(&mut self, now: std::time::Instant) {
        let elapsed = self
            .last_frame
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .filter(|seconds| *seconds < 0.25)
            .unwrap_or(1.0 / 60.0);

        self.last_frame = Some(now);
        self.spin = (self.spin + elapsed * TURN_RATE) % std::f32::consts::TAU;
    }

    fn step_image(&mut self, delta: i32) -> Task<Message> {
        let Some((index, current)) = self.detail else {
            return Task::none();
        };

        let count = self.catalog.get(index).map(|e| e.media.len()).unwrap_or(0).max(1);

        let shown = (current as i32 + delta).rem_euclid(count as i32) as usize;
        self.detail = Some((index, shown));
        self.prefetch_around(index, shown)
    }

    fn repair(&mut self) -> Task<Message> {
        self.menu_open = false;

        let Some(game_dir) = self.game_dir.clone() else {
            self.chooser = true;
            return Task::none();
        };

        if self.refuse_while_working() {
            return Task::none();
        }

        if self.record.backup.is_none() {
            self.notice = Some("Nothing has been replaced yet, so there is nothing to put back".to_string());
            return Task::none();
        }

        let record = self.record.clone();

        self.start_job(
            None,
            "putting Underpass back".to_string(),
            0,
            move |progress| {
                let mut record = record;
                match install::restore(&mut record, &game_dir, progress) {
                    Ok(()) => Ok(record),
                    Err(err) => Err(err.to_string()),
                }
            },
            Message::Restored,
        )
    }

    fn game_folder_picked(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        let Some(picked) = paths.into_iter().next() else {
            return Task::none();
        };

        match game::resolve_root(&picked) {
            Some(root) => {
                self.chooser = false;
                self.use_game_dir(root)
            }
            None => {
                self.notice = Some(format!("No Rocket League in {}. Look for the folder with TAGame inside it", picked.display()));
                Task::none()
            }
        }
    }

    fn library_folder_picked(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        let Some(dir) = paths.into_iter().next() else {
            return Task::none();
        };

        if let Err(err) = std::fs::create_dir_all(&dir) {
            self.notice = Some(format!("Cannot use that folder: {err}"));
            return Task::none();
        }

        self.library_dir = dir.clone();
        self.config.library_dir = Some(dir);
        self.save_config();

        self.rescan();
        self.tab = Tab::Library;
        self.notice = Some(format!("Maps now come from {}", self.library_dir.display()));
        self.refocus()
    }

    fn import_files(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        if paths.is_empty() || self.refuse_while_working() {
            return Task::none();
        }

        let label = match paths.len() {
            1 => format!("importing {}", files::name_of(&paths[0])),
            n => format!("importing {n} files"),
        };

        let library = self.library_dir.clone();
        let bytes = files::total_bytes(paths.iter().map(PathBuf::as_path));

        self.start_job(None, label, bytes, move |progress| import(paths, &library, progress), Message::Imported)
    }

    fn import_folder_picked(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        let Some(dir) = paths.into_iter().next() else {
            return Task::none();
        };

        if self.refuse_while_working() {
            return Task::none();
        }

        let label = format!("importing {}", files::name_of(&dir));
        let library = self.library_dir.clone();

        self.start_job(
            None,
            label,
            0,
            move |progress| match fetch::import_folder(&dir, &library, progress) {
                Ok(folder) => format!("Added {}", files::name_of(&folder)),
                Err(err) => format!("Could not import that folder: {err}"),
            },
            Message::Imported,
        )
    }
}
