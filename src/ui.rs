use std::borrow::Borrow;
use std::time::Duration;

use iced::widget::image::Handle;
use iced::widget::text::LineHeight;
use iced::widget::{Container, Space, button, column, container, image, mouse_area, pick_list, row, scrollable, stack, text, tooltip};
use iced::{Center, Color, ContentFit, Element, Fill, Length, Theme};

use crate::icon;
use crate::theme::{self, I_MD, PAD_PANEL, S1, S2, S3, SCROLL_RAIL, SCROLL_THUMB, T_MD, T_SM, T_XL, T_XS};

pub const ICON_BUTTON_SIZE: f32 = 34.0;

pub fn icon_button<'a, Message: Clone + 'a>(
    mark: Element<'a, Message>,
    box_size: f32,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    on_press: Message,
) -> Element<'a, Message> {
    button(container(mark).width(Fill).height(Fill).center(Fill))
        .width(Length::Fixed(box_size))
        .height(Length::Fixed(box_size))
        .padding(0)
        .style(style)
        .on_press(on_press)
        .into()
}

pub fn close_button<'a, Message: Clone + 'a>(
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Message,
) -> Element<'a, Message> {
    icon_button(icon::close(I_MD, theme::TEXT_DIM), ICON_BUTTON_SIZE, style, on_press)
}

pub fn hinted_icon_button<'a, Message: Clone + 'a>(
    mark: Element<'a, Message>,
    box_size: f32,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    on_press: Message,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(icon_button(mark, box_size, style, on_press), text(label).size(T_XS).color(theme::TEXT), position)
        .gap(S1)
        .padding(S2)
        .delay(Duration::from_millis(450))
        .style(theme::hint)
        .into()
}

pub fn dropdown<'a, T, L, V, Message>(options: L, selected: Option<V>, on_select: impl Fn(T) -> Message + 'a) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone + 'a,
{
    pick_list(options, selected, on_select).text_size(T_SM).padding([S2, S3]).style(theme::picker).menu_style(theme::picker_menu).into()
}

pub fn action<'a, Message: Clone + 'a>(
    mark: Element<'a, Message>,
    label: &'a str,
    padding: impl Into<iced::Padding>,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    on_press: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    button(labelled(mark, label)).padding(padding).style(style).on_press_maybe(on_press.into()).into()
}

pub fn worded<'a, Message: Clone + 'a>(
    label: &'a str,
    padding: impl Into<iced::Padding>,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label).size(T_SM)).padding(padding).style(style).on_press(on_press).into()
}

pub fn chip<'a, Message: Clone + 'a>(
    mark: Element<'a, Message>,
    label: impl text::IntoFragment<'a>,
    on_press: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    let body = row![mark, text(label).size(T_XS)].spacing(S1).align_y(Center);
    button(body).padding([S1, S2]).style(theme::menu_control_button).on_press_maybe(on_press.into()).into()
}

pub fn card_action<'a, Message: Clone + 'a>(
    mark: Element<'a, Message>,
    label: &'a str,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    on_press: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    let body = container(labelled(mark, label)).width(Fill).align_x(Center);
    button(body).width(Fill).padding([S2, S3]).style(style).on_press_maybe(on_press.into()).into()
}

pub fn title<'a, Message: 'a>(body: impl text::IntoFragment<'a>, size: f32) -> Element<'a, Message> {
    text(body).size(size).font(theme::FONT_MEDIUM).color(theme::TEXT).into()
}

pub fn paragraph<'a, Message: 'a>(body: impl text::IntoFragment<'a>, size: f32, color: Color) -> Element<'a, Message> {
    text(body).size(size).color(color).line_height(LineHeight::Absolute(theme::line_of(size).into())).into()
}

pub fn wrapped<'a, Message: 'a>(body: impl text::IntoFragment<'a>, size: f32, color: Color) -> Element<'a, Message> {
    text(body).size(size).color(color).wrapping(text::Wrapping::WordOrGlyph).into()
}

pub fn labelled<'a, Message: 'a>(mark: Element<'a, Message>, label: &'a str) -> Element<'a, Message> {
    row![mark, text(label).size(T_SM)].spacing(S2).align_y(Center).into()
}

pub fn icon_menu_item<'a, Message: Clone + 'a>(mark: Element<'a, Message>, label: &'a str, on_press: Message) -> Element<'a, Message> {
    button(labelled(mark, label)).width(Fill).padding([S2, S3]).style(theme::menu_item).on_press(on_press).into()
}

pub fn hairline<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new().height(1)).width(Fill).style(theme::hairline).into()
}

pub fn one_line<'a, Message: 'a>(content: String, size: f32, color: Color) -> Element<'a, Message> {
    let line = theme::line_of(size);
    let label = text(content).size(size).line_height(LineHeight::Absolute(line.into())).center().color(color);
    let label = if size >= T_MD { label.font(theme::FONT_MEDIUM) } else { label };

    container(label).width(Fill).height(Length::Fixed(line)).clip(true).into()
}

