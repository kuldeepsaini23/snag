//! Basic views with default iced widgets. The Figma pass replaces this module.

use crate::format;
use crate::state::{Model, Screen};
use crate::update::{App, Message};
use iced::widget::{button, checkbox, column, container, progress_bar, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill};
use rdm_core::{Item, Status};

pub fn view(app: &App) -> Element<'_, Message> {
    match app.model.screen {
        Screen::Downloads => downloads(&app.model),
        Screen::Settings => settings(&app.model),
    }
}

fn downloads(m: &Model) -> Element<'_, Message> {
    let top = row![
        text_input("Paste a link and press Enter…", &m.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Add)
            .padding(8)
            .width(Fill),
        button("Add").on_press(Message::Add).padding([8, 16]),
        button("Settings").on_press(Message::OpenSettings).padding([8, 16]),
    ]
    .spacing(8);

    let list: Element<'_, Message> = if m.items.is_empty() {
        container(text("No downloads yet. Paste a link above.").size(14)).padding(24).into()
    } else {
        scrollable(column(m.items.iter().rev().map(|i| item_row(i, m.selected == Some(i.id)))).spacing(8).padding([0, 12]))
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

fn item_row(i: &Item, selected: bool) -> Element<'_, Message> {
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

    let mut actions = row![].spacing(6);
    actions = match i.status {
        Status::Running | Status::Queued => actions.push(button("Pause").on_press(Message::Pause(i.id))),
        Status::Paused | Status::Failed(_) => actions.push(button("Resume").on_press(Message::Resume(i.id))),
        Status::Done => actions.push(button("Show in folder").on_press_maybe(i.dest.as_ref().map(|_| Message::ShowInFolder(i.id)))),
    };
    actions = actions.push(button("Remove").on_press(Message::Remove(i.id))).push(button("Delete").on_press(Message::Delete(i.id)));

    let body = column![
        row![text(&i.name).size(15).width(Fill), text(format::status_label(&i.status)).size(13)].align_y(Alignment::Center),
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
