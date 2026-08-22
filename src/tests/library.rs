use super::*;
use crate::testing::scratch as tmp;
use std::fs::File;
use std::io::Write;

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = File::create(path).unwrap();
    f.write_all(&vec![0u8; bytes]).unwrap();
}

#[test]
fn the_persistent_suffix_is_matched_and_stripped_the_same_way() {
    assert_eq!(strip_persistent_suffix("Underpass_P"), "Underpass");
    assert_eq!(strip_persistent_suffix("Underpass_p"), "Underpass");
    assert_eq!(strip_persistent_suffix("Underpass"), "Underpass");
    assert!(is_persistent(Path::new("Underpass_p.upk")));
    assert!(!is_persistent(Path::new("Underpass.upk")));
}

#[test]
fn a_multibyte_name_is_not_sliced_down_the_middle() {
    assert_eq!(strip_persistent_suffix("Café"), "Café");
    assert_eq!(strip_persistent_suffix("Café_P"), "Café");
    assert_eq!(strip_persistent_suffix("é"), "é");
    assert_eq!(strip_persistent_suffix(""), "");
}

#[test]
fn a_maps_key_names_what_deleting_it_would_take() {
    let root = tmp("key");
    write(&root.join("Folder Map/Folder_P.upk"), 64);
    write(&root.join("Dropped.upk"), 64);

    for map in scan(&root).maps {
        assert_eq!(map.key(), map.home().to_string_lossy());
        match &map.folder {
            Some(dir) => assert_eq!(map.home(), dir),
            None => assert_eq!(map.home(), map.primary),
        }
    }
}

#[test]
fn deleting_a_map_removes_its_folder_and_nothing_else() {
    let root = tmp("del");
    write(&root.join("Doomed/Doomed_P.upk"), 256);
    write(&root.join("Doomed/preview.png"), 32);
    write(&root.join("Keeper/Keeper_P.upk"), 256);

    let scan = scan(&root);
    let doomed = scan.maps.iter().find(|m| m.name == "Doomed").unwrap();
    remove(doomed, &root).unwrap();

    assert!(!root.join("Doomed").exists());
    assert!(root.join("Keeper/Keeper_P.upk").exists());
    assert!(root.exists(), "the library itself survives");
}

#[test]
fn deleting_a_loose_map_takes_the_file_not_the_library() {
    let root = tmp("delloose");
    write(&root.join("Dropped.upk"), 128);
    write(&root.join("Other/Other_P.upk"), 128);

    let scan = scan(&root);
    let loose = scan.maps.iter().find(|m| m.name == "Dropped").unwrap();
    assert!(loose.folder.is_none(), "a bare file in the root owns no folder");
    assert_eq!(loose.home(), loose.primary, "so deleting it takes the file");

    remove(loose, &root).unwrap();

    assert!(!root.join("Dropped.upk").exists());
    assert!(root.exists(), "the library must not be deleted with it");
    assert!(root.join("Other/Other_P.upk").exists(), "other maps must be untouched");
}

#[test]
fn a_map_outside_the_library_is_refused() {
    let root = tmp("delout");
    let elsewhere = tmp("delsomewhere");
    write(&elsewhere.join("Stray/Stray_P.upk"), 64);
    fs::create_dir_all(&root).unwrap();

    let stray = scan(&elsewhere).maps.remove(0);
    assert!(remove(&stray, &root).is_err());
    assert!(elsewhere.join("Stray/Stray_P.upk").exists(), "nothing deleted");
}

#[test]
fn the_library_folder_is_never_the_thing_deleted() {
    let root = tmp("delroot");
    write(&root.join("Only.upk"), 64);

    let mut map = scan(&root).maps.remove(0);
    map.folder = Some(root.clone());

    assert!(remove(&map, &root).is_err(), "a map claiming the library root");
    assert!(root.exists());
}

#[test]
fn finds_single_file_map_in_a_folder() {
    let root = tmp("single");
    write(&root.join("Cool Map/CoolMap.upk"), 2048);

    let scan = scan(&root);
    assert_eq!(scan.maps.len(), 1);
    assert_eq!(scan.maps[0].name, "Cool Map");
    assert_eq!(scan.maps[0].file_count(), 1);
    assert!(scan.skipped.is_empty());
}

#[test]
fn accepts_a_bare_upk_in_the_library_root() {
    let root = tmp("loose");
    write(&root.join("Dropped Map.upk"), 1024);

    let scan = scan(&root);
    assert_eq!(scan.maps.len(), 1);
    assert_eq!(scan.maps[0].name, "Dropped Map");
    assert_eq!(scan.maps[0].bytes, 1024);
}

#[test]
fn multi_file_map_keeps_every_package() {
    let root = tmp("multi");
    write(&root.join("Big/Big_Textures.upk"), 9000);
    write(&root.join("Big/Big_P.upk"), 4000);
    write(&root.join("Big/Big_Meshes.upk"), 7000);

    let scan = scan(&root);
    let map = &scan.maps[0];

    assert!(map.primary.ends_with("Big_P.upk"));
    assert_eq!(map.file_count(), 3);
    assert_eq!(map.bytes, 20_000);
    assert_eq!(map.extras.len(), 2, "both packages travel with the level");
}

