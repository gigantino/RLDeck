use iced::widget::{Space, button, column, container, grid, mouse_area, row, scrollable, stack, text, text_input, tooltip};
use iced::{Center, Element, Fill, Length, Right, Top};

use crate::model::{Shelf, Sort, Tab};
use crate::theme::{I_LG, I_MD, I_SM, PAD_MENU, S1, S2, S3, S4, S5, T_MD, T_SM, T_XL, T_XS};
use crate::ui::{hairline, icon_menu_item, megabytes};
use crate::{catalog, icon, theme, ui};

use super::{CARD_W, CATALOG_CARD_W, Loading, MENU_TOP, Message, Pending, Pick, RlDeck, Working};

impl RlDeck {
    pub fn view(&self) -> Element<'_, Message> {
        let cards = self.cards();

        let body: Element<'_, Message> = if cards.is_empty() {
            container(self.empty_state()).width(Fill).height(Fill).center(Fill).into()
        } else {
            let tiles = cards.iter().map(|card| self.card(card)).collect::<Vec<_>>();

            scrollable(
                container(
                    // Shrink is the only sizing that lets a cell be as tall as
                    // its content; the others clip whatever doesn't fit.
                    grid(tiles).fluid(if self.tab == Tab::Explore { CATALOG_CARD_W } else { CARD_W }).height(Length::Shrink).spacing(S3),
                )
                .padding(iced::Padding::from([S3, S4]).bottom(S5)),
            )
            .direction(ui::overlay_scrollbar())
            .style(theme::scroller)
            .id(iced::widget::Id::new(match self.tab {
                Tab::Library => "grid-library",
                Tab::Explore => "grid-catalog",
            }))
            .on_scroll(|viewport| Message::Scrolled(viewport.relative_offset().y))
            .height(Fill)
            .into()
        };

        // The notice keeps its slot whether or not there's anything in it.
        // Adding and removing a child would renumber the tree and throw the
        // scroll position back to the top mid-download.
        let notice: Element<'_, Message> = match (&self.working, &self.notice) {
            (Some(working), _) => self.working_bar(working),
            (None, Some(notice)) => notice_bar(notice),
            (None, None) => Space::new().height(0).into(),
        };

        let screen = column![self.top_bar(), hairline(), notice, body].width(Fill).height(Fill);

        if let Some(pending) = &self.pending {
            return stack![screen, confirm_backup(pending)].into();
        }

        if self.chooser {
            return stack![screen, self.game_chooser()].into();
        }

        if let Some((entry, shown)) = self.detail {
            return stack![screen, self.detail_view(entry, shown)].into();
        }

        if let Some(key) = &self.local_detail {
            return stack![screen, self.local_view(key)].into();
        }

        if self.menu_open { stack![screen, self.menu()].into() } else { screen.into() }
    }

    pub(super) fn game_chooser(&self) -> Element<'_, Message> {
        let (head, sub) = match self.installs.len() {
            0 => (
                "Where is Rocket League?",
                "We couldn't find it automatically. Pick the folder it's installed in, the one with a TAGame folder inside.",
            ),
            1 => ("Rocket League folder", "This is the install we found. Pick a different folder if it isn't the one you play."),
            _ => ("Which Rocket League?", "More than one install is on this PC. Maps go into the one you pick."),
        };

        let mut body = column![ui::title(head, T_XL), ui::paragraph(sub, T_SM, theme::TEXT_DIM),].spacing(S2);

        let mut found = column![].spacing(S2);

        for install in &self.installs {
            let current = self.game_dir.as_deref() == Some(install.root.as_path());

            let entry = column![
                row![
                    text(install.launcher.label()).size(T_SM).color(theme::TEXT),
                    Space::new().width(Fill),
                    if current { icon::check(I_SM, theme::ACCENT_HI) } else { Space::new().width(0).into() },
                ]
                .align_y(Center),
                ui::wrapped(install.root.display().to_string(), T_XS, theme::TEXT_FAINT),
            ]
            .spacing(S1);

            found = found.push(
                button(entry).width(Fill).padding([S2, S3]).style(theme::menu_item).on_press(Message::GameChosen(install.root.clone())),
            );
        }

        if !self.installs.is_empty() {
            body = body.push(container(found).padding([S2, 0.0]));
        }

        let actions = row![
            Space::new().width(Fill),
            ui::worded("Not now", [S2, S3], theme::ghost_button, Message::CloseChooser),
            ui::action(
                icon::folder_open(I_MD, theme::TEXT),
                "Choose a folder\u{2026}",
                [S2, S4],
                theme::card_button(false),
                Message::Browse(Pick::GameFolder),
            ),
        ]
        .align_y(Center)
        .spacing(S2);

        ui::modal(column![body, actions].spacing(S4), 460.0, Message::CloseChooser, Message::Absorb)
    }

    pub(super) fn top_bar(&self) -> Element<'_, Message> {
        let mut search = row![
            text_input("Search maps\u{2026}", &self.query)
                .on_input(Message::QueryChanged)
                .size(T_MD)
                .padding([S2, S3])
                .style(theme::search)
                .width(Length::Fixed(200.0))
        ]
        .spacing(S1)
        .align_y(Center);

        if !self.query.is_empty() {
            search = search.push(ui::close_button(theme::icon_button, Message::QueryChanged(String::new())));
        }

        let tabs =
            row(Tab::ALL.map(|tab| ui::worded(tab.label(), [S2, S3], theme::tab(self.tab == tab), Message::TabSelected(tab)))).spacing(S1);

        let actions = row![
            ui::action(icon::import(I_MD, theme::TEXT_DIM), "Import", S2, theme::ghost_button, Message::Browse(Pick::MapFiles)),
            ui::action(icon::repair(I_MD, theme::TEXT_DIM), "Repair", S2, theme::ghost_button, Message::Repair),
            ui::hinted_icon_button(
                icon::settings(I_MD, theme::TEXT_DIM),
                ui::ICON_BUTTON_SIZE,
                theme::ghost_button,
                Message::MenuToggled,
                "Settings & folders",
                tooltip::Position::Bottom,
            ),
        ]
        .spacing(S1)
        .align_y(Center);

        let ordering = match self.tab {
            Tab::Library => ui::dropdown(Shelf::ALL, Some(self.shelf), Message::ShelfSelected),
            Tab::Explore => ui::dropdown(Sort::ALL, Some(self.sort), Message::SortSelected),
        };

        let trailing = row![ordering, actions].align_y(Center).spacing(S1);

        let bar = row![search, tabs, Space::new().width(Fill), trailing].align_y(Center).spacing(S4);

        container(bar).width(Fill).padding([S3, S4]).style(theme::top_bar).into()
    }

    pub(super) fn working_bar(&self, working: &Working) -> Element<'_, Message> {
        let done = self.progress.done();
        let total = self.progress.total();

        let counted = if total > 0 {
            format!("{} of {}", megabytes(done), megabytes(total))
        } else if done > 0 {
            megabytes(done)
        } else {
            String::new()
        };

        let head = row![
            icon::spinner(I_SM, theme::TEXT_DIM, self.spin),
            text(ui::capitalised(&working.label)).size(T_SM).color(theme::TEXT).width(Fill),
            text(counted).size(T_XS).color(theme::TEXT_FAINT),
        ]
        .align_y(Center)
        .spacing(S3);

        column![
            container(column![head, ui::bar(self.progress.fraction())].spacing(S2)).width(Fill).padding([S2, S4]).style(theme::notice),
            hairline(),
        ]
        .into()
    }

    pub(super) fn menu(&self) -> Element<'_, Message> {
        let panel = container(
            column![
                icon_menu_item(icon::folder_input(I_SM, theme::TEXT_DIM), "Import a folder\u{2026}", Message::Browse(Pick::MapFolder),),
                hairline(),
                folder_row(
                    "Rocket League",
                    self.game_dir.as_deref().map(|dir| {
                        match self.installs.iter().find(|found| found.root == dir) {
                            Some(found) => {
                                format!("{} \u{00b7} {}", found.launcher.label(), dir.display())
                            }
                            None => dir.display().to_string(),
                        }
                    }),
                    "Not set",
                    self.game_dir.as_ref().map(|_| Message::OpenGameFolder),
                    "Change...",
                    Message::ChooseGame,
                ),
                hairline(),
                folder_row(
                    "Map folder",
                    Some(self.library_dir.display().to_string()),
                    "",
                    Some(Message::OpenMapFolder),
                    "Change...",
                    Message::Browse(Pick::LibraryFolder),
                ),
            ]
            .spacing(S1),
        )
        .width(Length::Fixed(340.0))
        .padding(PAD_MENU)
        .style(theme::menu);

        let panel = mouse_area(panel).on_press(Message::Absorb).on_scroll(|_| Message::Absorb);

        mouse_area(container(panel).width(Fill).height(Fill).padding([MENU_TOP, S4]).align_x(Right).align_y(Top))
            .on_press(Message::MenuToggled)
            .on_scroll(|_| Message::Absorb)
            .into()
    }

    pub(super) fn empty_state(&self) -> Element<'_, Message> {
        let searching = !self.query.trim().is_empty();

        let (head, sub) = match (self.tab, searching) {
            (_, true) => ("No matches".to_string(), String::new()),
            (Tab::Library, _) => ("No maps yet".to_string(), self.library_dir.display().to_string()),
            (Tab::Explore, _) => match &self.catalog_state {
                Loading::Busy => ("Loading\u{2026}".to_string(), String::new()),
                Loading::Failed(err) => ("Catalog unavailable".to_string(), err.clone()),
                Loading::Ready => ("Nothing here".to_string(), String::new()),
            },
        };

        let readable_width = (self.window.width - S5 * 2.0).clamp(240.0, 520.0);

        let mut stack = column![ui::title(head, theme::T_LG)].spacing(S2).width(Length::Fixed(readable_width)).align_x(Center);

        if !sub.is_empty() {
            stack = stack.push(ui::wrapped(sub, T_XS, theme::TEXT_FAINT));
        }

        if self.tab == Tab::Library && !searching {
            let mark = icon::import(I_MD, theme::TEXT);
            stack = stack.push(ui::action(mark, "Import a map", [S2, S4], theme::card_button(false), Message::Browse(Pick::MapFiles)));
        }

        if self.tab == Tab::Explore && !searching && matches!(self.catalog_state, Loading::Failed(_)) {
            let mark = icon::repair(I_MD, theme::TEXT);
            stack = stack.push(ui::action(mark, "Try again", [S2, S4], theme::card_button(false), Message::RetryCatalog));
        }

        stack.into()
    }
}

