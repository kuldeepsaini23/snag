//! Basic views with default iced widgets. The Figma pass replaces this module.

pub mod icon;
pub mod theme;

use crate::choices::{Limit, Quality, QueueChoice, quality_choices, speed_presets};
use crate::format;
use crate::queues::DAY_LETTERS;
use crate::state::{Model, Picker, Screen, file_missing};
use crate::update::{App, Message};
use iced::widget::{button, checkbox, column, container, pick_list, progress_bar, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill};
use rdm_core::{Item, Status};

pub fn view(app: &App) -> Element<'_, Message> {
    match app.model.screen {
        Screen::Downloads => downloads(&app.model),
        Screen::Settings => settings(&app.model),
        Screen::Queues => queues(&app.model),
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
        button("Queues").on_press(Message::OpenQueues).padding([8, 16]),
        button("Settings").on_press(Message::OpenSettings).padding([8, 16]),
    ]
    .spacing(8);

    let list: Element<'_, Message> = if m.items.is_empty() {
        container(text("No downloads yet. Paste a link above.").size(14)).padding(24).into()
    } else {
        scrollable(column(m.items.iter().rev().map(|i| item_row(m, i))).spacing(8).padding([0, 12]))
            .height(Fill)
            .into()
    };

    let (running, speed) = m.totals();
    let limit = m.settings.speed_limit_bps;
    let footer = row![
        text(format!("{running} downloading · {}", format::speed(speed))).size(12).width(Fill),
        pick_list(speed_presets(limit), Some(Limit(limit)), |l: Limit| Message::QuickLimit(l.0)).text_size(12).padding([4, 8]),
    ]
    .align_y(Alignment::Center);

    let mut page = column![top].spacing(12).padding(16);
    if let Some(n) = &m.notice {
        page = page.push(text(n).size(13));
    }
    page.push(list).push(footer).into()
}

fn item_row<'a>(m: &'a Model, i: &'a Item) -> Element<'a, Message> {
    let (selected, confirming_delete) = (m.selected == Some(i.id), m.pending_delete == Some(i.id));
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
    if m.queues.len() > 1 {
        let choices: Vec<QueueChoice> = m.queues.iter().map(|q| QueueChoice { id: q.id, name: q.name.clone() }).collect();
        let current = choices.iter().find(|c| c.id == i.queue).cloned();
        let id = i.id;
        actions = actions.push(pick_list(choices, current, move |c: QueueChoice| Message::MoveToQueue(id, c.id)).text_size(13).padding([4, 8]));
    }
    // Delete sits apart from Remove, is red, and needs a second click.
    let delete = if confirming_delete { "Click again: delete file" } else { "Delete file" };
    actions = actions
        .push(button("Remove").on_press(Message::Remove(i.id)))
        .push(space::horizontal().width(24))
        .push(button(delete).style(button::danger).on_press(Message::Delete(i.id)));

    let body = column![
        row![
            text(&i.name).size(15).width(Fill),
            text(if missing {
                "File missing".to_string()
            } else if m.waiting_for_schedule(i) {
                "Waiting for schedule".to_string()
            } else {
                format::status_label(&i.status)
            })
            .size(13),
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
        text("Videos").size(18),
        row![
            text("Quality").size(13),
            pick_list(quality_choices(), Some(Quality(d.preferred_quality.clone())), |q: Quality| Message::DraftQuality(q.0)).padding([6, 10]),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        checkbox(d.ask_quality).label("Ask every time (show the quality list with this one selected)").on_toggle(Message::DraftAsk),
        row![
            button(if m.updating_ytdlp { "Updating…" } else { "Update yt-dlp" }).on_press_maybe((!m.updating_ytdlp).then_some(Message::UpdateYtdlp)),
            text("It also updates itself once a week. Update now if a site stops working.").size(12),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
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

fn queues(m: &Model) -> Element<'_, Message> {
    let mut page = column![
        text("Queues").size(22),
        text("A queue with a schedule only downloads inside its time window. Move downloads between queues from the list.").size(13),
    ]
    .spacing(14)
    .padding(20)
    .max_width(640);
    for (i, d) in m.queue_drafts.iter().enumerate() {
        let mut head = row![text_input("Name", &d.name).on_input(move |v| Message::QueueName(i, v)).padding(6).width(Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if d.id != 0 {
            head = head
                .push(text("At once").size(13))
                .push(text_input("1", &d.max_concurrent).on_input(move |v| Message::QueueMax(i, v)).padding(6).width(50))
                .push(button("Delete").style(button::danger).on_press(Message::DeleteQueue(i)));
        }
        let mut card = column![head, checkbox(d.scheduled).label("Run on a schedule").on_toggle(move |v| Message::QueueScheduled(i, v))].spacing(10);
        if d.scheduled {
            card = card.push(
                row![
                    text("From").size(13),
                    text_input("23:00", &d.start).on_input(move |v| Message::QueueStart(i, v)).padding(6).width(70),
                    text("to").size(13),
                    text_input("until midnight", &d.stop).on_input(move |v| Message::QueueStop(i, v)).padding(6).width(120),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
            let days = DAY_LETTERS
                .iter()
                .enumerate()
                .fold(row![].spacing(12), |r, (day, letter)| r.push(checkbox(d.days[day]).label(*letter).on_toggle(move |v| Message::QueueDay(i, day, v))));
            card = card.push(days);
        }
        page = page.push(container(card).padding(12).width(Fill).style(container::rounded_box));
    }
    page = page.push(button("Add queue").on_press(Message::AddQueue));
    if let Some(n) = &m.notice {
        page = page.push(text(n).size(13));
    }
    page = page.push(
        row![
            button("Save").on_press(Message::SaveQueues).padding([8, 20]),
            button("Cancel").on_press(Message::CloseQueues).padding([8, 20]),
        ]
        .spacing(8),
    );
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
