use super::*;

#[test]
fn reads_every_steam_library_including_other_drives() {
    let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;
    let libs = parse_library_folders(vdf);
    assert_eq!(
        libs,
        vec![PathBuf::from(r"C:\Program Files (x86)\Steam"), PathBuf::from(r"D:\SteamLibrary"),],
        "a D: drive install is the case that sent people to the issue tracker"
    );
}

#[test]
fn windows_install_identity_ignores_case_and_slash_direction() {
    let registry = Path::new(r"C:\Program Files (x86)\Steam\steamapps\common\rocketleague");
    let vdf = Path::new(r"c:/program files (x86)/steam/steamapps/common/rocketleague");
    assert_eq!(windows_path_key(registry), windows_path_key(vdf));
}

#[test]
fn ignores_non_path_keys() {
    let vdf = r#"
	"contentstatsid"		"-123456"
	"path"		"E:\\Games"
"#;
    assert_eq!(parse_library_folders(vdf), vec![PathBuf::from(r"E:\Games")]);
}

#[test]
fn reads_an_epic_manifest() {
    let json = r#"{
        "DisplayName": "Rocket League",
        "InstallLocation": "C:\\Program Files\\Epic Games\\rocketleague",
        "AppName": "Sugar"
    }"#;

    let install = parse_epic_manifest(json).unwrap();
    assert_eq!(install.launcher, Launcher::Epic);
    assert_eq!(install.root, PathBuf::from(r"C:\Program Files\Epic Games\rocketleague"));
}

#[test]
fn skips_epic_manifests_for_other_games() {
    let json = r#"{"DisplayName":"Fortnite","InstallLocation":"C:\\x"}"#;
    assert!(parse_epic_manifest(json).is_none());
}

#[test]
fn only_counts_folders_that_actually_hold_the_game() {
    let root = crate::testing::scratch("game");

    let real = root.join("lib/steamapps/common/rocketleague");
    std::fs::create_dir_all(real.join("TAGame/CookedPCConsole")).unwrap();

    let decoy = root.join("empty");
    std::fs::create_dir_all(decoy.join("steamapps/common/rocketleague")).unwrap();

    let found = steam_installs_from_libraries(&[root.join("lib"), decoy]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].root, real);
}

#[test]
fn picking_the_folder_maps_get_pasted_into_still_finds_the_install() {
    let root = crate::testing::scratch("resolve");

    let game = root.join("steamapps/common/rocketleague");
    let cooked = game.join("TAGame/CookedPCConsole");
    std::fs::create_dir_all(&cooked).unwrap();

    assert_eq!(resolve_root(&cooked).as_deref(), Some(game.as_path()));
    assert_eq!(resolve_root(&game.join("TAGame")).as_deref(), Some(game.as_path()));
    assert_eq!(resolve_root(&game).as_deref(), Some(game.as_path()));
}

#[test]
fn picking_the_folder_above_the_install_finds_it_too() {
    let root = crate::testing::scratch("resolve-down");

    let game = root.join("common/rocketleague");
    std::fs::create_dir_all(game.join("TAGame/CookedPCConsole")).unwrap();

    assert_eq!(resolve_root(&root.join("common")).as_deref(), Some(game.as_path()));
}

#[test]
fn a_folder_with_no_game_in_it_is_refused() {
    let root = crate::testing::scratch("resolve-no");
    std::fs::create_dir_all(root.join("Documents")).unwrap();

    assert_eq!(resolve_root(&root), None, "a wrong folder must not be accepted quietly");
}
