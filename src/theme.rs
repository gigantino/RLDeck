use iced::border::Radius;
use iced::overlay::menu;
use iced::widget::{button, container, pick_list, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};

pub const INK: Color = rgb(0x09, 0x09, 0x0b);
pub const ZINC: Color = rgb(0xe4, 0xe4, 0xe7);
pub const AMBER: Color = rgb(0xf0, 0xa4, 0x50);
pub const RED: Color = rgb(0xf0, 0x72, 0x68);

pub const BG: Color = INK;
pub const SURFACE: Color = mix(INK, ZINC, 0.055);
pub const SURFACE_HI: Color = mix(INK, ZINC, 0.095);
pub const SURFACE_MAIN: Color = mix(INK, ZINC, 0.13);
pub const BORDER: Color = mix(INK, ZINC, 0.14);
pub const BORDER_HI: Color = mix(INK, ZINC, 0.24);

pub const TEXT_FAINT: Color = mix(INK, ZINC, 0.58);
pub const TEXT_DIM: Color = mix(INK, ZINC, 0.69);
pub const ACCENT: Color = mix(INK, ZINC, 0.88);
pub const TEXT: Color = mix(INK, ZINC, 0.98);
pub const ACCENT_HI: Color = ZINC;

pub const EMBER: Color = mix(INK, AMBER, 0.82);
pub const AUTHOR: Color = EMBER;
pub const DANGER: Color = mix(INK, RED, 0.9);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

const fn mix(from: Color, to: Color, k: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * k,
        g: from.g + (to.g - from.g) * k,
        b: from.b + (to.b - from.b) * k,
        a: from.a + (to.a - from.a) * k,
    }
}

pub fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

pub const S1: f32 = 4.0;
pub const S2: f32 = 8.0;
pub const S3: f32 = 12.0;
pub const S4: f32 = 16.0;
pub const S5: f32 = 24.0;

pub const T_XS: f32 = 12.0;
pub const T_SM: f32 = 13.0;
pub const T_MD: f32 = 14.0;
pub const T_LG: f32 = 16.0;
pub const T_XL: f32 = 20.0;

pub const I_SM: f32 = 14.0;
pub const I_MD: f32 = 16.0;
pub const I_LG: f32 = 18.0;
pub const SCROLL_RAIL: f32 = 6.0;
pub const SCROLL_THUMB: f32 = 3.0;

pub const FONT_MEDIUM: Font = Font { family: iced::font::Family::Name("Fira Sans"), weight: iced::font::Weight::Medium, ..Font::DEFAULT };

pub const fn line_of(size: f32) -> f32 {
    size * 1.35
}

pub const fn inner_radius(outer: f32, padding: f32) -> f32 {
    if outer > padding { outer - padding } else { 0.0 }
}

pub const R_CTRL: f32 = S2;
pub const R_CARD: f32 = S3;
pub const R_PANEL: f32 = S4;
pub const R_PILL: f32 = 999.0;

pub const PAD_CARD: f32 = 6.0;
pub const PAD_MENU: f32 = S1;
pub const PAD_PANEL: f32 = S4;

pub const R_IN_CARD: f32 = inner_radius(R_CARD, PAD_CARD);
pub const R_IN_MENU: f32 = inner_radius(R_CTRL, PAD_MENU);

fn edge(color: Color, radius: f32) -> Border {
    Border { color, width: 1.0, radius: Radius::new(radius) }
}

fn round(radius: f32) -> Border {
    Border { radius: Radius::new(radius), ..Default::default() }
}

fn shade(color: Color, drop: f32, blur: f32) -> Shadow {
    Shadow { color, offset: Vector::new(0.0, drop), blur_radius: blur }
}

fn plain(fill: Color) -> container::Style {
    container::Style { background: Some(Background::Color(fill)), ..Default::default() }
}

fn boxed(fill: Color, border: Border) -> container::Style {
    container::Style { border, ..plain(fill) }
}

fn raised(fill: Color, border: Border, shadow: Shadow) -> container::Style {
    container::Style { shadow, ..boxed(fill, border) }
}

