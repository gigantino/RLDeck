use super::*;

fn entry(json: &str) -> catalog::Entry {
    serde_json::from_str(json).unwrap()
}

#[test]
fn a_catalog_card_carries_no_markup_into_the_view() {
    let card = from_catalog(&entry(r#"{"name":"Rings","description_short":"Fly &amp; land<br>fast"}"#), 3);

    assert_eq!(card.blurb.as_deref(), Some("Fly & land\nfast"));
    assert_eq!(card.catalog_index, Some(3));
    assert_eq!(card.source, Source::Catalog);
}

#[test]
fn a_card_with_no_picture_asks_the_gallery_for_nothing() {
    let bare = from_catalog(&entry(r#"{"name":"Bare"}"#), 0);
    assert_eq!(bare.art, Art::None);
    assert_eq!(bare.art_key(), None);

    let art = from_catalog(&entry(r#"{"name":"Shot","media":["https://art/1.jpg","https://art/2.jpg"]}"#), 0);
    assert_eq!(art.art_key().as_deref(), Some("https://art/1.jpg"));
}

#[test]
fn a_local_card_is_keyed_by_what_deleting_it_would_take() {
    let map = library::Map {
        name: "Cool".to_string(),
        folder: Some(PathBuf::from("/library/Cool")),
        primary: PathBuf::from("/library/Cool/Cool_P.upk"),
        image: Some(PathBuf::from("/library/Cool/preview.png")),
        ..library::Map::default()
    };

    let card = from_library(&map, Some("Cool"));

    assert_eq!(card.key, "/library/Cool");
    assert!(card.loaded, "the map named as loaded is the one in the game");
    assert_eq!(card.art_key().as_deref(), Some("/library/Cool/preview.png"));
    assert_eq!(card.blurb, None, "local cards show the author instead");
}

#[test]
fn search_looks_at_the_name_and_the_author() {
    let card = from_library(
        &library::Map { name: "Speed Rings".to_string(), author: Some("Lethamyr".to_string()), ..library::Map::default() },
        None,
    );

    assert!(card.matches(&Query::new("")), "an empty box hides nothing");
    assert!(card.matches(&Query::new("rings")));
    assert!(card.matches(&Query::new("speed r")));
    assert!(card.matches(&Query::new("  LETHAMYR ")), "typed case does not matter");
    assert!(!card.matches(&Query::new("dribble")));
}

#[test]
fn the_tabs_and_orderings_are_labelled_for_the_pickers() {
    assert_eq!(Tab::ALL.map(Tab::label), ["Library", "Explore"]);
    assert_eq!(Shelf::default(), Shelf::Newest);
    assert_eq!(Shelf::ALL.map(|s| s.to_string()), ["Newest", "Oldest", "Starred"]);
    assert_eq!(Sort::default(), Sort::MostLiked);
    assert_eq!(Sort::ALL.map(|s| s.to_string()), ["Most liked", "Most downloaded", "Newest", "Starred"]);
}
