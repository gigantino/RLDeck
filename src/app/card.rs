use iced::widget::{Column, Space, button, column, container, mouse_area, row, text, text_editor, tooltip};
use iced::{Center, Element, Fill};

use crate::matching::catalog_star_key;
use crate::model::{Art, Card, Source};
use crate::theme::{I_LG, I_MD, I_SM, PAD_CARD, S2, S3, S4, S5, T_MD, T_SM, T_XS};
use crate::ui::{megabytes, one_line};
use crate::{catalog, icon, theme, ui};

use super::tasks::has_distinct_body;
use super::view::{links_in, refusal_note};
use super::{ART_H, CATALOG_ART_H, LOCAL_SHEET_H, Message, RlDeck, SHEET_W, SHOT_H};

impl RlDeck {
    pub(super) fn card(&self, card: &Card) -> Element<'_, Message> {
        let hovered = self.hovered.as_deref() == Some(card.key.as_str());
        let catalog = card.catalog_index.is_some();
        let art_h = if catalog { CATALOG_ART_H } else { ART_H };
        let title_size = if catalog { theme::T_LG } else { T_MD };

        let downloading = self.busy.contains(&card.key);
        let swapping = self.busy_with(&card.name);
        let ink = if card.loaded { theme::ACCENT_HI } else { theme::ACCENT };

        let (mark, action, act_on) = if downloading {
            (icon::spinner(I_SM, ink, self.spin), "Downloading\u{2026}", None)
        } else if swapping {
            (icon::spinner(I_SM, ink, self.spin), "Loading\u{2026}", None)
        } else if card.loaded {
            (icon::check(I_SM, ink), "In game", Some(Message::Repair))
        } else {
            match card.source {
                Source::Local => (icon::load(I_SM, ink), "Load", Some(Message::Act(card.key.clone()))),
                Source::Catalog => (icon::download(I_SM, ink), "Download", Some(Message::Act(card.key.clone()))),
            }
        };

        let art = ui::art_tile(self.handle_for(&card.art), art_h);

        let mut upper = column![art].spacing(S2).align_x(Center);
        upper = upper.push(one_line(card.name.clone(), title_size, theme::TEXT));

        let (under, tint) = match card.source {
            Source::Catalog => (card.blurb.clone().unwrap_or_default(), theme::TEXT_DIM),
            Source::Local => (card.author.clone().unwrap_or_else(|| "Unknown".to_string()), theme::AUTHOR),
        };
        upper = upper.push(one_line(under, T_XS, tint));

        let open = match card.catalog_index {
            Some(index) => Message::OpenDetail(index),
            None => Message::OpenLocal(card.key.clone()),
        };

        let upper: Element<'_, Message> = button(upper).padding(0).width(Fill).style(theme::bare_button).on_press(open).into();

        let act = ui::card_action(mark, action, theme::card_button(card.loaded), act_on);
        let armed = self.armed.as_deref() == Some(card.key.as_str());