fn boxed<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, height: f32) -> Container<'a, Message> {
    container(content).width(Fill).height(Length::Fixed(height))
}

fn fitted<'a, Message: 'a>(handle: Handle, fit: ContentFit, radius: f32) -> Element<'a, Message> {
    image(handle).width(Fill).height(Fill).content_fit(fit).border_radius(radius).into()
}

pub fn art_tile<'a, Message: 'a>(handle: Option<Handle>, height: f32) -> Element<'a, Message> {
    match handle {
        Some(handle) => boxed(fitted(handle, ContentFit::Cover, theme::R_IN_CARD), height).into(),
        None => boxed(Space::new(), height).style(theme::tile).into(),
    }
}

pub fn stage<'a, Message: 'a>(handle: Option<Handle>, height: f32, placeholder: &'a str) -> Element<'a, Message> {
    match handle {
        Some(handle) => boxed(fitted(handle, ContentFit::Contain, theme::R_CTRL), height).style(theme::inset).into(),
        None => boxed(text(placeholder).size(T_SM).color(theme::TEXT_FAINT), height).center(Fill).style(theme::inset).into(),
    }
}

pub fn header<'a, Message: Clone + 'a>(title: String, close: Message) -> Element<'a, Message> {
    row![
        text(title).size(T_XL).font(theme::FONT_MEDIUM).color(theme::TEXT),
        Space::new().width(Fill),
        close_button(theme::icon_button, close),
    ]
    .align_y(Center)
    .spacing(S3)
    .into()
}

pub fn overlay_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(SCROLL_RAIL).scroller_width(SCROLL_THUMB).margin(1))
}

pub fn sheet<'a, Message: Clone + 'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: impl Into<Element<'a, Message>>,
    id: &'static str,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    let middle = scrollable(container(body.into()).width(Fill))
        .direction(overlay_scrollbar())
        .style(theme::scroller)
        .id(iced::widget::Id::new(id))
        .height(Fill);

    container(column![header.into(), middle, footer.into()].spacing(S3))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .padding(PAD_PANEL)
        .style(theme::panel)
        .into()
}

pub fn overlay<'a, Message: Clone + 'a>(panel: Element<'a, Message>, dismiss: Message, absorb: Message) -> Element<'a, Message> {
    let behind = mouse_area(container(Space::new()).width(Fill).height(Fill).style(theme::backdrop)).on_press(dismiss).on_scroll({
        let absorb = absorb.clone();
        move |_| absorb.clone()
    });

    stack![behind, container(mouse_area(panel).on_press(absorb)).width(Fill).height(Fill).center(Fill),].into()
}

pub fn modal<'a, Message: Clone + 'a>(
    body: impl Into<Element<'a, Message>>,
    width: f32,
    dismiss: Message,
    absorb: Message,
) -> Element<'a, Message> {
    let panel = container(body.into()).width(Length::Fixed(width)).padding(PAD_PANEL + S2).style(theme::panel);

    overlay(panel.into(), dismiss, absorb)
}

pub fn bar<'a, Message: 'a>(fraction: Option<f32>) -> Element<'a, Message> {
    let track = |width: Length, tint: Color| {
        container(Space::new().height(3)).width(width).style(move |_: &Theme| container::Style {
            background: Some(tint.into()),
            border: iced::border::rounded(2),
            ..container::Style::default()
        })
    };

    let (filled, empty) = portions(fraction);
    let mut line = row![].width(Fill);

    if filled > 0 {
        line = line.push(track(Length::FillPortion(filled), theme::ACCENT));
    }

    if empty > 0 {
        line = line.push(track(Length::FillPortion(empty), theme::BORDER_HI));
    }

    line.spacing(2).into()
}

fn portions(fraction: Option<f32>) -> (u16, u16) {
    let filled = fraction.map_or(0, |f| (f.clamp(0.0, 1.0) * 1000.0).round() as u16);

    (filled, 1000 - filled)
}

pub fn settings_block<'a, Message: 'a>(settings: Option<&str>, looked: bool) -> Option<Element<'a, Message>> {
    let body: Element<'a, Message> = match settings {
        Some(settings) => text(settings.to_string()).size(T_SM).color(theme::TEXT).into(),
        None if looked => text("Not listed here. Check the map's page, linked below.").size(T_SM).color(theme::TEXT_DIM).into(),
        None => return None,
    };

    Some(
        container(column![text("Recommended settings").size(T_XS).color(theme::TEXT_DIM), body,].spacing(S1))
            .width(Fill)
            .padding(S3)
            .style(theme::inset)
            .into(),
    )
}

pub fn megabytes(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / 1_000_000.0)
}

pub fn capitalised(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "tests/ui.rs"]
mod tests;
