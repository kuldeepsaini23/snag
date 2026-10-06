//! Basic views with default iced widgets. The Figma pass replaces this module.

use crate::format;
use crate::state::{Model, Picker, Screen, file_missing};
use crate::update::{App, Message};
use iced::widget::{button, checkbox, column, container, progress_bar, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill};
use rdm_core::{Item, Status};

pub fn view(app: &App) -> Element<'_, Message> {
    match app.model.screen {
        Screen::Downloads => downloads(&app.model),
        Screen::Settings => settings(&app.model),
        Screen::Picker => match &app.model.picker {
            Some(p) => picker(p),
            None => downloads(&app.model),
        },
    }
}

fn downloads(m: &Model) -> Element<'_, Message> {
    let top = row![
        text_input("Paste a link and press Enter…", &m.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Add)
            .padding(8)
            .width(Fill),
        button(if m.probing { "Reading…" } else { "Add" }).on_press_maybe((!m.probing).then_some(Message::Add)).padding([8, 16]),
        button("Settings").on_press(Message::OpenSettings).padding([8, 16]),
    ]
    .spacing(8);

    let list: Element<'_, Message> = if m.items.is_empty() {
        container(text("No downloads yet. Paste a link above.").size(14)).padding(24).into()
    } else {
        scrollable(column(m.items.iter().rev().map(|i| item_row(i, m.selected == Some(i.id), m.pending_delete == Some(i.id)))).spacing(8).padding([0, 12]))
            .height(Fill)
            .into()
    };

    let (running, speed) = m.totals();
    let footer = text(format!(
        "{running} downloading · {} · limit {}",
        format::speed(speed),
        if m.settings.speed_limit_bps == 0 { "off".to_string() } else { format::speed(m.settings.speed_limit_bps) }
    ))
    .size(12);

    let mut page = column![top].spacing(12).padding(16);
    if let Some(n) = &m.notice {
        page = page.push(text(n).size(13));
    }
    page.push(list).push(footer).into()
}

fn item_row(i: &Item, selected: bool, confirming_delete: bool) -> Element<'_, Message> {
    let percent = match (i.total, &i.status) {
        (_, Status::Done) => 100.0,
        (Some(t), _) if t > 0 => i.downloaded as f32 * 100.0 / t as f32,
        _ => 0.0,
    };
    let mut detail = match i.total {
        Some(t) => format!("{} of {}", format::bytes(i.downloaded), format::bytes(t)),
        None => format::bytes(i.downloaded),
    };
    if i.status == Status::Running {
        detail.push_str(&format!(" · {}", format::speed(i.speed_bps)));
        if let Some(eta) = i.total.and_then(|t| format::eta(t.saturating_sub(i.downloaded), i.speed_bps)) {
            detail.push_str(&format!(" · {eta} left"));
        }
    }

    let missing = file_missing(i);
    let mut actions = row![].spacing(6);
    actions = match i.status {
        Status::Done if missing => actions.push(button("Download again").on_press(Message::Redownload(i.id))),
        Status::Running | Status::Queued => actions.push(button("Pause").on_press(Message::Pause(i.id))),
        Status::Paused | Status::Failed(_) => actions.push(button("Resume").on_press(Message::Resume(i.id))),
        Status::Done => actions.push(button("Show in folder").on_press_maybe(i.dest.as_ref().map(|_| Message::ShowInFolder(i.id)))),
    };
    // Delete sits apart from Remove, is red, and needs a second click.
    let delete = if confirming_delete { "Click again: delete file" } else { "Delete file" };
    actions = actions
        .push(button("Remove").on_press(Message::Remove(i.id)))
        .push(space::horizontal().width(24))
        .push(button(delete).style(button::danger).on_press(Message::Delete(i.id)));

    let body = column![
        row![
            text(&i.name).size(15).width(Fill),
            text(if missing { "File missing".to_string() } else { format::status_label(&i.status) }).size(13),
        ]
        .align_y(Alignment::Center),
        progress_bar(0.0..=100.0, percent).girth(6),
        row![text(detail).size(12).width(Fill), actions].align_y(Alignment::Center),
    ]
    .spacing(6);

    let card = container(body).padding(12).width(Fill);
    let card = if selected { card.style(container::bordered_box) } else { card.style(container::rounded_box) };
    button(card).on_press(Message::Select(i.id)).style(button::text).padding(0).into()
}

fn settings(m: &Model) -> Element<'_, Message> {
    let d = &m.draft;
    let field = |label: &'static str, value: &str, on: fn(String) -> Message| {
        column![text(label).size(13), text_input("", value).on_input(on).padding(8)].spacing(4)
    };
    let mut page = column![
        text("Settings").size(22),
        field("Download folder", &d.download_dir, Message::DraftDir),
        field("Connections per download (1–16)", &d.connections, Message::DraftConnections),
        field("Downloads at the same time (1–10)", &d.max_concurrent, Message::DraftMax),
        field("Speed limit in KB/s (0 = unlimited)", &d.speed_limit_kbps, Message::DraftLimit),
        checkbox(d.sort_into_folders).label("Sort into category folders (Videos, Music, …)").on_toggle(Message::DraftSort),
        checkbox(d.start_immediately).label("Start downloads immediately").on_toggle(Message::DraftStart),
        checkbox(d.clipboard_watch).label("Watch the clipboard for links").on_toggle(Message::DraftClipboard),
        text("Chrome extension").size(18),
        text(&m.bridge_status).size(13),
        column![
            text("Pairing code: paste it once into the extension popup").size(13),
            row![
                text(&m.settings.extension_token).size(14).width(Fill),
                button("Copy").on_press(Message::CopyToken),
                button("New code").on_press(Message::NewToken),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(4),
    ]
    .spacing(14)
    .padding(20)
    .max_width(560);
    if let Some(n) = &m.notice {
        page = page.push(text(n).size(13));
    }
    page = page.push(row![
        button("Save").on_press(Message::SaveSettings).padding([8, 20]),
        button("Cancel").on_press(Message::CloseSettings).padding([8, 20]),
        space::horizontal(),
    ]
    .spacing(8));
    scrollable(page).into()
}

fn picker(p: &Picker) -> Element<'_, Message> {
    let mut page = column![text(&p.info.title).size(20)].spacing(10).padding(20).max_width(620);
    if !p.info.entries.is_empty() {
        page = page.push(text(format!("Playlist · {} videos · same quality for all", p.info.entries.len())).size(13));
    }
    page = page.push(text("Choose quality").size(14));
    for (i, option) in p.info.options.iter().enumerate() {
        let size = option.approx_size.map(|s| format!("≈ {}", format::bytes(s))).unwrap_or_default();
        let label = row![text(if i == p.choice { "●" } else { "○" }), text(&option.label).width(Fill), text(size).size(13)].spacing(10);
        let style = if i == p.choice { button::primary } else { button::secondary };
        page = page.push(button(label).on_press(Message::PickOption(i)).style(style).padding([8, 12]).width(Fill));
    }
    let count = p.info.entries.len().max(1);
    page = page.push(
        row![
            button(text(if count > 1 { format!("Download {count} videos") } else { "Download".to_string() })).on_press(Message::DownloadPicked).padding([8, 20]),
            button("Cancel").on_press(Message::CancelPick).padding([8, 20]),
        ]
        .spacing(8),
    );
    scrollable(page).into()
}
