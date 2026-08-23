use super::tasks::{has_distinct_body, swap_in};
use super::*;
use crate::hash;
use crate::testing::scratch as tmp;
use crate::theme::{S3, S4};
use std::fs;

#[test]
fn a_multi_file_map_goes_from_an_archive_into_the_game_and_back() {
    use std::io::Write;

    let root = tmp("e2e");
    let library_dir = root.join("library");
    let game_dir = root.join("rocketleague");
    let backups = root.join("originals");
    let cooked = install::maps_dir(&game_dir);
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&cooked).unwrap();

    let underpass = cooked.join(install::TARGET);
    fs::write(&underpass, vec![0x11; 4_120_000]).unwrap();
    let stock = hash::of_file(&underpass).unwrap();

    let archive = root.join("Whack A Mole.zip");
    {
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();

        for (name, byte, len) in [
            ("Whack_A_Mole_P.upk", 0xAAu8, 40_000usize),
            ("Whack_A_Mole_Textures.upk", 0xBB, 8_000),
            ("Whack_A_Mole_Meshes.upk", 0xCC, 4_000),
            ("preview.jpg", 0xDD, 512),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(&vec![byte; len]).unwrap();
        }

        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"install instructions").unwrap();
        zip.finish().unwrap();
    }

    let watched = progress::Progress::default();
    fetch::import_file(&archive, &library_dir, &watched).unwrap();
    assert_eq!(watched.fraction(), Some(1.0), "an import must finish its own bar, or it hangs at 90% forever");

    let maps = library::scan(&library_dir).maps;
    assert_eq!(maps.len(), 1, "one archive is one map");
    let map = maps[0].clone();
    assert_eq!(map.name, "Whack A Mole");
    assert_eq!(map.file_count(), 3);
    assert!(map.primary.ends_with("Whack_A_Mole_P.upk"), "the _P package is the level");
    assert!(map.image.is_some(), "artwork travels with it");

    let level = hash::of_file(&map.primary).unwrap();

    watched.clear();

    let record = match swap_in(install::Record::default(), map.clone(), &game_dir, &backups, true, &watched) {
        Swap::Done { record, name } => {
            assert_eq!(name, "Whack A Mole");
            record
        }
        other => panic!("expected the map to load, got {other:?}"),
    };

    assert_eq!(hash::of_file(&underpass).unwrap(), level, "Underpass should now be the custom level, byte for byte");
    assert!(cooked.join("Whack_A_Mole_Textures.upk").exists());
    assert!(cooked.join("Whack_A_Mole_Meshes.upk").exists());
    assert!(!cooked.join("preview.jpg").exists(), "only packages belong in the game folder");
    assert_eq!(install::state(&record, &game_dir).unwrap(), install::State::Loaded("Whack A Mole".into()));
    assert_eq!(watched.done(), map.bytes, "every byte of a multi-file map should be counted, not just the level");

    let mut record = record;
    install::restore(&mut record, &game_dir, &progress::Progress::default()).unwrap();

    assert_eq!(hash::of_file(&underpass).unwrap(), stock, "Repair must give back the exact file the game shipped");
    assert!(!cooked.join("Whack_A_Mole_Textures.upk").exists());
    assert!(!cooked.join("Whack_A_Mole_Meshes.upk").exists());
    assert_eq!(install::state(&record, &game_dir).unwrap(), install::State::Original);
}

#[test]
fn a_load_stops_and_asks_before_it_backs_anything_up() {
    let root = tmp("ask");
    let game_dir = root.join("rocketleague");
    let cooked = install::maps_dir(&game_dir);
    fs::create_dir_all(&cooked).unwrap();
    fs::write(cooked.join(install::TARGET), vec![0x11; 4_120_000]).unwrap();

    let library_dir = root.join("library");
    fs::create_dir_all(library_dir.join("Cool")).unwrap();
    fs::write(library_dir.join("Cool/Cool_P.upk"), vec![0xAA; 2048]).unwrap();

    let map = library::scan(&library_dir).maps.remove(0);

    match swap_in(install::Record::default(), map, &game_dir, &root.join("originals"), false, &progress::Progress::default()) {
        Swap::Confirm { key, name, bytes } => {
            assert_eq!(key, library_dir.join("Cool").to_string_lossy());
            assert_eq!(name, "Cool");
            assert_eq!(bytes, 4_120_000);
        }
        other => panic!("expected a question, got {other:?}"),
    }

    assert!(!root.join("originals").exists(), "nothing copied without a yes");
}