fn pressable(fill: Color, text_color: Color, border: Border) -> button::Style {
    button::Style { background: Some(Background::Color(fill)), text_color, border, ..Default::default() }
}

fn engaged(status: button::Status) -> bool {
    matches!(status, button::Status::Hovered | button::Status::Pressed)
}

pub fn base() -> Theme {
    let palette = iced::theme::Palette { background: BG, text: TEXT, primary: ACCENT, success: ACCENT, warning: EMBER, danger: DANGER };

    Theme::custom("RLDeck".to_string(), palette)
}

pub fn top_bar(_: &Theme) -> container::Style {
    plain(SURFACE)
}

pub fn hairline(_: &Theme) -> container::Style {
    plain(BORDER)
}

pub fn backdrop(_: &Theme) -> container::Style {
    plain(alpha(INK, 0.72))
}

pub fn notice(_: &Theme) -> container::Style {
    container::Style::default()
}

pub fn tile(_: &Theme) -> container::Style {
    boxed(SURFACE_HI, edge(BORDER, R_IN_CARD))
}

pub fn inset(_: &Theme) -> container::Style {
    boxed(BG, edge(BORDER, R_CTRL))
}

pub fn warning(_: &Theme) -> container::Style {
    boxed(alpha(EMBER, 0.12), edge(alpha(EMBER, 0.26), R_CTRL))
}

pub fn panel(_: &Theme) -> container::Style {
    raised(SURFACE, edge(BORDER_HI, R_PANEL), shade(alpha(INK, 0.7), 18.0, 48.0))
}

pub fn menu(_: &Theme) -> container::Style {
    raised(SURFACE_HI, edge(BORDER_HI, R_CTRL), shade(alpha(INK, 0.6), 10.0, 30.0))
}

pub fn hint(_: &Theme) -> container::Style {
    raised(SURFACE_HI, edge(BORDER_HI, R_IN_CARD), shade(alpha(INK, 0.55), S1, S3))
}

pub fn card(hovered: bool, loaded: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let fill = match (loaded, hovered) {
            (true, _) => SURFACE_MAIN,
            (false, true) => SURFACE_HI,
            (false, false) => SURFACE,
        };
        let (ink, drop, blur) = if loaded { (0.62, S1, S4) } else { (0.28, 2.0, S2) };
        let line = if loaded || hovered { BORDER_HI } else { BORDER };

        raised(fill, edge(line, R_CARD), shade(alpha(INK, ink), drop, blur))
    }
}

fn framed(status: button::Status, resting_fill: Color, radius: f32) -> button::Style {
    let (fill, line, ink) = match status {
        button::Status::Pressed => (BG, BORDER_HI, TEXT),
        button::Status::Hovered => (SURFACE, BORDER_HI, TEXT),
        button::Status::Disabled => (resting_fill, BORDER, TEXT_FAINT),
        button::Status::Active => (resting_fill, BORDER, TEXT_DIM),
    };

    pressable(fill, ink, edge(line, radius))
}

pub fn icon_button(_: &Theme, status: button::Status) -> button::Style {
    framed(status, SURFACE_HI, R_CTRL)
}

pub fn menu_control_button(_: &Theme, status: button::Status) -> button::Style {
    framed(status, BG, R_IN_MENU)
}

pub fn dot(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let fill = match (active, status) {
            (true, _) => ACCENT_HI,
            (false, button::Status::Hovered) => BORDER_HI,
            (false, _) => BORDER,
        };

        pressable(fill, Color::TRANSPARENT, round(R_PILL))
    }
}

pub fn bare_button(_: &Theme, _status: button::Status) -> button::Style {
    button::Style { text_color: TEXT, ..Default::default() }
}

pub fn menu_item(_: &Theme, status: button::Status) -> button::Style {
    let hot = engaged(status);
    let fill = if hot { alpha(ACCENT, 0.16) } else { Color::TRANSPARENT };

    pressable(fill, if hot { TEXT } else { TEXT_DIM }, round(R_IN_MENU))
}

