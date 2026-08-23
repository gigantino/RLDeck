use super::*;

pub(super) fn fetch_art(urls: Vec<String>) -> Task<Message> {
    Task::batch(urls.into_iter().map(|url| Task::perform(thumbs::fetch(url), Message::ArtLoaded)))
}

pub(super) fn has_distinct_body(blurb: Option<&str>, body: Option<&str>) -> bool {
    body.is_some_and(|body| blurb.is_none_or(|blurb| !body.trim().eq_ignore_ascii_case(blurb.trim())))
}

pub(super) fn tracing() -> bool {
    std::env::var_os("RLDECK_TRACE").is_some()
}

pub(super) fn survey() -> Boot {
    let mut config = config::load();
    let installs = game::find_installs();

    let game_dir = config.game_dir.clone().filter(|dir| game::looks_like_rocket_league(dir)).or_else(|| match installs.as_slice() {
        [only] => Some(only.root.clone()),
        _ => None,
    });

    if config.game_dir != game_dir {
        config.game_dir = game_dir.clone();
        let _ = config::save(&config);
    }

    let record = game_dir.as_deref().map(|dir| config.record(dir)).unwrap_or_default();

    let state = game_dir.as_deref().and_then(|dir| install::state(&record, dir).ok());

    let library_dir = config
        .library_dir
        .clone()
        .filter(|dir| std::fs::create_dir_all(dir).is_ok())
        .unwrap_or_else(|| game::ensure_library_dir().unwrap_or_else(|_| game::default_library_dir()));

    let library = library::scan(&library_dir).maps;

    Boot { config, installs, game_dir, record, state, library_dir, library }
}

pub(super) async fn blocking<T, F>(job: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(job).await {
        Ok(value) => value,
        Err(err) => std::panic::resume_unwind(err.into_panic()),
    }
}

pub(super) fn swap_in(
    mut record: install::Record,
    map: library::Map,
    game_dir: &Path,
    backups: &Path,
    confirmed: bool,
    progress: &progress::Progress,
) -> Swap {
    let key = map.key();
    if let Err(err) = install::protect(&mut record, game_dir, backups, confirmed) {
        return match err {
            install::Error::NeedsConfirmation { bytes } => Swap::Confirm { key, name: map.name.clone(), bytes },
            other => Swap::Failed(format!("Could not back up Underpass: {other}")),
        };
    }

    match install::install(&mut record, &map, game_dir, backups, progress) {
        Ok(()) => Swap::Done { name: map.name, record },
        Err(err) => Swap::Failed(format!("Could not load {}: {err}", map.name)),
    }
}

pub(super) fn import(paths: Vec<PathBuf>, library: &Path, progress: &progress::Progress) -> String {
    let (archives, loose): (Vec<PathBuf>, Vec<PathBuf>) =
        paths.into_iter().partition(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")));

    let mut added: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for archive in archives {
        match fetch::import_file(&archive, library, progress) {
            Ok(folder) => added.push(files::name_of(folder)),
            Err(err) => failed.push(format!("{}: {err}", files::name_of(&archive))),
        }
    }

    if !loose.is_empty() {
        match fetch::import_group(&loose, library, progress) {
            Ok(folder) => added.push(files::name_of(folder)),
            Err(err) => failed.push(err.to_string()),
        }
    }

    match (added.as_slice(), failed.as_slice()) {
        ([], []) => "Nothing to import".to_string(),
        ([], problems) => problems.join(" \u{00b7} "),
        (names, []) => format!("Added {}", names.join(", ")),
        (names, problems) => {
            format!("Added {} \u{00b7} {}", names.join(", "), problems.join(" \u{00b7} "))
        }
    }
}

pub(super) async fn ask(pick: Pick) -> Option<Vec<PathBuf>> {
    let dialog = rfd::AsyncFileDialog::new().set_title(pick.prompt());

    match pick {
        Pick::MapFiles => dialog
            .add_filter("Maps and map archives", &["zip", "upk", "udk"])
            .pick_files()
            .await
            .map(|files| files.iter().map(|f| f.path().to_path_buf()).collect()),
        _ => dialog.pick_folder().await.map(|folder| vec![folder.path().to_path_buf()]),
    }
}

pub(super) fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(super) async fn catalog_with_cache() -> Result<Vec<catalog::Entry>, catalog::Error> {
    let dir = thumbs::cache_dir();
    let now = now_unix();

    if let Some(cache) = catalog::load(&dir)
        && now.saturating_sub(cache.fetched_unix) < CATALOG_TTL
        && !cache.entries.is_empty()
    {
        return Ok(cache.entries);
    }

    let entries = catalog::fetch_all().await?;
    let _ = catalog::save(&dir, &entries, now);
    Ok(entries)
}