#[test]
fn confirming_a_backup_retries_with_the_maps_filesystem_key() {
    let root = tmp("confirm-key");
    let map_dir = root.join("library/Cool");
    fs::create_dir_all(&map_dir).unwrap();
    fs::write(map_dir.join("Cool_P.upk"), vec![0xAA; 64]).unwrap();
    let map = library::scan(&root.join("library")).maps.remove(0);
    let key = map.key();

    let mut deck = RlDeck::boot().0;
    deck.game_dir = Some(root.join("rocketleague"));
    deck.library = vec![map];
    deck.pending = Some(Pending { key, name: "Cool".to_string(), bytes: 64 });

    let task = deck.update(Message::BackupConfirmed);
    assert_eq!(
        deck.working.as_ref().and_then(|working| working.map.as_deref()),
        Some("Cool"),
        "confirmation must actually restart the load"
    );
    drop(task);
}

#[test]
fn the_button_on_a_card_finds_the_map_it_came_from() {
    let root = tmp("key");
    fs::create_dir_all(root.join("Folder Map")).unwrap();
    fs::write(root.join("Folder Map/Folder_P.upk"), vec![0u8; 64]).unwrap();
    fs::write(root.join("Loose_P.upk"), vec![0u8; 64]).unwrap();

    let library = library::scan(&root).maps;
    assert_eq!(library.len(), 2);

    let mut deck = RlDeck::boot().0;
    deck.library = library.clone();

    for map in &library {
        let card = model::from_library(map, None);
        let found = deck.find_map(&card.key).unwrap_or_else(|| panic!("{} has a card that leads nowhere", map.name));

        assert_eq!(found.primary, map.primary);
    }
}

#[test]
fn escape_closes_only_the_topmost_transient_state() {
    let mut deck = RlDeck::boot().0;
    deck.pending = Some(Pending { key: "/library/Cool".to_string(), name: "Cool".to_string(), bytes: 42 });
    deck.chooser = true;
    deck.detail = Some((0, 0));
    deck.menu_open = true;
    deck.armed = Some("Cool".to_string());

    let _ = deck.update(Message::Escape);
    assert!(deck.pending.is_none());
    assert!(deck.chooser, "the chooser remains behind the confirmation");

    let _ = deck.update(Message::Escape);
    assert!(!deck.chooser);
    assert!(deck.detail.is_some(), "the detail remains behind the chooser");

    let _ = deck.update(Message::Escape);
    assert!(deck.detail.is_none());
    assert!(deck.menu_open, "the menu remains behind the detail");

    let _ = deck.update(Message::Escape);
    assert!(!deck.menu_open);
    assert!(deck.armed.is_some(), "the armed action is the last thing dismissed");

    let _ = deck.update(Message::Escape);
    assert!(deck.armed.is_none());
}

#[test]
fn an_answer_about_a_folder_we_have_left_is_ignored() {
    let mut deck = RlDeck::boot().0;
    deck.game_dir = Some(PathBuf::from("/games/second"));
    deck.loaded_map = Some("Cool".to_string());

    let _ = deck.update(Message::GameState(PathBuf::from("/games/first"), Some(install::State::Original)));
    assert_eq!(deck.loaded_map.as_deref(), Some("Cool"), "a slow answer about the previous install must not overwrite the current one");

    let _ = deck.update(Message::GameState(PathBuf::from("/games/second"), Some(install::State::Loaded("Rings".to_string()))));
    assert_eq!(deck.loaded_map.as_deref(), Some("Rings"));
}

