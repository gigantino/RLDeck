use iced::widget::{container, svg};
use iced::{Center, Element, Length, Radians, Rotation};

fn ink(color: iced::Color) -> String {
    let (r, g, b) = ((color.r * 255.0).round() as u8, (color.g * 255.0).round() as u8, (color.b * 255.0).round() as u8);

    format!("{r:02x}{g:02x}{b:02x}")
}

fn drawing(path: &str, color: iced::Color, filled: bool) -> svg::Handle {
    let ink = ink(color);
    let fill = if filled { format!("#{ink}") } else { "none".to_string() };

    svg::Handle::from_memory(
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="{fill}"
                 stroke="#{ink}" stroke-width="2"
                 stroke-linecap="round" stroke-linejoin="round">{path}</svg>"##
        )
        .into_bytes(),
    )
}

fn mark<'a>(path: &str, size: f32, color: iced::Color, filled: bool) -> svg::Svg<'a> {
    svg(drawing(path, color, filled)).width(Length::Fixed(size)).height(Length::Fixed(size))
}

fn inline<'a, Message: 'a>(path: &str, size: f32, color: iced::Color) -> Element<'a, Message> {
    mark(path, size, color, false).into()
}

fn glyph<'a, Message: 'a>(path: &str, size: f32, color: iced::Color) -> Element<'a, Message> {
    container(mark(path, size, color, false)).width(Length::Fill).height(Length::Fill).align_x(Center).align_y(Center).into()
}

pub fn chevron_left<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    glyph(r#"<path d="m15 18-6-6 6-6"/>"#, size, color)
}

pub fn chevron_right<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    glyph(r#"<path d="m9 18 6-6-6-6"/>"#, size, color)
}

pub fn close<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    glyph(r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#, size, color)
}

pub fn settings<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    glyph(r#"<path d="M14 17H5"/><path d="M19 7h-9"/><circle cx="17" cy="17" r="3"/><circle cx="7" cy="7" r="3"/>"#, size, color)
}

const DOWNLOAD: &str = r#"<path d="M12 15V3"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/>"#;
const IMPORT: &str = r#"<path d="M12 3v12"/><path d="m17 8-5-5-5 5"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>"#;
const FOLDER_OPEN: &str = r#"<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>"#;
const FOLDER_INPUT: &str = r#"<path d="M2 9V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-1"/><path d="M2 13h10"/><path d="m9 16 3-3-3-3"/>"#;
const REPAIR: &str = r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#;
const EXTERNAL: &str =
    r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#;
const LOAD: &str = r#"<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"/>"#;
const TRASH: &str = r#"<path d="M10 11v6"/><path d="M14 11v6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#;
const CHECK: &str = r#"<path d="M20 6 9 17l-5-5"/>"#;
const ALERT: &str =
    r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#;
const SPINNER: &str = r#"<path d="M21 12a9 9 0 1 1-6.219-8.56"/>"#;
const STAR: &str = r#"<path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.12 2.12 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.12 2.12 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.12 2.12 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.12 2.12 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.12 2.12 0 0 0 1.597-1.16z"/>"#;

pub fn download<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(DOWNLOAD, size, color)
}

pub fn import<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(IMPORT, size, color)
}

pub fn folder_open<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(FOLDER_OPEN, size, color)
}

pub fn folder_input<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(FOLDER_INPUT, size, color)
}

pub fn repair<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(REPAIR, size, color)
}

pub fn external<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(EXTERNAL, size, color)
}

pub fn load<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(LOAD, size, color)
}

pub fn trash<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(TRASH, size, color)
}

pub fn check<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(CHECK, size, color)
}

pub fn star<'a, Message: 'a>(size: f32, color: iced::Color, filled: bool) -> Element<'a, Message> {
    mark(STAR, size, color, filled).into()
}

pub fn alert<'a, Message: 'a>(size: f32, color: iced::Color) -> Element<'a, Message> {
    inline(ALERT, size, color)
}

pub fn spinner<'a, Message: 'a>(size: f32, color: iced::Color, turn: f32) -> Element<'a, Message> {
    mark(SPINNER, size, color, false).rotation(Rotation::Floating(Radians(turn))).into()
}
