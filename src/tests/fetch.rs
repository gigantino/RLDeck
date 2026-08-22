use super::*;
use crate::testing::scratch as tmp;
use zip::write::SimpleFileOptions;

fn build_zip(path: &Path, entries: &[(&str, usize)]) {
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    for (name, size) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&vec![0u8; *size]).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn unpacks_a_multi_file_map() {
    let dir = tmp("multi");
    let archive = dir.join("map.zip");
    build_zip(
        &archive,
        &[("CoolMap/CoolMap_P.upk", 512), ("CoolMap/CoolMap_Textures.upk", 256), ("CoolMap/preview.png", 64), ("CoolMap/info.json", 32)],
    );

    let library = dir.join("library");
    let folder = install_into_library(&archive, "Cool Map", &library, &Progress::default()).unwrap();

    assert!(folder.join("CoolMap_P.upk").exists());
    assert!(folder.join("CoolMap_Textures.upk").exists());
    assert!(folder.join("preview.png").exists());
    assert!(folder.join("info.json").exists());
}

#[test]
fn a_traversal_entry_cannot_escape_the_library() {
    let dir = tmp("slip");
    let archive = dir.join("evil.zip");
    build_zip(&archive, &[("../../../../evil.upk", 16), ("real/Map_P.upk", 64)]);

    let library = dir.join("library");
    let folder = install_into_library(&archive, "Evil", &library, &Progress::default()).unwrap();

    assert!(folder.join("Map_P.upk").exists());
    assert!(!dir.join("evil.upk").exists() && !library.join("evil.upk").exists(), "nothing may be written outside the map folder");
    for entry in fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        assert_eq!(path.parent(), Some(folder.as_path()), "{} climbed out of the map folder", path.display());
    }
    assert!(fs::read_dir(&folder).unwrap().count() <= 2);
}

#[test]
fn two_downloads_at_once_do_not_write_to_the_same_file() {
    let library = Path::new("/maps");

    let one = staging_for(library, "Triple Goal", "https://drive/a");
    let two = staging_for(library, "Banjo Bridge", "https://drive/b");
    let same_name = staging_for(library, "Triple Goal", "https://drive/c");

    assert_ne!(one, two);
    assert_ne!(one, same_name, "a name can point at more than one file");
    assert_eq!(one, staging_for(library, "Triple Goal", "https://drive/a"));
    assert!(one.starts_with(library.join(STAGING)));
}

#[test]
fn two_entries_with_the_same_name_do_not_overwrite_each_other() {
    let dir = tmp("dupes");
    let archive = dir.join("map.zip");
    build_zip(&archive, &[("v2/Map_P.upk", 4096), ("v1/Map_P.upk", 128), ("v1/art.png", 64)]);

    let watched = Progress::default();
    let folder = install_into_library(&archive, "Dupes", &dir.join("library"), &watched).unwrap();

    assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
    assert_eq!(fs::metadata(folder.join("Map_P.upk")).unwrap().len(), 4096, "the first entry with that name is the one that lands");
    assert_eq!(watched.fraction(), Some(1.0), "the bar must not count bytes that were never written");
}

#[test]
fn a_refused_archive_leaves_no_empty_map_in_the_library() {
    let dir = tmp("nostub");
    let library = dir.join("library");
    let archive = dir.join("nope.zip");
    build_zip(&archive, &[("readme.txt", 16)]);

    assert!(install_into_library(&archive, "Nope", &library, &Progress::default()).is_err());
    assert!(!library.join("Nope").exists(), "a failed download must not leave a map with nothing in it");
    assert!(archive.exists(), "and it must not eat the archive either");
}

#[test]
fn a_failed_second_download_does_not_delete_the_copy_that_works() {
    let dir = tmp("keepgood");
    let library = dir.join("library");

    let good = dir.join("good.zip");
    build_zip(&good, &[("Cool_P.upk", 512)]);
    let folder = install_into_library(&good, "Cool", &library, &Progress::default()).unwrap();
    assert!(folder.join("Cool_P.upk").exists());

    let junk = dir.join("junk.zip");
    build_zip(&junk, &[("readme.txt", 16)]);
    assert!(install_into_library(&junk, "Cool", &library, &Progress::default()).is_err());

    assert!(folder.join("Cool_P.upk").exists(), "the map already in the library must survive a failed re-download");
}

#[test]
fn a_bare_download_is_moved_rather_than_copied_and_left_behind() {
    let dir = tmp("moved");
    let staged = dir.join("staging");
    fs::create_dir_all(&staged).unwrap();

    let file = staged.join("download.part");
    fs::write(&file, vec![7u8; 256]).unwrap();

    let folder = install_into_library(&file, "Loose", &dir.join("library"), &Progress::default()).unwrap();

    assert_eq!(fs::read(folder.join("Loose.upk")).unwrap().len(), 256);
    assert!(!file.exists(), "the staging copy must not survive");
}

