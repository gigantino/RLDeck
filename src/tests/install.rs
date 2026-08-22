use super::*;
use std::io::Write;

fn write(path: &Path, byte: u8, len: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::File::create(path).unwrap().write_all(&vec![byte; len]).unwrap();
}

struct Rig {
    game: PathBuf,
    backups: PathBuf,
    maps: PathBuf,
}

fn rig(label: &str) -> Rig {
    let root = crate::testing::scratch(label);
    let rig = Rig { game: root.join("rocketleague"), backups: root.join("backups"), maps: root.join("library") };
    fs::create_dir_all(maps_dir(&rig.game)).unwrap();
    fs::create_dir_all(&rig.maps).unwrap();
    rig
}

fn stock(rig: &Rig, len: usize) {
    write(&maps_dir(&rig.game).join(TARGET), 0x11, len);
}

fn custom_map(rig: &Rig, name: &str, extras: &[&str]) -> Map {
    let dir = rig.maps.join(name);
    let primary = dir.join(format!("{name}_P.upk"));
    write(&primary, 0xAA, 4096);

    let extras: Vec<PathBuf> = extras
        .iter()
        .map(|e| {
            let p = dir.join(e);
            write(&p, 0xBB, 512);
            p
        })
        .collect();

    Map { name: name.to_string(), bytes: 4096 + 512 * extras.len() as u64, folder: Some(dir), primary, extras, ..Map::default() }
}

#[test]
fn the_progress_total_covers_every_byte_the_bar_will_count() {
    let rig = rig("progresstotal");
    stock(&rig, 8192);
    write(&maps_dir(&rig.game).join("Shared.upk"), 0xCC, 2048);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let map = custom_map(&rig, "First", &["Shared.upk"]);
    let progress = Progress::default();
    install(&mut record, &map, &rig.game, &rig.backups, &progress).unwrap();
    assert_eq!(progress.done(), progress.total(), "parking the file already in the slot is work the bar has to allow for");

    let progress = Progress::default();
    restore(&mut record, &rig.game, &progress).unwrap();
    assert_eq!(progress.done(), progress.total(), "putting the displaced file back goes through the same counter");
}

#[test]
fn a_four_megabyte_original_is_fine() {
    let rig = rig("bigoriginal");
    stock(&rig, 4_120_000);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    assert_eq!(state(&record, &rig.game).unwrap(), State::Original);
    assert_eq!(record.original_bytes, 4_120_000);
}

#[test]
fn install_then_restore_returns_the_exact_original() {
    let rig = rig("roundtrip");
    stock(&rig, 4_120_000);
    let before = hash::of_file(&maps_dir(&rig.game).join(TARGET)).unwrap();

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let map = custom_map(&rig, "Cool", &[]);
    install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).unwrap();
    assert_eq!(state(&record, &rig.game).unwrap(), State::Loaded("Cool".into()));

    restore(&mut record, &rig.game, &Progress::default()).unwrap();
    assert_eq!(hash::of_file(&maps_dir(&rig.game).join(TARGET)).unwrap(), before);
    assert_eq!(state(&record, &rig.game).unwrap(), State::Original);
}

#[test]
fn multi_file_maps_copy_and_clean_up_every_package() {
    let rig = rig("multi");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let map = custom_map(&rig, "Big", &["Big_Textures.upk", "Big_Meshes.upk"]);
    install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).unwrap();

    assert!(dir.join("Big_Textures.upk").exists());
    assert!(dir.join("Big_Meshes.upk").exists());

    restore(&mut record, &rig.game, &Progress::default()).unwrap();
    assert!(!dir.join("Big_Textures.upk").exists());
    assert!(!dir.join("Big_Meshes.upk").exists(), "restore must leave no litter");
}

#[test]
fn switching_maps_does_not_accumulate_packages() {
    let rig = rig("switch");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    install(&mut record, &custom_map(&rig, "First", &["First_Tex.upk"]), &rig.game, &rig.backups, &Progress::default()).unwrap();
    install(&mut record, &custom_map(&rig, "Second", &["Second_Tex.upk"]), &rig.game, &rig.backups, &Progress::default()).unwrap();

    assert!(!dir.join("First_Tex.upk").exists(), "previous map's files must go");
    assert!(dir.join("Second_Tex.upk").exists());
}

#[test]
fn a_custom_map_can_never_be_captured_as_the_original() {
    let rig = rig("noclobber");
    stock(&rig, 4_120_000);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();
    let original = record.original_sha256.clone();

    install(&mut record, &custom_map(&rig, "Cool", &[]), &rig.game, &rig.backups, &Progress::default()).unwrap();

    protect(&mut record, &rig.game, &rig.backups, true).unwrap();
    assert_eq!(record.original_sha256, original);

    restore(&mut record, &rig.game, &Progress::default()).unwrap();
    assert_eq!(state(&record, &rig.game).unwrap(), State::Original);
}