fn card_button_style(loaded: bool, status: button::Status) -> button::Style {
    let (fill, line) = match status {
        button::Status::Pressed => (BG, BORDER_HI),
        button::Status::Hovered => (SURFACE, BORDER_HI),
        _ if loaded => (BG, BORDER_HI),
        _ => (SURFACE_HI, BORDER),
    };

    pressable(fill, ACCENT, edge(line, R_IN_CARD))
}

pub fn card_button(loaded: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| card_button_style(loaded, status)
}

pub fn card_icon_button(_: &Theme, status: button::Status) -> button::Style {
    card_button_style(false, status)
}

pub fn star_button(starred: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if !starred {
            return card_button_style(false, status);
        }

        let hot = engaged(status);
        let line = alpha(EMBER, if hot { 0.9 } else { 0.62 });

        pressable(alpha(EMBER, if hot { 0.18 } else { 0.1 }), EMBER, edge(line, R_IN_CARD))
    }
}

fn ghost_style(status: button::Status, radius: f32) -> button::Style {
    let hot = engaged(status);
    let fill = match status {
        button::Status::Pressed => BG,
        _ if hot => SURFACE,
        _ => Color::TRANSPARENT,
    };
    let line = if hot { BORDER_HI } else { Color::TRANSPARENT };

    pressable(fill, if hot { TEXT } else { TEXT_DIM }, edge(line, radius))
}

pub fn ghost_button(_: &Theme, status: button::Status) -> button::Style {
    ghost_style(status, R_CTRL)
}

pub fn tab(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hot = engaged(status);
        let fill = match (selected, status) {
            (_, button::Status::Pressed) => BG,
            (true, _) => SURFACE_HI,
            (false, _) if hot => SURFACE,
            (false, _) => Color::TRANSPARENT,
        };
        let line = match (selected, hot) {
            (true, _) => BORDER_HI,
            (false, true) => BORDER,
            (false, false) => Color::TRANSPARENT,
        };

        pressable(fill, if selected || hot { TEXT } else { TEXT_DIM }, edge(line, R_CTRL))
    }
}

pub fn picker(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let open = matches!(status, pick_list::Status::Opened { .. });
    let hovered = matches!(status, pick_list::Status::Hovered);
    let (fill, line) = match (open, hovered) {
        (true, _) => (BG, ACCENT),
        (false, true) => (SURFACE, BORDER_HI),
        (false, false) => (SURFACE_HI, BORDER),
    };
    let ink = if open || hovered { TEXT } else { TEXT_DIM };

    pick_list::Style {
        text_color: ink,
        placeholder_color: TEXT_FAINT,
        handle_color: ink,
        background: Background::Color(fill),
        border: edge(line, R_CTRL),
    }
}

pub fn picker_menu(_: &Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(SURFACE_HI),
        border: edge(BORDER_HI, R_CTRL),
        text_color: TEXT_DIM,
        selected_text_color: TEXT,
        selected_background: Background::Color(SURFACE),
        shadow: shade(alpha(INK, 0.65), S2, S5),
    }
}

pub fn readable(_: &Theme, _status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: TEXT_FAINT,
        value: TEXT_DIM,
        selection: BORDER_HI,
    }
}

pub fn search(_: &Theme, status: text_input::Status) -> text_input::Style {
    let (fill, line) = match status {
        text_input::Status::Focused { .. } => (BG, ACCENT),
        text_input::Status::Hovered => (SURFACE_HI, BORDER_HI),
        text_input::Status::Disabled => (SURFACE, BORDER),
        text_input::Status::Active => (SURFACE_HI, BORDER),
    };

    text_input::Style {
        background: Background::Color(fill),
        border: edge(line, R_CTRL),
        icon: TEXT_FAINT,
        placeholder: TEXT_FAINT,
        value: TEXT,
        selection: alpha(ACCENT, 0.35),
    }
}

pub fn scroller(_: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller { background: Background::Color(BORDER_HI), border: round(R_PILL) },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(SURFACE_HI),
            border: edge(BORDER_HI, R_PILL),
            shadow: Shadow::default(),
            icon: TEXT_DIM,
        },
    }
}

#[cfg(test)]
#[path = "tests/theme.rs"]
mod tests;