#[test]
fn junk_files_in_the_archive_are_left_out() {
    let dir = tmp("junk");
    let archive = dir.join("map.zip");
    build_zip(&archive, &[("Map_P.upk", 64), ("readme.txt", 16), ("install.exe", 16), ("__MACOSX/._Map_P.upk", 8)]);

    let folder = install_into_library(&archive, "Map", &dir.join("library"), &Progress::default()).unwrap();

    assert!(folder.join("Map_P.upk").exists());
    assert!(!folder.join("readme.txt").exists());
    assert!(!folder.join("install.exe").exists(), "no executables land in the library");
}

#[test]
fn an_archive_with_no_map_is_an_error() {
    let dir = tmp("empty");
    let archive = dir.join("nope.zip");
    build_zip(&archive, &[("readme.txt", 16)]);

    assert!(matches!(install_into_library(&archive, "Nope", &dir.join("library"), &Progress::default()), Err(Error::Archive(_))));
}

#[test]
fn a_bare_upk_download_becomes_a_map_folder() {
    let dir = tmp("bare");
    let file = dir.join("download.part");
    fs::write(&file, vec![0u8; 128]).unwrap();

    let folder = install_into_library(&file, "Loose Map", &dir.join("library"), &Progress::default()).unwrap();
    assert!(folder.join("Loose Map.upk").exists());
}

#[test]
fn a_downloaded_map_gets_the_catalog_artwork_and_credits() {
    let dir = tmp("extras");
    let archive = dir.join("map.zip");
    build_zip(&archive, &[("OnlyLevel_P.upk", 256)]);

    let library = dir.join("library");
    let folder = install_into_library(&archive, "Only Level", &library, &Progress::default()).unwrap();

    write_extras(
        &folder,
        "Only Level",
        &Extras {
            author: Some("Lethamyr".into()),
            description: Some("A map".into()),
            artwork: Some(vec![0xFF, 0xD8, 0xFF, 0xE0]),
            ..Default::default()
        },
    );

    assert!(folder.join("preview.jpg").exists(), "archive had no image of its own");
    let info = fs::read_to_string(folder.join("info.json")).unwrap();
    assert!(info.contains("Lethamyr"));
}

#[test]
fn artwork_inside_the_archive_wins_over_the_catalog_thumbnail() {
    let dir = tmp("ownart");
    let archive = dir.join("map.zip");
    build_zip(&archive, &[("Map_P.upk", 128), ("shot.png", 64)]);

    let folder = install_into_library(&archive, "Map", &dir.join("library"), &Progress::default()).unwrap();
    write_extras(&folder, "Map", &Extras { artwork: Some(vec![0xFF, 0xD8, 0xFF, 0xE0]), ..Default::default() });

    assert!(folder.join("shot.png").exists());
    assert!(!folder.join("preview.jpg").exists(), "don't override the map's own art");
}

#[tokio::test]
#[ignore = "downloads a real map from lethamyr.com"]
async fn downloads_a_real_map_end_to_end() {
    let dir = tmp("live");
    let library = dir.join("library");

    let entries = crate::catalog::fetch_all().await.expect("catalog");
    let entry = entries.iter().find(|e| e.name == "Triple Goal").expect("Triple Goal in catalog");

    let name = get_map(entry.name.clone(), entry.download_url.clone().expect("download url"), library.clone(), Extras::default())
        .await
        .expect("download and install");

    let scan = crate::library::scan(&library);
    println!("installed {name}: {} map(s)", scan.maps.len());

    let map = scan.maps.first().expect("a map in the library");
    println!("  primary: {}", map.primary.display());
    println!("  files:   {}", map.file_count());
    println!("  bytes:   {}", map.bytes);

    assert_eq!(map.name, "Triple Goal");
    assert!(map.bytes > 100_000, "the map should be real, got {}", map.bytes);
    assert!(!library.join(".staging").exists(), "staging must be cleaned up");
}

#[test]
fn names_that_would_break_windows_are_cleaned() {
    assert_eq!(sanitize("Map: The <Sequel>"), "Map_ The _Sequel_");
    assert_eq!(sanitize("  ..  "), "map");
}