#[test]
fn an_unknown_file_asks_before_being_trusted() {
    let rig = rig("confirm");
    stock(&rig, 9_999_999);

    let mut record = Record::default();
    match protect(&mut record, &rig.game, &rig.backups, false) {
        Err(Error::NeedsConfirmation { bytes }) => assert_eq!(bytes, 9_999_999),
        other => panic!("expected a confirmation prompt, got {other:?}"),
    }
    assert!(record.original_sha256.is_none(), "nothing recorded until confirmed");
}

#[test]
fn a_game_update_reads_as_foreign_rather_than_original() {
    let rig = rig("update");
    stock(&rig, 4_120_000);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    write(&maps_dir(&rig.game).join(TARGET), 0x22, 4_500_000);
    assert_eq!(state(&record, &rig.game).unwrap(), State::Foreign);

    restore(&mut record, &rig.game, &Progress::default()).unwrap();
    assert_eq!(state(&record, &rig.game).unwrap(), State::Original);
}

#[test]
fn restoring_without_a_backup_is_an_error_not_a_crash() {
    let rig = rig("nobackup");
    stock(&rig, 4_120_000);

    let mut record = Record::default();
    assert!(matches!(restore(&mut record, &rig.game, &Progress::default()), Err(Error::NoBackup)));
}

#[test]
fn a_map_that_overwrites_a_game_file_puts_it_back() {
    let rig = rig("displace");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let game_file = dir.join("Startup.upk");
    write(&game_file, 0x77, 2048);
    let untouched = hash::of_file(&game_file).unwrap();

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let map = custom_map(&rig, "Rude", &["Startup.upk"]);
    install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).unwrap();
    assert_ne!(hash::of_file(&game_file).unwrap(), untouched, "the map's own package should be in place while it is loaded");

    restore(&mut record, &rig.game, &Progress::default()).unwrap();
    assert_eq!(hash::of_file(&game_file).unwrap(), untouched, "the game's file must come back byte for byte");
}

#[test]
fn a_displaced_game_file_survives_switching_maps() {
    let rig = rig("displace-switch");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let game_file = dir.join("Startup.upk");
    write(&game_file, 0x77, 2048);
    let untouched = hash::of_file(&game_file).unwrap();

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    install(&mut record, &custom_map(&rig, "One", &["Startup.upk"]), &rig.game, &rig.backups, &Progress::default()).unwrap();
    install(&mut record, &custom_map(&rig, "Two", &[]), &rig.game, &rig.backups, &Progress::default()).unwrap();

    assert_eq!(hash::of_file(&game_file).unwrap(), untouched, "loading a second map must restore what the first one displaced");
}

#[test]
fn a_failed_copy_leaves_no_half_written_level() {
    let rig = rig("atomic");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let mut map = custom_map(&rig, "Ghost", &[]);
    map.primary = rig.maps.join("does-not-exist.upk");

    assert!(install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).is_err());
    assert_eq!(state(&record, &rig.game).unwrap(), State::Original, "a failed load must leave the game exactly as it was");
    assert!(!atomic::part_of(&dir.join(TARGET)).exists(), "no scratch file left behind");
}

#[test]
fn a_load_that_fails_part_way_puts_the_game_folder_back() {
    let rig = rig("rollback");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let game_file = dir.join("Startup.upk");
    write(&game_file, 0x77, 2048);
    let untouched = hash::of_file(&game_file).unwrap();

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();
    let original = state(&record, &rig.game).unwrap();

    let mut map = custom_map(&rig, "Ghost", &["Startup.upk"]);
    map.primary = rig.maps.join("does-not-exist.upk");

    assert!(install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).is_err());

    assert_eq!(state(&record, &rig.game).unwrap(), original);
    assert!(record.loaded.is_none(), "nothing was loaded, so nothing is recorded");
    assert_eq!(hash::of_file(&game_file).unwrap(), untouched, "the game file the map displaced has to come back");
    assert!(!rig.backups.join("displaced-Startup.upk").exists(), "and its parked copy must not be left lying around");
}

#[test]
fn a_failed_load_leaves_no_package_of_its_own_behind() {
    let rig = rig("rollback-extras");
    stock(&rig, 4_120_000);
    let dir = maps_dir(&rig.game);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();

    let mut map = custom_map(&rig, "Ghost", &["Ghost_Tex.upk", "Ghost_Mesh.upk"]);
    map.primary = rig.maps.join("does-not-exist.upk");

    assert!(install(&mut record, &map, &rig.game, &rig.backups, &Progress::default()).is_err());

    assert!(!dir.join("Ghost_Tex.upk").exists());
    assert!(!dir.join("Ghost_Mesh.upk").exists(), "a failed load leaves no litter");
    assert_eq!(state(&record, &rig.game).unwrap(), State::Original);
}

#[test]
fn a_loaded_map_records_every_package_it_put_in_the_game_folder() {
    let rig = rig("recorded");
    stock(&rig, 4_120_000);

    let mut record = Record::default();
    protect(&mut record, &rig.game, &rig.backups, true).unwrap();
    install(&mut record, &custom_map(&rig, "Kept", &["Kept_Tex.upk"]), &rig.game, &rig.backups, &Progress::default()).unwrap();

    let loaded = record.loaded.clone().unwrap();
    assert_eq!(loaded.extras, vec!["Kept_Tex.upk".to_string()]);
}