#[test]
fn without_a_p_suffix_the_largest_package_is_the_level() {
    let root = tmp("largest");
    write(&root.join("Set/extra.upk"), 100);
    write(&root.join("Set/level.upk"), 9000);

    let scan = scan(&root);
    assert!(scan.maps[0].primary.ends_with("level.upk"));
}

#[test]
fn a_downloaded_map_keeps_its_catalog_details() {
    let root = tmp("details");
    write(&root.join("Rich/Rich_P.upk"), 256);
    fs::write(
        root.join("Rich/info.json"),
        r#"{"name":"Rich Map","author":"Lethamyr","blurb":"Short one",
            "description":"The long story.","settings":"Boost: Unlimited",
            "source":"https://lethamyr.com/maps/7"}"#,
    )
    .unwrap();

    let map = &scan(&root).maps[0];
    assert_eq!(map.blurb.as_deref(), Some("Short one"));
    assert_eq!(map.description.as_deref(), Some("The long story."));
    assert_eq!(map.settings.as_deref(), Some("Boost: Unlimited"));
    assert_eq!(map.source.as_deref(), Some("https://lethamyr.com/maps/7"));
}

#[test]
fn a_legacy_info_file_still_reads() {
    let root = tmp("legacy");
    write(&root.join("Old/Old.upk"), 128);
    fs::write(root.join("Old/info.json"), r#"{"author":"Someone","desc":"A short line"}"#).unwrap();

    let map = &scan(&root).maps[0];
    assert_eq!(map.author.as_deref(), Some("Someone"));
    assert_eq!(map.blurb.as_deref(), Some("A short line"));
    assert_eq!(map.name, "Old", "no recorded title, so the folder name");
}

#[test]
fn a_recorded_title_beats_the_sanitised_folder_name() {
    let root = tmp("title");
    write(&root.join("Speed Training_ Whack a Mole/Leth.udk"), 512);
    fs::write(root.join("Speed Training_ Whack a Mole/info.json"), r#"{"name":"Speed Training: Whack a Mole","author":"Lethamyr"}"#)
        .unwrap();

    let map = &scan(&root).maps[0];
    assert_eq!(map.name, "Speed Training: Whack a Mole");
}

#[test]
fn a_folder_with_a_dot_in_its_name_keeps_the_whole_name() {
    let root = tmp("dotted");
    write(&root.join("Map v1.2/Level_P.upk"), 128);

    assert_eq!(scan(&root).maps[0].name, "Map v1.2");
}

#[test]
fn without_a_recorded_title_the_folder_name_stands() {
    let root = tmp("notitle");
    write(&root.join("Plain Map/Plain.upk"), 128);

    assert_eq!(scan(&root).maps[0].name, "Plain Map");
}

#[test]
fn reads_legacy_info_json_and_artwork() {
    let root = tmp("meta");
    write(&root.join("Meta/Meta.upk"), 512);
    fs::write(root.join("Meta/info.json"), r#"{"author":"Lethamyr","desc":"A map"}"#).unwrap();
    write(&root.join("Meta/preview.png"), 64);

    let map = &scan(&root).maps[0];
    assert_eq!(map.author.as_deref(), Some("Lethamyr"));
    assert_eq!(map.description.as_deref(), Some("A map"));
    assert!(map.image.is_some());
}

#[test]
fn a_broken_info_json_costs_only_its_own_metadata() {
    let root = tmp("badinfo");
    write(&root.join("Broken/Broken.upk"), 512);
    fs::write(root.join("Broken/info.json"), "{not json at all").unwrap();
    write(&root.join("Fine/Fine.upk"), 512);

    let scan = scan(&root);
    assert_eq!(scan.maps.len(), 2, "the whole library must still load");
    let broken = scan.maps.iter().find(|m| m.name == "Broken").unwrap();
    assert_eq!(broken.author, None);
}

#[test]
fn folders_without_maps_are_ignored_not_reported() {
    let root = tmp("empty");
    fs::create_dir_all(root.join("Screenshots")).unwrap();
    write(&root.join("Real/Real.upk"), 128);

    let scan = scan(&root);
    assert_eq!(scan.maps.len(), 1);
    assert!(scan.skipped.is_empty());
}

#[test]
fn a_scanned_map_knows_when_it_arrived() {
    let root = tmp("dated");
    write(&root.join("Fresh/Fresh_P.upk"), 100);

    let scan = scan(&root);
    let saved = scan.maps[0].saved.expect("the filesystem should date a folder we just made");

    let age = std::time::SystemTime::now().duration_since(saved).expect("a folder created a moment ago cannot be in the future");

    assert!(age.as_secs() < 60, "a map made just now should read as made just now");
}
