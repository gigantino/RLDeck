use super::*;

fn set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn pixel() -> Handle {
    Handle::from_rgba(1, 1, vec![0, 0, 0, 255])
}

fn urls(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("https://art/{i}.jpg")).collect()
}

#[test]
fn the_open_popup_keeps_its_screenshot_through_an_eviction() {
    let pinned = set(&[]);
    let open = set(&["shot-2.jpg"]);
    let window: HashSet<&str> = ["far-away.jpg"].into_iter().collect();

    assert!(survives("shot-2.jpg", &pinned, &open, &window), "scrolling behind a popup must not blank what it is showing");
}

#[test]
fn local_artwork_is_never_evicted() {
    let pinned = set(&["/maps/Cool/preview.jpg"]);
    let window: HashSet<&str> = HashSet::new();

    assert!(survives("/maps/Cool/preview.jpg", &pinned, &set(&[]), &window));
}

#[test]
fn artwork_outside_every_reason_is_dropped() {
    let window: HashSet<&str> = ["near.jpg"].into_iter().collect();

    assert!(survives("near.jpg", &set(&[]), &set(&[]), &window));
    assert!(!survives("stale.jpg", &set(&[]), &set(&[]), &window));
}

#[test]
fn fetching_starts_at_the_row_on_screen() {
    assert_eq!(focus_order(100, 0.5, 6), vec![48, 49, 50, 51, 52, 53]);
}

#[test]
fn the_window_reaches_above_the_estimated_row() {
    let order = focus_order(200, 0.5, 36);
    assert!(order.iter().any(|i| *i < 100), "nothing above the focus was queued: {:?}", &order[..5]);
}

#[test]
fn at_the_top_it_reads_forward_as_before() {
    assert_eq!(focus_order(100, 0.0, 4), vec![0, 1, 2, 3]);
}

#[test]
fn at_the_bottom_it_covers_the_last_rows() {
    let order = focus_order(100, 1.0, 4);
    assert!(order.contains(&99), "the final card must be in the window");
    assert!(order.iter().all(|i| *i > 90));
}

#[test]
fn the_window_never_runs_past_the_end() {
    assert_eq!(focus_order(3, 0.5, 10).len(), 3);
    assert!(focus_order(0, 0.5, 10).is_empty());
}

#[test]
fn every_card_is_reachable_by_scrolling() {
    let len = 60;
    let mut seen = vec![false; len];

    for step in 0..=20 {
        for i in focus_order(len, step as f32 / 20.0, 6) {
            seen[i] = true;
        }
    }

    assert!(seen.iter().all(|hit| *hit), "some cards were never queued");
}

#[test]
fn no_more_than_the_in_flight_limit_runs_at_once() {
    let mut gallery = Gallery::default();
    let all = urls(thumbs::MAX_IN_FLIGHT * 3);

    gallery.focus(&all, 0.0, &HashSet::new());
    let first = gallery.next_batch();

    assert_eq!(first.len(), thumbs::MAX_IN_FLIGHT);
    assert!(gallery.next_batch().is_empty(), "nothing more starts until one comes back");

    gallery.arrived(first[0].clone(), pixel());
    assert_eq!(gallery.next_batch().len(), 1, "a free slot starts one more");
}

#[test]
fn a_picture_that_arrived_is_never_fetched_again() {
    let mut gallery = Gallery::default();
    let all = urls(3);

    gallery.arrived(all[1].clone(), pixel());
    gallery.focus(&all, 0.0, &HashSet::new());

    assert!(!gallery.next_batch().contains(&all[1]));
    assert_eq!(gallery.prioritise(&all[1]), None);
}

#[test]
fn a_broken_link_is_retried_a_few_times_and_then_left_alone() {
    let mut gallery = Gallery::default();
    let url = "https://art/gone.jpg".to_string();

    for attempt in 1..MAX_ATTEMPTS {
        gallery.failed(url.clone(), thumbs::Retry::Worth);
        assert_eq!(gallery.next_batch(), vec![url.clone()], "attempt {attempt} should be queued again");
    }

    gallery.failed(url.clone(), thumbs::Retry::Worth);
    assert!(gallery.next_batch().is_empty(), "giving up is not a loop");
    assert!(!gallery.is_unavailable(&url), "it may still work next time");
}

#[test]
fn something_that_is_not_an_image_is_never_asked_for_again() {
    let mut gallery = Gallery::default();
    let url = "https://art/index.html".to_string();

    gallery.failed(url.clone(), thumbs::Retry::Pointless);

    assert!(gallery.is_unavailable(&url));
    assert!(gallery.next_batch().is_empty());
    assert_eq!(gallery.prioritise(&url), None);
    assert!(!gallery.any_missing(&[url], 0.0), "we are not waiting on it");
}

#[test]
fn scrolling_away_drops_what_was_queued_for_the_old_position() {
    let mut gallery = Gallery::default();
    let all = urls(400);

    gallery.focus(&all, 0.0, &HashSet::new());
    let top = gallery.next_batch();

    gallery.focus(&all, 1.0, &HashSet::new());
    let bottom = gallery.next_batch();

    assert!(bottom.iter().all(|url| !top.contains(url)), "the bar should be fetching the bottom of the list, not the top");
}

#[test]
fn far_away_pictures_are_dropped_but_pinned_ones_stay() {
    let mut gallery = Gallery::default();
    let all = urls(MAX_HANDLES * 3);

    for url in &all {
        gallery.arrived(url.clone(), pixel());
    }
    gallery.pinned.insert("/maps/Cool/preview.jpg".to_string());
    gallery.handles.insert("/maps/Cool/preview.jpg".to_string(), pixel());

    let open: HashSet<String> = [all[all.len() - 1].clone()].into_iter().collect();
    gallery.focus(&all, 0.0, &open);

    let held = gallery.tally().held;
    assert!(held <= MAX_HANDLES + 2, "the cap was not enforced, {held} handles held");
    assert!(gallery.get("/maps/Cool/preview.jpg").is_some(), "local art");
    assert!(gallery.get(&all[all.len() - 1]).is_some(), "the open popup");
    assert!(gallery.get(&all[0]).is_some(), "the top of the window");
}

#[test]
fn nothing_is_missing_once_everything_has_settled_one_way_or_the_other() {
    let mut gallery = Gallery::default();
    let all = urls(3);

    assert!(gallery.any_missing(&all, 0.0));

    gallery.arrived(all[0].clone(), pixel());
    gallery.arrived(all[1].clone(), pixel());
    gallery.failed(all[2].clone(), thumbs::Retry::Pointless);

    assert!(!gallery.any_missing(&all, 0.0));
    assert!(gallery.idle());
}
