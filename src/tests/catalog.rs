use super::*;

const FIXTURE: &str = include_str!("../../tests/fixtures/lethamyr-page1.json");

#[test]
fn parses_a_real_response() {
    let page = parse_page(FIXTURE).unwrap();

    assert!(!page.data.is_empty());
    assert!(page.meta.last_page > 1, "paging is what fetch_all reads, and the real catalog has more than one page");

    let first = &page.data[0];
    assert!(!first.name.is_empty());
    assert!(first.thumbnail().is_some_and(|t| t.starts_with("https://")));
}

#[test]
fn an_entry_without_a_download_url_is_not_an_error() {
    let entry: Entry = serde_json::from_str(r#"{"name":"Bare"}"#).unwrap();
    assert_eq!(entry.name, "Bare");
    assert_eq!(entry.download_url, None);
    assert_eq!(entry.thumbnail(), None);
}

#[test]
fn strips_the_markup_the_api_actually_sends() {
    let raw = "Disguise your car!</p><p class=\"\" style=\"white-space:pre-wrap;\">Second para.";
    assert_eq!(plain_text(raw), "Disguise your car!\nSecond para.");
}

#[test]
fn line_breaks_survive_as_line_breaks() {
    assert_eq!(plain_text("one<br>two<br/>three"), "one\ntwo\nthree");
}

#[test]
fn decodes_the_entities_that_appear() {
    assert_eq!(plain_text("Rock &amp; Roll &#39;22"), "Rock & Roll '22");
}

#[test]
fn nothing_executable_survives() {
    let hostile = "<script>alert('x')</script>hi<img src=x onerror=alert(1)>";
    let out = plain_text(hostile);
    assert!(!out.contains('<'), "no tags: {out}");
    assert!(!out.contains("onerror"), "no attributes: {out}");
    assert!(out.contains("hi"));
}

#[test]
fn unclosed_tags_do_not_swallow_everything_or_panic() {
    assert_eq!(plain_text("text <unclosed"), "text");
    assert_eq!(plain_text("a & b"), "a & b");
}

#[test]
fn plain_descriptions_are_untouched() {
    let plain = "Each team has 3 goals!";
    assert_eq!(plain_text(plain), plain);
}

#[test]
fn pulls_links_out_of_a_real_description() {
    let chunks = linkify("Run it at https://www.speedrun.com/ipt/ for the leaderboard.");

    assert_eq!(
        chunks,
        vec![Chunk::Text("Run it at "), Chunk::Link("https://www.speedrun.com/ipt/"), Chunk::Text(" for the leaderboard."),]
    );
}

#[test]
fn trailing_punctuation_is_not_part_of_the_link() {
    let chunks = linkify("see https://speedrun.com/ipi.");
    assert_eq!(chunks[1], Chunk::Link("https://speedrun.com/ipi"));
    assert_eq!(chunks[2], Chunk::Text("."));
}

#[test]
fn bare_www_links_are_found_and_given_a_scheme() {
    let chunks = linkify("go to www.example.com now");
    assert_eq!(chunks[1], Chunk::Link("www.example.com"));
    assert_eq!(absolute("www.example.com"), "https://www.example.com");
    assert_eq!(absolute("https://x.com"), "https://x.com");
}

#[test]
fn text_without_links_stays_one_chunk() {
    let chunks = linkify("Each team has 3 goals!");
    assert_eq!(chunks, vec![Chunk::Text("Each team has 3 goals!")]);
}

#[test]
fn reads_the_recommended_settings_off_a_real_page() {
    let html = include_str!("../../tests/fixtures/lethamyr-map-page.html");
    let settings = parse_settings(html).expect("the page has a settings block");

    assert!(settings.contains("Boost"), "expected the boost line, got {settings:?}");
    assert!(!settings.contains('<'), "markup must not survive");
    assert!(!settings.contains("window."), "page scripts must not be mistaken for settings");
}

#[test]
fn a_link_inside_the_settings_keeps_its_destination() {
    let html = r#"<h3>Recommended Settings</h3><p>Needs
        <a href="https://bakkesmod.com/">BakkesMod</a> and boost: unlimited</p>"#;

    let settings = parse_settings(html).expect("settings block");
    assert!(settings.contains("bakkesmod.com"), "the destination must survive stripping: {settings:?}");

    let links: Vec<_> = linkify(&settings)
        .into_iter()
        .filter_map(|c| match c {
            Chunk::Link(url) => Some(url),
            Chunk::Text(_) => None,
        })
        .collect();
    assert_eq!(links, vec!["https://bakkesmod.com/"]);
}

#[test]
fn stylesheet_links_do_not_leak_into_recommended_settings() {
    let html = r#"<h3>Recommended Settings</h3><p>Boost: Unlimited</p>
        <link href="https://fonts.googleapis.com/css2?family=Poppins">"#;

    let settings = parse_settings(html).expect("settings block");
    assert_eq!(settings, "Boost: Unlimited");
}

#[test]
fn single_quoted_and_bare_hrefs_are_read_too() {
    assert_eq!(href_in("a href='https://x.com/a' target=_blank"), Some("https://x.com/a".to_string()));
    assert_eq!(href_in("a href=https://y.com"), Some("https://y.com".to_string()));
    assert_eq!(href_in("p class=\"x\""), None);
}

#[test]
fn a_multibyte_page_is_not_cut_down_the_middle_of_a_character() {
    let filler = "\u{2014}".repeat(400);
    let html = format!("<h3>Recommended Settings</h3><p>Boost: unlimited</p>{filler}");

    let settings = parse_settings(&html).expect("settings block");
    assert!(settings.contains("Boost: unlimited"));
}

#[test]
fn the_cut_never_lands_inside_a_character() {
    let text = "a\u{2014}b\u{00e9}c";

    for at in 0..=text.len() + 4 {
        let cut = floor_boundary(text, at);
        assert!(text.is_char_boundary(cut), "{at} floored to {cut}");
        assert!(cut <= at.min(text.len()));
        let _ = &text[..cut];
    }
}

#[test]
fn an_uppercase_href_is_read_like_any_other() {
    assert_eq!(href_in(r#"A HREF="https://x.com/a" TARGET=_blank"#), Some("https://x.com/a".to_string()));
    assert_eq!(href_in(r"a Href='https://y.com'"), Some("https://y.com".to_string()));
}

#[test]
fn a_tag_carrying_multibyte_text_does_not_shift_the_href() {
    let tag = "a title=\"Caf\u{00e9} \u{2014} best map\" href=\"https://z.com/caf\u{00e9}\"";
    assert_eq!(href_in(tag), Some("https://z.com/caf\u{00e9}".to_string()));
}

#[test]
fn a_page_without_the_block_yields_nothing() {
    assert_eq!(parse_settings("<p>Just a description.</p>"), None);
}

#[test]
fn page_titles_are_matched_without_the_site_suffix() {
    assert!(title_matches("Triple Goal - Lethamyr", "Triple Goal"));
    assert!(title_matches("Banjo Bridge - Lethamyr", "banjo bridge"));
    assert!(!title_matches("Capture the Flag Retro - Lethamyr", "Banjo Bridge"));
}

#[test]
fn cache_round_trips() {
    let dir = crate::testing::scratch("catalog");
    let entries = parse_page(FIXTURE).unwrap().data;

    save(&dir, &entries, 1_700_000_000).unwrap();
    let back = load(&dir).unwrap();

    assert_eq!(back.entries.len(), entries.len());
    assert_eq!(back.fetched_unix, 1_700_000_000);
}