pub(super) fn confirm_backup(pending: &Pending) -> Element<'_, Message> {
    let body = column![
        ui::title("Back up your Underpass first?", T_XL),
        ui::paragraph(
            format!("Loading {} replaces Underpass. Before that happens we'll keep a copy of the one you have now ({}), so Repair can always put it back.", pending.name, megabytes(pending.bytes),),
            T_SM,
            theme::TEXT_DIM,
        ),
        container(ui::paragraph(
            "If you've loaded a map with another tool and haven't restored it, close this and repair through Steam or Epic first. Otherwise that map gets saved as your original.",
            T_XS,
            theme::TEXT_DIM,
        ))
        .width(Fill)
        .padding(S3)
        .style(theme::warning),
    ]
    .spacing(S3);

    let actions = row![
        Space::new().width(Fill),
        ui::worded("Cancel", [S2, S3], theme::ghost_button, Message::BackupDeclined),
        ui::action(icon::check(I_LG, theme::TEXT), "Back up and load", [S2, S4], theme::card_button(false), Message::BackupConfirmed,),
    ]
    .align_y(Center)
    .spacing(S2);

    ui::modal(column![body, actions].spacing(S4), 480.0, Message::BackupDeclined, Message::Absorb)
}

pub(super) fn notice_bar(notice: &str) -> Element<'_, Message> {
    column![
        container(
            row![
                container(ui::wrapped(notice, T_SM, theme::TEXT)).width(Fill),
                ui::icon_button(icon::close(I_SM, theme::TEXT_DIM), ui::ICON_BUTTON_SIZE, theme::ghost_button, Message::DismissNotice,),
            ]
            .align_y(Center)
            .spacing(S3),
        )
        .width(Fill)
        .padding([S2, S4])
        .style(theme::notice),
        hairline(),
    ]
    .into()
}

