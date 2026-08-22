use super::*;

#[test]
fn extensions_are_matched_however_they_are_capitalised() {
    assert!(is_map("Underpass_P.UPK"));
    assert!(is_map(Path::new("a/b/Level.udk")));
    assert!(is_image("shot.JPEG"));
    assert!(!is_map("readme.txt"));
    assert!(!is_map("noextension"));
}

#[test]
fn only_maps_artwork_and_info_survive_an_archive() {
    assert!(worth_extracting("Map_P.upk"));
    assert!(worth_extracting("preview.png"));
    assert!(worth_extracting("info.json"));
    assert!(!worth_extracting("install.exe"));
    assert!(!worth_extracting("readme.txt"));
}

#[test]
fn a_path_with_no_file_name_still_has_something_to_call_it() {
    assert_eq!(name_of("/maps/Cool/Cool_P.upk"), "Cool_P.upk");
    assert_eq!(name_of("/maps/Cool/.."), "/maps/Cool/..");
    assert_eq!(stem_of("/maps/Cool_P.upk").as_deref(), Some("Cool_P"));
}

#[test]
fn a_missing_file_counts_as_no_bytes_rather_than_failing() {
    assert_eq!(bytes_at("/nowhere/at/all.upk"), 0);
    assert_eq!(total_bytes([Path::new("/nowhere/at/all.upk")]), 0);
}