#[test]
fn a_second_click_on_download_does_not_start_a_second_download() {
    let mut deck = RlDeck::boot().0;
    deck.catalog = vec![serde_json::from_str(r#"{"name":"Triple Goal","download_url":"https://x/1.zip"}"#).unwrap()];

    let _ = deck.update(Message::Act("Triple Goal".to_string()));
    assert!(deck.busy.contains("Triple Goal"), "the first click starts it");

    deck.notice = None;
    let _ = deck.update(Message::Act("Triple Goal".to_string()));
    assert_eq!(deck.notice, None, "the second click is simply ignored");
}

#[test]
fn a_catalog_entry_with_no_link_says_so_instead_of_hanging() {
    let mut deck = RlDeck::boot().0;
    deck.catalog = vec![serde_json::from_str(r#"{"name":"Bare"}"#).unwrap()];

    let _ = deck.update(Message::Act("Bare".to_string()));

    assert!(!deck.busy.contains("Bare"));
    assert_eq!(deck.notice.as_deref(), Some("Bare has no download link"));
}

#[test]
fn repair_will_not_run_on_top_of_a_load_already_in_flight() {
    let root = tmp("busy");
    fs::create_dir_all(install::maps_dir(&root)).unwrap();

    let mut deck = RlDeck::boot().0;
    deck.game_dir = Some(root.clone());
    deck.record.backup = Some(root.join("backup.upk"));
    deck.working = Some(Working { map: Some("Cool".to_string()), label: "loading Cool".to_string() });

    let _ = deck.update(Message::Repair);

    assert_eq!(deck.notice.as_deref(), Some("Still loading Cool. One at a time"));
    assert_eq!(deck.working.as_ref().map(|w| w.label.as_str()), Some("loading Cool"), "the load in flight must not be replaced");
}

fn shelved(name: &str, saved: Option<u64>) -> library::Map {
    library::Map {
        name: name.to_string(),
        folder: Some(PathBuf::from(format!("/library/{name}"))),
        primary: PathBuf::from(format!("/library/{name}/{name}_P.upk")),
        bytes: 1024,
        saved: saved.map(|secs| UNIX_EPOCH + std::time::Duration::from_secs(secs)),
        ..library::Map::default()
    }
}

fn shown(deck: &RlDeck) -> Vec<String> {
    deck.cards().into_iter().map(|card| card.name).collect()
}

#[test]
fn the_library_is_ordered_by_when_maps_arrived() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Zebra Crossing", Some(300)), shelved("alpha rings", Some(100)), shelved("Middle Ground", Some(200))];

    deck.shelf = Shelf::Newest;
    assert_eq!(shown(&deck), ["Zebra Crossing", "Middle Ground", "alpha rings"]);

    deck.shelf = Shelf::Oldest;
    assert_eq!(shown(&deck), ["alpha rings", "Middle Ground", "Zebra Crossing"]);
}

#[test]
fn a_map_with_no_date_sorts_last_in_both_directions() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Undated", None), shelved("Old", Some(100)), shelved("New", Some(900))];

    deck.shelf = Shelf::Newest;
    assert_eq!(shown(&deck), ["New", "Old", "Undated"]);

    deck.shelf = Shelf::Oldest;
    assert_eq!(shown(&deck), ["Old", "New", "Undated"]);
}

#[test]
fn maps_saved_in_the_same_second_stay_in_a_stable_order() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Charlie", Some(500)), shelved("alpha", Some(500)), shelved("Bravo", Some(500))];

    for shelf in [Shelf::Newest, Shelf::Oldest] {
        deck.shelf = shelf;
        assert_eq!(shown(&deck), ["alpha", "Bravo", "Charlie"], "{shelf}");
    }
}

#[test]
fn searching_still_works_whichever_order_is_chosen() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Speed Rings", Some(1)), shelved("Dribble", Some(2))];
    deck.query = "rings".to_string();

    for shelf in [Shelf::Newest, Shelf::Oldest] {
        deck.shelf = shelf;
        assert_eq!(shown(&deck), ["Speed Rings"], "{shelf} lost the filter");
    }
}

#[test]
fn the_starred_shelf_contains_only_starred_maps_in_newest_order() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Old Star", Some(100)), shelved("Not Starred", Some(300)), shelved("New Star", Some(200))];
    for name in ["Old Star", "New Star"] {
        let key = deck.library.iter().find(|map| map.name == name).unwrap().key();
        deck.config.toggle_star(&key);
    }

    deck.shelf = Shelf::Starred;
    assert_eq!(shown(&deck), ["New Star", "Old Star"]);
}