        let actions: Element<'_, Message> = if card.source != Source::Local {
            row![act, self.star_control(&catalog_star_key(&card.name), tooltip::Position::Top),].spacing(S2).align_y(Center).into()
        } else if armed {
            row![
                ui::card_action(
                    icon::trash(I_SM, theme::ACCENT),
                    "Are you sure?",
                    theme::card_button(false),
                    Message::DeleteConfirmed(card.key.clone()),
                ),
                ui::close_button(theme::card_icon_button, Message::Arm(None)),
            ]
            .spacing(S2)
            .align_y(Center)
            .into()
        } else {
            row![
                act,
                self.star_control(&card.key, tooltip::Position::Top),
                ui::hinted_icon_button(
                    icon::trash(I_MD, theme::ACCENT),
                    ui::ICON_BUTTON_SIZE,
                    theme::card_icon_button,
                    Message::Arm(Some(card.key.clone())),
                    "Delete map",
                    tooltip::Position::Top,
                ),
            ]
            .spacing(S2)
            .align_y(Center)
            .into()
        };

        let tile = container(column![upper, actions].spacing(S2).align_x(Center))
            .padding(PAD_CARD)
            .width(Fill)
            .style(theme::card(hovered, card.loaded));

        mouse_area(tile).on_enter(Message::HoverStart(card.key.clone())).on_exit(Message::HoverEnd(card.key.clone())).into()
    }

    pub(super) fn star_control(&self, key: &str, position: tooltip::Position) -> Element<'static, Message> {
        let starred = self.config.is_starred(key);
        ui::hinted_icon_button(
            icon::star(I_MD, if starred { theme::EMBER } else { theme::ACCENT }, starred),
            ui::ICON_BUTTON_SIZE,
            theme::star_button(starred),
            Message::StarToggled(key.to_string()),
            if starred { "Remove star" } else { "Star map" },
            position,
        )
    }

    pub(super) fn handle_for(&self, art: &Art) -> Option<iced::widget::image::Handle> {
        art.key().and_then(|key| self.gallery.get(&key)).cloned()
    }

    pub(super) fn copy_block(&self, blurb: Option<&str>, body: Option<&str>, settings: Option<&str>, looked: bool) -> Column<'_, Message> {
        let mut copy = column![].spacing(S2).width(Fill);

        if let Some(blurb) = blurb {
            copy = copy.push(text(blurb.to_string()).size(T_MD).color(theme::TEXT));
        }
        if has_distinct_body(blurb, body) {
            copy = copy.push(self.readable_description());
        }
        if let Some(block) = ui::settings_block(settings, looked) {
            copy = copy.push(block);
        }
        if let Some(links) = links_in(&[blurb, body, settings]) {
            copy = copy.push(links);
        }

        copy
    }

    pub(super) fn sheet<'a>(
        &'a self,
        title: String,
        body: impl Into<Element<'a, Message>>,
        footer: impl Into<Element<'a, Message>>,
        id: &'static str,
        height: f32,
    ) -> Element<'a, Message> {
        let sheet = ui::sheet(ui::header(title, Message::CloseDetail), body, footer, id, SHEET_W, height);
        ui::overlay(sheet, Message::CloseDetail, Message::Absorb)
    }

    pub(super) fn detail_view(&self, index: usize, shown: usize) -> Element<'_, Message> {
        let Some(entry) = self.catalog.get(index) else {
            return Space::new().into();
        };

        let count = entry.media.len();
        let url = entry.media.get(shown);

        let placeholder = match url {
            Some(url) if self.gallery.is_unavailable(url) => "Image unavailable",
            Some(_) => "Loading\u{2026}",
            None => "No screenshot",
        };

        let picture = ui::stage(url.and_then(|url| self.gallery.get(url)).cloned(), SHOT_H, placeholder);

        let arrow = |mark, delta| ui::icon_button(mark, ui::ICON_BUTTON_SIZE, theme::icon_button, Message::StepImage(delta));

        let stage: Element<'_, Message> = if count > 1 {
            row![arrow(icon::chevron_left(I_MD, theme::TEXT_DIM), -1), picture, arrow(icon::chevron_right(I_MD, theme::TEXT_DIM), 1)]
                .spacing(S3)
                .align_y(Center)
                .into()
        } else {
            picture
        };

        let dots = row((0..count).map(|i| {
            button(Space::new().width(S2).height(S2)).padding(0).style(theme::dot(i == shown)).on_press(Message::ShowImage(i)).into()
        }))
        .spacing(S2);

        let blurb = entry.description_short.as_deref().map(catalog::plain_text);
        let body = entry.description.as_deref().map(catalog::plain_text);
        let settings = self.settings.get(&entry.name).map(String::as_str);

        let looked = self.checked.contains(&entry.name);
        let mut copy = self.copy_block(blurb.as_deref(), body.as_deref(), settings, looked);

        if self.refused.contains(&entry.name) {
            copy = copy.push(refusal_note());
        }

        let busy = self.busy.contains(&entry.name);

        let download = ui::action(
            if busy { icon::spinner(I_LG, theme::ACCENT, self.spin) } else { icon::download(I_LG, theme::ACCENT) },
            if busy { "Downloading\u{2026}" } else { "Download" },
            [S2, S4],
            theme::card_button(false),
            (!busy).then(|| Message::Act(entry.name.clone())),
        );

        let footer = row![
            ui::action(
                icon::external(I_MD, theme::TEXT_DIM),
                "Open on lethamyr.com",
                [S2, S3],
                theme::menu_control_button,
                Message::OpenMapPage(entry.name.clone(), index),
            ),
            Space::new().width(Fill),
            self.star_control(&catalog_star_key(&entry.name), tooltip::Position::Top),
            download,
        ]
        .align_y(Center)
        .spacing(S3);

        let body = column![stage, dots, copy].spacing(S4).align_x(Center);

        self.sheet(entry.name.clone(), body, footer, "detail", self.sheet_height())
    }

    pub(super) fn local_view(&self, key: &str) -> Element<'_, Message> {
        let Some(map) = self.find_map(key) else {
            return Space::new().into();
        };

        let art = ui::stage(map.image.as_ref().and_then(|p| self.gallery.get(&p.to_string_lossy())).cloned(), SHOT_H, "No screenshot");

        let recorded = map.settings.as_deref().or_else(|| self.settings.get(&map.name).map(String::as_str));
        let looked = map.settings_checked || self.checked.contains(&map.name);

        let mut copy = column![].spacing(S2).width(Fill);
        if let Some(author) = &map.author {
            copy = copy.push(text(author.clone()).size(T_SM).color(theme::AUTHOR));
        }
        let copy = copy.push(self.copy_block(map.blurb.as_deref(), map.description.as_deref(), recorded, looked));

        let size = megabytes(map.bytes);
        let files = match map.file_count() {
            1 => size,
            n => format!("{n} files \u{00b7} {size}"),
        };

        let mut footer = row![text(files).size(T_XS).color(theme::TEXT_FAINT)].align_y(Center).spacing(S3);

        if let Some(source) = &map.source {
            footer = footer.push(ui::action(
                icon::external(I_MD, theme::TEXT_DIM),
                "Original page",
                [S2, S3],
                theme::menu_control_button,
                Message::OpenBrowser(source.clone()),
            ));
        }

        footer = footer.push(Space::new().width(Fill));

        let loaded = self.loaded_map.as_deref() == Some(map.name.as_str());
        let swapping = self.busy_with(&map.name);

        let (mark, label, act_on) = if swapping {
            (icon::spinner(I_LG, theme::ACCENT, self.spin), "Loading\u{2026}", None)
        } else if loaded {
            (icon::check(I_LG, theme::ACCENT_HI), "In game", Some(Message::Repair))
        } else {
            (icon::load(I_LG, theme::ACCENT), "Load", Some(Message::Act(key.to_string())))
        };

        let act = ui::action(mark, label, [S2, S4], theme::card_button(loaded), act_on);

        footer = footer.push(self.star_control(key, tooltip::Position::Top));

        let height = self.sheet_height().min(LOCAL_SHEET_H);

        self.sheet(map.name.clone(), column![art, copy].spacing(S4), footer.push(act), "local-detail", height)
    }

    pub(super) fn sheet_height(&self) -> f32 {
        (self.window.height - S5 * 2.0).max(240.0)
    }

    pub(super) fn readable_description(&self) -> Element<'_, Message> {
        text_editor(&self.description).on_action(Message::Description).size(T_SM).padding(0).style(theme::readable).into()
    }
}
