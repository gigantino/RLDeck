use super::*;

#[test]
fn two_installs_keep_separate_records() {
    let mut config = Config::default();

    let steam = PathBuf::from(r"D:\SteamLibrary\steamapps\common\rocketleague");
    let epic = PathBuf::from(r"C:\Program Files\Epic Games\rocketleague");

    config.set_record(&steam, Record { original_sha256: Some("aaa".into()), ..Record::default() });
    config.set_record(&epic, Record { original_sha256: Some("bbb".into()), ..Record::default() });

    assert_eq!(config.record(&steam).original_sha256.as_deref(), Some("aaa"));
    assert_eq!(config.record(&epic).original_sha256.as_deref(), Some("bbb"));
}

#[test]
fn an_unknown_install_starts_with_a_blank_record() {
    let config = Config::default();
    assert!(config.record(Path::new("/nowhere")).original_sha256.is_none());
}

#[test]
fn a_saved_config_reads_back_the_same() {
    let mut config =
        Config { game_dir: Some(PathBuf::from("/games/rocketleague")), library_dir: Some(PathBuf::from("/maps")), ..Config::default() };
    config.set_record(
        Path::new("/games/rocketleague"),
        Record {
            original_sha256: Some("deadbeef".into()),
            original_bytes: 4_120_000,
            backup: Some(PathBuf::from("/backups/deadbeef.upk")),
            loaded: None,
        },
    );

    let raw = serde_json::to_string(&config).unwrap();
    let back: Config = serde_json::from_str(&raw).unwrap();

    assert_eq!(back.game_dir, config.game_dir);
    assert_eq!(back.library_dir, config.library_dir);
    assert_eq!(back.record(Path::new("/games/rocketleague")).original_bytes, 4_120_000);
}

#[test]
fn a_config_from_an_older_build_still_loads() {
    let older = r#"{"game_dir":"/games/rocketleague"}"#;
    let config: Config = serde_json::from_str(older).unwrap();
    assert_eq!(config.game_dir, Some(PathBuf::from("/games/rocketleague")));
    assert!(!config.is_starred("/maps/Rings"));
}

#[test]
fn stars_round_trip_and_toggle_cleanly() {
    let mut config = Config::default();
    let key = "/maps/Speed Rings".to_string();

    assert!(config.toggle_star(&key));
    assert!(config.is_starred(&key));

    let raw = serde_json::to_string(&config).unwrap();
    let mut back: Config = serde_json::from_str(&raw).unwrap();
    assert!(back.is_starred(&key));
    assert!(!back.toggle_star(&key));
    assert!(!back.is_starred(&key));
}