#[test]
fn a_map_already_in_the_library_is_not_offered_again_in_explore() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Explore;
    deck.library = vec![shelved("triple goal", Some(1))];
    deck.catalog =
        ["Triple Goal", "Banjo Bridge"].iter().map(|name| serde_json::from_str(&format!(r#"{{"name":"{name}"}}"#)).unwrap()).collect();

    assert_eq!(shown(&deck), ["Banjo Bridge"], "matched without regard to case");
}

#[test]
fn a_manual_import_matching_the_catalog_gets_its_known_credit() {
    let mut maps = vec![shelved("Haunted Hallows", Some(1))];
    let catalog = vec![
        serde_json::from_str(
            r#"{"name":"Haunted Hallows Escape","description_short":"Spooky &amp; polished","description":"A seasonal arena"}"#,
        )
        .unwrap(),
    ];

    reconcile_catalog_metadata(&mut maps, &catalog);

    assert_eq!(maps[0].author.as_deref(), Some("Lethamyr"));
    assert_eq!(maps[0].blurb.as_deref(), Some("Spooky & polished"));
    assert_eq!(maps[0].description.as_deref(), Some("A seasonal arena"));
}

#[test]
fn an_ambiguous_short_import_title_is_not_misattributed() {
    let mut maps = vec![shelved("Speed Training", Some(1))];
    let catalog = ["Speed Training Rings", "Speed Training Dribble"]
        .iter()
        .map(|name| serde_json::from_str(&format!(r#"{{"name":"{name}"}}"#)).unwrap())
        .collect::<Vec<_>>();

    reconcile_catalog_metadata(&mut maps, &catalog);

    assert_eq!(maps[0].author, None);
}

#[test]
fn the_in_game_map_is_the_primary_library_card() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Library;
    deck.library = vec![shelved("Newest", Some(300)), shelved("In Game", Some(100)), shelved("Middle", Some(200))];
    deck.loaded_map = Some("In Game".to_string());

    assert_eq!(shown(&deck), ["In Game", "Newest", "Middle"]);
}

#[test]
fn explore_can_show_only_starred_catalog_maps() {
    let mut deck = RlDeck::boot().0;
    deck.tab = Tab::Explore;
    deck.catalog =
        ["Triple Goal", "Banjo Bridge"].iter().map(|name| serde_json::from_str(&format!(r#"{{"name":"{name}"}}"#)).unwrap()).collect();
    deck.config.toggle_star(&catalog_star_key("  BANJO BRIDGE "));

    deck.sort = Sort::Starred;
    assert_eq!(shown(&deck), ["Banjo Bridge"]);
}

#[test]
fn an_idle_window_asks_for_no_frames() {
    let mut deck = RlDeck::boot().0;
    assert!(!deck.spinning(), "a fresh deck is doing nothing");

    deck.busy.insert("Some Map".to_string());
    assert!(deck.spinning(), "a download turns its card's spinner");

    deck.busy.clear();
    assert!(!deck.spinning());

    deck.working = Some(Working { map: None, label: "importing".to_string() });
    assert!(deck.spinning());

    deck.finished();
    assert!(!deck.spinning(), "finishing must stop the clock");
}

#[test]
fn the_spinner_turns_at_the_same_speed_whatever_the_refresh_rate() {
    let start = std::time::Instant::now();

    let mut sixty = RlDeck::boot().0;
    let mut hundred_and_twenty = RlDeck::boot().0;

    for frame in 1..=60 {
        let at = start + std::time::Duration::from_secs_f32(frame as f32 / 60.0);
        let _ = sixty.update(Message::Framed(at));
    }
    for frame in 1..=120 {
        let at = start + std::time::Duration::from_secs_f32(frame as f32 / 120.0);
        let _ = hundred_and_twenty.update(Message::Framed(at));
    }

    let apart = (sixty.spin - hundred_and_twenty.spin).abs();
    assert!(apart < 0.2, "a 120Hz panel spun to {} and a 60Hz one to {}", hundred_and_twenty.spin, sixty.spin);
}

#[test]
fn the_angle_wraps_instead_of_growing_forever() {
    let start = std::time::Instant::now();
    let mut deck = RlDeck::boot().0;

    for frame in 1..=600 {
        let at = start + std::time::Duration::from_secs_f32(frame as f32 / 60.0);
        let _ = deck.update(Message::Framed(at));
    }

    assert!(deck.spin >= 0.0 && deck.spin < std::f32::consts::TAU, "ten seconds of spinning left the angle at {}", deck.spin);
}

#[test]
fn a_long_gap_between_frames_does_not_jump_the_spinner() {
    let start = std::time::Instant::now();
    let mut deck = RlDeck::boot().0;

    let _ = deck.update(Message::Framed(start));
    let after_first = deck.spin;

    let _ = deck.update(Message::Framed(start + std::time::Duration::from_secs(30)));
    let step = deck.spin - after_first;

    assert!(step > 0.0 && step < 0.5, "a 30 second gap advanced the spinner by {step} radians");
}

#[test]
fn duplicate_blurb_and_description_are_only_shown_once() {
    assert!(!has_distinct_body(Some("Aerial practice"), Some(" aerial practice ")));
    assert!(has_distinct_body(Some("Short summary"), Some("Longer explanation")));
    assert!(has_distinct_body(None, Some("Only body")));
    assert!(!has_distinct_body(Some("Only blurb"), None));
}

#[test]
fn retrying_the_catalog_immediately_returns_to_a_loading_state() {
    let mut deck = RlDeck::boot().0;
    deck.catalog_state = Loading::Failed("offline".to_string());

    let _ = deck.update(Message::RetryCatalog);

    assert!(matches!(deck.catalog_state, Loading::Busy));
}

#[test]
fn dense_local_cards_keep_a_readable_minimum_width() {
    let available = MIN_WINDOW.width - S4 * 2.0;
    let columns = ((available + S3) / (CARD_W + S3)).ceil();
    let cell = (available - S3 * (columns - 1.0)) / columns;

    assert!(cell >= 220.0, "the minimum window would squeeze local cards to {cell}px");
}