#[test]
fn several_loose_files_import_as_one_multi_file_map() {
    let root = tmp("import-group");
    let source = root.join("downloads");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&library).unwrap();

    for (name, size) in [("Whack_A_Mole_P.upk", 4096), ("Whack_A_Mole_Textures.upk", 2048), ("Whack_A_Mole_Meshes.upk", 1024)] {
        fs::write(source.join(name), vec![0u8; size]).unwrap();
    }

    let files: Vec<PathBuf> =
        ["Whack_A_Mole_P.upk", "Whack_A_Mole_Textures.upk", "Whack_A_Mole_Meshes.upk"].iter().map(|n| source.join(n)).collect();

    let folder = import_group(&files, &library, &Progress::default()).unwrap();

    assert_eq!(folder.file_name().unwrap(), "Whack_A_Mole");
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 3);
    assert!(source.join("Whack_A_Mole_P.upk").exists(), "the user's own copy must stay where it was");
}

#[test]
fn the_level_names_the_map_even_when_it_is_not_the_first_file() {
    let root = tmp("import-level");
    let source = root.join("downloads");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&library).unwrap();

    fs::write(source.join("Aaa_Textures.upk"), vec![0u8; 8192]).unwrap();
    fs::write(source.join("Cool_Map_P.upk"), vec![0u8; 512]).unwrap();

    let files = vec![source.join("Aaa_Textures.upk"), source.join("Cool_Map_P.upk")];
    let folder = import_group(&files, &library, &Progress::default()).unwrap();

    assert_eq!(folder.file_name().unwrap(), "Cool_Map");
}

#[test]
fn with_no_p_suffix_the_biggest_package_names_the_map() {
    let root = tmp("import-biggest");
    let source = root.join("downloads");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&library).unwrap();

    fs::write(source.join("Small.upk"), vec![0u8; 128]).unwrap();
    fs::write(source.join("TheLevel.upk"), vec![0u8; 65536]).unwrap();

    let files = vec![source.join("Small.upk"), source.join("TheLevel.upk")];
    assert_eq!(import_group(&files, &library, &Progress::default()).unwrap().file_name().unwrap(), "TheLevel");
}

#[test]
fn a_second_map_of_the_same_name_gets_its_own_folder() {
    let root = tmp("import-clash");
    let source = root.join("downloads");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&library).unwrap();

    fs::write(source.join("Dribble_P.upk"), vec![0u8; 512]).unwrap();
    let files = vec![source.join("Dribble_P.upk")];

    let first = import_group(&files, &library, &Progress::default()).unwrap();
    let second = import_group(&files, &library, &Progress::default()).unwrap();

    assert_ne!(first, second);
    assert_eq!(second.file_name().unwrap(), "Dribble (2)");
}

#[test]
fn importing_a_folder_takes_the_map_and_leaves_the_rest() {
    let root = tmp("import-folder");
    let source = root.join("Speed Jump Ring");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(source.join("Screenshots")).unwrap();
    fs::create_dir_all(&library).unwrap();

    fs::write(source.join("Speed_Jump_Ring_P.upk"), vec![0u8; 4096]).unwrap();
    fs::write(source.join("Speed_Jump_Ring_SF.upk"), vec![0u8; 2048]).unwrap();
    fs::write(source.join("preview.jpg"), vec![0u8; 64]).unwrap();
    fs::write(source.join("readme.txt"), b"hello").unwrap();

    let folder = import_folder(&source, &library, &Progress::default()).unwrap();

    assert!(folder.join("Speed_Jump_Ring_P.upk").exists());
    assert!(folder.join("Speed_Jump_Ring_SF.upk").exists());
    assert!(folder.join("preview.jpg").exists());
    assert!(!folder.join("readme.txt").exists(), "junk stays out of the library");
    assert!(source.join("readme.txt").exists(), "the original folder is untouched");
}

#[test]
fn a_folder_with_no_map_in_it_is_refused_without_leaving_a_stub() {
    let root = tmp("import-empty");
    let source = root.join("Holiday Photos");
    let library = root.join("library");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&library).unwrap();
    fs::write(source.join("beach.jpg"), vec![0u8; 64]).unwrap();

    assert!(import_folder(&source, &library, &Progress::default()).is_err());
    assert_eq!(fs::read_dir(&library).unwrap().count(), 0, "a refused import must not leave an empty map behind");
}

#[test]
fn importing_an_archive_keeps_the_users_zip() {
    let root = tmp("import-zip");
    let library = root.join("library");
    fs::create_dir_all(&library).unwrap();

    let zip = root.join("Cool Map.zip");
    build_zip(&zip, &[("Cool_P.upk", 2048), ("Cool_T.upk", 512)]);

    let folder = import_file(&zip, &library, &Progress::default()).unwrap();

    assert!(folder.join("Cool_P.upk").exists());
    assert!(folder.join("Cool_T.upk").exists());
    assert!(zip.exists(), "importing must not consume the file it was given");
}
