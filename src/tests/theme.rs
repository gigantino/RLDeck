use super::*;

fn spread(color: Color) -> u8 {
    let channels = [color.r, color.g, color.b].map(|c| (c * 255.0).round() as u8);
    channels.iter().max().unwrap() - channels.iter().min().unwrap()
}

fn luminance(color: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
    };

    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(a: Color, b: Color) -> f32 {
    let (light, dark) = if luminance(a) > luminance(b) { (luminance(a), luminance(b)) } else { (luminance(b), luminance(a)) };

    (light + 0.05) / (dark + 0.05)
}

#[test]
fn the_neutral_ramp_stays_neutral() {
    for (name, color) in [
        ("BG", BG),
        ("SURFACE", SURFACE),
        ("SURFACE_HI", SURFACE_HI),
        ("BORDER", BORDER),
        ("BORDER_HI", BORDER_HI),
        ("TEXT", TEXT),
        ("TEXT_DIM", TEXT_DIM),
        ("TEXT_FAINT", TEXT_FAINT),
        ("ACCENT", ACCENT),
        ("ACCENT_HI", ACCENT_HI),
    ] {
        assert!(spread(color) <= 9, "{name} has a {} point channel spread, which is a tint, not a grey", spread(color));
    }
}

#[test]
fn the_ramp_gets_lighter_in_order() {
    let luma = |c: Color| c.r + c.g + c.b;
    let ramp = [BG, SURFACE, SURFACE_HI, BORDER, BORDER_HI, TEXT_FAINT, TEXT_DIM, ACCENT, TEXT];

    for pair in ramp.windows(2) {
        assert!(luma(pair[0]) < luma(pair[1]), "the ramp must increase in lightness at every step");
    }
}

#[test]
fn only_the_warm_accents_carry_hue() {
    assert!(spread(AUTHOR) > 20);
    assert!(spread(EMBER) > 20);
}

#[test]
fn every_small_text_tier_is_readable_on_the_lightest_surface() {
    for (name, color) in [("TEXT_FAINT", TEXT_FAINT), ("TEXT_DIM", TEXT_DIM), ("TEXT", TEXT), ("AUTHOR", AUTHOR), ("DANGER", DANGER)] {
        assert!(contrast(color, SURFACE_HI) >= 4.5, "{name} must keep 4.5:1 contrast on raised surfaces");
    }
}

#[test]
fn a_window_edge_notice_does_not_draw_a_second_rectangle() {
    let style = notice(&base());
    assert_eq!(style.background, None);
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.radius, Radius::default());
}

#[test]
fn card_actions_and_icon_actions_share_one_hover_language() {
    let theme = base();
    let card = card_button(false)(&theme, button::Status::Hovered);
    let icon = card_icon_button(&theme, button::Status::Hovered);

    assert_eq!(card.background, icon.background);
    assert_eq!(card.text_color, icon.text_color);
    assert_eq!(card.border, icon.border);
}

#[test]
fn dropdown_surface_and_popup_share_the_zinc_palette() {
    let theme = base();
    let closed = picker(&theme, pick_list::Status::Active);
    let hovered = picker(&theme, pick_list::Status::Hovered);
    let opened = picker(&theme, pick_list::Status::Opened { is_hovered: true });
    let popup = picker_menu(&theme);

    assert_eq!(closed.background, Background::Color(SURFACE_HI));
    assert_eq!(closed.border.color, BORDER);
    assert_eq!(hovered.border.color, BORDER_HI);
    assert_eq!(opened.border.color, ACCENT);
    assert_eq!(popup.background, Background::Color(SURFACE_HI));
    assert_eq!(popup.selected_background, Background::Color(SURFACE));
    assert_eq!(popup.border.radius, Radius::new(R_CTRL));
}

#[test]
fn panel_icon_buttons_are_rounded_rectangles_with_real_borders() {
    let style = icon_button(&base(), button::Status::Active);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, BORDER);
    assert_eq!(style.border.radius, Radius::new(R_CTRL));
}

#[test]
fn a_selected_star_uses_the_existing_warm_accent() {
    let style = star_button(true)(&base(), button::Status::Active);
    assert_eq!(style.text_color, EMBER);
    assert_eq!(style.border.width, 1.0);
    assert_ne!(style.border.color, BORDER);
}

#[test]
fn search_hover_and_focus_are_distinct_from_rest() {
    let theme = base();
    let rest = search(&theme, text_input::Status::Active);
    let hover = search(&theme, text_input::Status::Hovered);
    let focus = search(&theme, text_input::Status::Focused { is_hovered: true });

    assert_eq!(rest.border.color, BORDER);
    assert_eq!(hover.border.color, BORDER_HI);
    assert_eq!(focus.background, Background::Color(BG));
    assert_eq!(focus.border.color, ACCENT);
}

#[test]
fn menu_row_actions_have_a_dark_resting_surface() {
    let style = menu_control_button(&base(), button::Status::Active);
    assert_eq!(style.background, Some(Background::Color(BG)));
    assert_eq!(style.border.color, BORDER);
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn the_in_game_card_is_a_stronger_surface_without_a_white_outline() {
    let style = card(false, true)(&base());
    assert_eq!(style.background, Some(Background::Color(SURFACE_MAIN)));
    assert_eq!(style.border.color, BORDER_HI);
    assert_ne!(style.border.color, ACCENT);
}

#[test]
fn spacing_and_rounding_follow_one_component_hierarchy() {
    assert_eq!([S1, S2, S3, S4, S5], [4.0, 8.0, 12.0, 16.0, 24.0]);
    assert_eq!(R_CTRL, S2);
    assert_eq!(R_CARD, S3);
    assert_eq!(R_PANEL, S4);
    assert_eq!(R_IN_CARD, inner_radius(R_CARD, PAD_CARD));
    assert_eq!(R_IN_MENU, inner_radius(R_CTRL, PAD_MENU));
}