pub(super) fn folder_row<'a>(
    label: &'a str,
    path: Option<String>,
    missing: &'a str,
    open: Option<Message>,
    action: &'a str,
    on_press: Message,
) -> Element<'a, Message> {
    let (body, tint) = match path {
        Some(path) => (path, theme::TEXT_DIM),
        None => (missing.to_string(), theme::EMBER),
    };

    container(
        column![
            text(label).size(T_XS).color(theme::TEXT_FAINT),
            ui::wrapped(body, T_XS, tint),
            row![
                ui::chip(icon::folder_open(I_SM, theme::TEXT_DIM), "Open", open),
                ui::chip(icon::folder_input(I_SM, theme::TEXT_DIM), action, on_press),
            ]
            .spacing(S2),
        ]
        .spacing(S1),
    )
    .padding([S2, S3])
    .into()
}

pub(super) fn refusal_note<'a>() -> Element<'a, Message> {
    container(
        row![
            icon::alert(I_MD, theme::EMBER),
            column![
                ui::title("Google Drive wouldn't hand this one over", T_SM),
                text(
                    "This usually means the file hit its daily download limit. Open it in your \
                     browser, save the file, then bring it back here with Import."
                )
                .size(T_XS)
                .color(theme::TEXT_DIM),
            ]
            .spacing(S1),
        ]
        .spacing(S3),
    )
    .width(Fill)
    .padding(S3)
    .style(theme::warning)
    .into()
}

pub(super) fn links_in<'a>(sources: &[Option<&str>]) -> Option<Element<'a, Message>> {
    let mut urls: Vec<String> = Vec::new();

    for body in sources.iter().flatten() {
        for chunk in catalog::linkify(body) {
            if let catalog::Chunk::Link(url) = chunk {
                let url = catalog::absolute(url);
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
        }
    }

    if urls.is_empty() {
        return None;
    }

    let buttons = urls.into_iter().map(|url| {
        let label = url.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string();

        ui::chip(icon::external(I_SM, theme::TEXT_DIM), label, Message::OpenBrowser(url))
    });

    Some(row(buttons).spacing(S2).wrap().into())
}
