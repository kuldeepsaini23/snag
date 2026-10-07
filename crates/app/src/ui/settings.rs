//! The Settings sheet (Figma frames 07–12). Closing it saves.

use super::color_picker;
use super::icon::{Icon, icon};
use super::style;
use super::theme::{self, Colors, SWATCHES};
use super::small;
use crate::choices::{Quality, quality_choices};
use crate::queues::DAY_LETTERS;
use crate::state::{Model, SETTINGS_TABS, SettingsTab};
use crate::update::Message;
use crate::view;
use iced::widget::{Space, button, column, container, pick_list, row, rule, scrollable, text, text_input, toggler};
use iced::{Alignment, Color, Element, Fill, Length};
use rdm_core::Status;

/// `phone`: the phone page link and its QR code, while phone sharing is on.
pub fn view<'a>(m: &'a Model, phone: Option<(&'a str, &'a iced::widget::qr_code::Data)>, c: Colors) -> Element<'a, Message> {
    let nav = SETTINGS_TABS.iter().fold(column![text("Settings").size(15).font(style::SEMIBOLD), Space::new().height(8)].spacing(2), |col, &tab| {
        let (glyph, label) = tab_info(tab);
        let on = m.settings_tab == tab;
        col.push(
            button(row![icon(glyph, 14), text(label).size(13)].spacing(10).align_y(Alignment::Center))
                .width(Fill)
                .padding([7, 10])
                .style(style::choice(c, on, 7.0))
                .on_press(Message::SettingsTab(tab)),
        )
    });
    let nav = container(nav).width(204).height(Fill).padding([18, 12]).style(move |_| container::Style {
        background: Some(c.sidebar.into()),
        border: iced::Border { radius: iced::border::Radius::default().top_left(14.0).bottom_left(14.0), ..Default::default() },
        ..Default::default()
    });

    let (title, subtitle) = match m.settings_tab {
        SettingsTab::General => ("General", "Where files go and what happens when they finish"),
        SettingsTab::Appearance => ("Appearance", "Make Snag yours: pick any accent colour"),
        SettingsTab::Connections => ("Connections", "How hard Snag pushes each server"),
        SettingsTab::Speed => ("Speed & Schedule", "Cap bandwidth and run queues on a schedule"),
        SettingsTab::Extension => ("Extension", "Catch downloads and videos straight from Chrome"),
        SettingsTab::Tools => ("Tools", "Helpers Snag uses for video and audio"),
    };
    let close = button(container(icon(Icon::X, 13)).center(Fill)).width(28).height(28).padding(0).style(style::secondary(c)).on_press(Message::CloseSettings);
    let head = row![column![text(title).size(17).font(style::SEMIBOLD), small(subtitle, c.text2)].spacing(3).width(Fill), close];

    let body = match m.settings_tab {
        SettingsTab::General => general(m, c),
        SettingsTab::Appearance => appearance(m, c),
        SettingsTab::Connections => connections(m, c),
        SettingsTab::Speed => speed(m, c),
        SettingsTab::Extension => extension(m, phone, c),
        SettingsTab::Tools => tools(m, c),
    };
    let mut content = column![head, scrollable(container(body).padding(iced::Padding { right: 10.0, ..Default::default() })).style(style::scroll(c)).height(Fill)].spacing(14);
    if let Some(e) = &m.settings_error {
        content = content.push(row![icon(Icon::WarningCircle, 14).color(c.danger), text(e).size(12.5)].spacing(8).align_y(Alignment::Center));
    } else if let Some(n) = &m.notice {
        content = content.push(row![icon(Icon::CheckCircle, 14).color(c.success), text(n).size(12.5)].spacing(8).align_y(Alignment::Center));
    }
    let content = container(content).width(Fill).height(Fill).padding([18, 22]);
    container(row![nav, content]).width(820).height(584).style(style::sheet(c)).into()
}

fn tab_info(tab: SettingsTab) -> (Icon, &'static str) {
    match tab {
        SettingsTab::General => (Icon::Gear, "General"),
        SettingsTab::Appearance => (Icon::Palette, "Appearance"),
        SettingsTab::Connections => (Icon::Plug, "Connections"),
        SettingsTab::Speed => (Icon::Gauge, "Speed & Schedule"),
        SettingsTab::Extension => (Icon::Browser, "Extension"),
        SettingsTab::Tools => (Icon::Wrench, "Tools"),
    }
}

// ---------- building blocks ----------

fn section<'a>(label: &str, rows: Vec<Element<'a, Message>>, c: Colors) -> Element<'a, Message> {
    let n = rows.len();
    let mut group = column![];
    for (k, r) in rows.into_iter().enumerate() {
        group = group.push(r);
        if k + 1 < n {
            group = group.push(rule::horizontal(1).style(move |_| rule::Style { color: c.line, radius: 0.0.into(), fill_mode: rule::FillMode::Full, snap: true }));
        }
    }
    column![
        text(label.to_uppercase()).size(10.5).font(style::SEMIBOLD).color(Color { a: 0.42, ..c.text }),
        container(group).style(style::card(c)),
    ]
    .spacing(8)
    .into()
}

/// One settings row: title, optional description, control on the right.
fn line<'a>(title: &str, desc: &str, control: Element<'a, Message>, c: Colors) -> Element<'a, Message> {
    let mut words = column![text(title.to_string()).size(13).font(style::MEDIUM)].spacing(2).width(Fill);
    if !desc.is_empty() {
        words = words.push(small(desc.to_string(), c.text3));
    }
    row![words, control].spacing(16).align_y(Alignment::Center).padding([10, 14]).into()
}

fn switch<'a>(on: bool, msg: fn(bool) -> Message, c: Colors) -> Element<'a, Message> {
    toggler(on).on_toggle(msg).size(18).style(style::toggle(c)).into()
}

fn field<'a>(placeholder: &str, value: &str, width: f32, msg: fn(String) -> Message, c: Colors) -> Element<'a, Message> {
    text_input(placeholder, value).on_input(msg).on_submit(Message::CloseSettings).size(12.5).padding([7, 10]).width(width).style(style::input(c)).into()
}

/// A segmented choice of numbers (1 2 4 8 16).
fn numbers<'a>(choices: Vec<usize>, current: &str, msg: fn(usize) -> Message, c: Colors) -> Element<'a, Message> {
    let current: Option<usize> = current.trim().parse().ok();
    let r = choices.into_iter().fold(row![].spacing(2), |r, n| {
        let on = current == Some(n);
        r.push(
            button(container(text(n.to_string()).size(12).font(if on { style::SEMIBOLD } else { style::INTER })).center_x(Fill))
                .width(30)
                .padding([4, 0])
                .style(style::choice(c, on, 5.0))
                .on_press(msg(n)),
        )
    });
    container(r).padding(3).style(style::segmented(c)).into()
}

// ---------- tabs ----------

fn general(m: &Model, c: Colors) -> Element<'_, Message> {
    let d = &m.draft;
    let quality = pick_list(quality_choices(), Some(Quality(d.preferred_quality.clone())), |q: Quality| Message::DraftQuality(q.0))
        .text_size(12.5)
        .padding([6, 10])
        .style(style::pick(c))
        .menu_style(style::menu(c));
    column![
        section(
            "Downloads",
            vec![
                line("Default folder", "New downloads are saved here unless a category folder applies", field("Folder", &d.download_dir, 260.0, Message::DraftDir, c), c),
                line("Sort into category folders", "Videos, Music, Images, Archives, Documents, Programs", switch(d.sort_into_folders, Message::DraftSort, c), c),
            ],
            c,
        ),
        section(
            "Behaviour",
            vec![
                line("Start downloads immediately", "Otherwise new items wait, paused, until you start them", switch(d.start_immediately, Message::DraftStart, c), c),
                line("Watch the clipboard for links", "A copied link pops up in the corner, ready to download", switch(d.clipboard_watch, Message::DraftClipboard, c), c),
                line("Notify when downloads finish", "A Windows notification when a download finishes or fails", switch(d.notify, Message::DraftNotify, c), c),
            ],
            c,
        ),
        section(
            "Videos",
            vec![
                line("Quality", "Picked for you, or pre-selected in the quality list", quality.into(), c),
                line("Ask every time", "Show the quality list for every video link", switch(d.ask_quality, Message::DraftAsk, c), c),
                line("Subtitles", "Fetch subtitles (also auto-generated ones) and embed them in the video", switch(d.subtitles, Message::DraftSubtitles, c), c),
                line(
                    "Subtitle languages",
                    "Language codes, comma separated: en.* = all English, hi = Hindi, all = everything",
                    field("en.*", &d.subtitle_langs, 120.0, Message::DraftSubtitleLangs, c),
                    c,
                ),
            ],
            c,
        ),
    ]
    .spacing(18)
    .into()
}

fn appearance(m: &Model, c: Colors) -> Element<'_, Message> {
    let current = theme::parse_hex(m.accent_hex());
    let swatches = SWATCHES.iter().fold(row![].spacing(8).align_y(Alignment::Center), |r, (name, hex)| {
        let color = theme::parse_hex(hex).unwrap_or(Color::WHITE);
        let chosen = current == Some(color);
        let _ = name;
        r.push(button(Space::new()).width(20).height(20).padding(0).style(style::swatch(color, chosen, c.ink)).on_press(Message::DraftAccent((*hex).to_string())))
    });
    let invalid = theme::parse_hex(&m.draft.accent).is_none();
    let desc = if invalid { "Type a hex value like #ff9f0a" } else { "Drag in the square, or type any hex value. Text on the accent switches between dark and light by itself" };
    // The square, the strip and the hex box all edit the same colour (`Model::type_accent`).
    let picker = column![
        color_picker::square(m.accent_hsv, PICKER_WIDTH, 146.0, c.surface, c),
        color_picker::hue_strip(m.accent_hsv, PICKER_WIDTH, 20.0, c.surface, c),
    ]
    .spacing(2);
    let logo = iced::widget::image(super::icon::logo(c.accent)).width(34).height(34).opacity(c.alpha);
    let side = column![
        text("Custom colour").size(13).font(style::MEDIUM),
        small(desc, c.text3),
        Space::new().height(4),
        field("#ff9f0a", &m.draft.accent, 120.0, Message::DraftAccent, c),
        Space::new().height(4),
        row![logo, small("The logo, file tiles and the browser extension follow it", c.text3)].spacing(10).align_y(Alignment::Center),
    ]
    .spacing(4)
    .padding(iced::Padding { top: 8.0, right: 8.0, ..Default::default() })
    .width(Fill);
    column![section(
        "Accent colour",
        vec![
            line("Accent", "Used for progress, links and highlights", swatches.into(), c),
            row![picker, side].spacing(12).padding([6, 6]).into(),
        ],
        c,
    )]
    .into()
}

/// The colour square's and hue strip's width in Appearance.
const PICKER_WIDTH: f32 = 248.0;

fn connections(m: &Model, c: Colors) -> Element<'_, Message> {
    let d = &m.draft;
    let per = view::segment_choices(&[1, 2, 4, 8, 16], d.connections.trim().parse().unwrap_or(8));
    let at_once = view::segment_choices(&[1, 2, 3, 4, 5], d.max_concurrent.trim().parse().unwrap_or(3));
    column![
        section(
            "Speed",
            vec![
                line("Connections per download", "More connections are faster, but some servers limit them", numbers(per, &d.connections, Message::DraftConnections, c), c),
                line("Downloads at the same time", "The rest wait in their queue", numbers(at_once, &d.max_concurrent, Message::DraftMax, c), c),
            ],
            c,
        ),
        section(
            "Reliability",
            vec![
                line(
                    "Retry failed downloads",
                    "After a timeout or a busy server: tries again after 10 s, 30 s and 90 s",
                    switch(d.auto_retry, Message::DraftAutoRetry, c),
                    c,
                ),
                line(
                    "Keep sharing torrents",
                    "After a torrent finishes, keep uploading it to others (off: Snag stops when done)",
                    switch(d.keep_sharing, Message::DraftKeepSharing, c),
                    c,
                ),
            ],
            c,
        ),
    ]
    .spacing(18)
    .into()
}

fn speed(m: &Model, c: Colors) -> Element<'_, Message> {
    let d = &m.draft;
    let limit = row![field("0", &d.speed_limit_kbps, 90.0, Message::DraftLimit, c), small("KB/s", c.text2)].spacing(8).align_y(Alignment::Center);
    let mut page = column![section("Speed limit", vec![line("Limit download speed", "Shared across all active downloads. 0 = no limit", limit.into(), c)], c)].spacing(18);

    for (i, q) in m.queue_drafts.iter().enumerate() {
        let name = text_input("Name", &q.name).on_input(move |v| Message::QueueName(i, v)).size(12.5).padding([7, 10]).width(200).style(style::input(c));
        let mut rows: Vec<Element<'_, Message>> = vec![line("Name", "", name.into(), c)];
        if q.id != 0 {
            let max = text_input("1", &q.max_concurrent).on_input(move |v| Message::QueueMax(i, v)).size(12.5).padding([7, 10]).width(60).style(style::input(c));
            rows.push(line("Downloads at once", "1 to 10", max.into(), c));
        }
        rows.push(line("Run on a schedule", "Outside its window the queue waits", toggler(q.scheduled).on_toggle(move |v| Message::QueueScheduled(i, v)).size(18).style(style::toggle(c)).into(), c));
        if q.scheduled {
            let time = |value: &str, ph: &str, msg: Box<dyn Fn(String) -> Message>| {
                text_input(ph, value).on_input(msg).size(12.5).padding([7, 10]).width(110).style(style::input(c))
            };
            rows.push(line("Start at", "24-hour time, e.g. 23:00", time(&q.start, "23:00", Box::new(move |v| Message::QueueStart(i, v))).into(), c));
            rows.push(line("Stop at", "Unfinished items pause and continue next time. Empty = midnight", time(&q.stop, "until midnight", Box::new(move |v| Message::QueueStop(i, v))).into(), c));
            let days = DAY_LETTERS.iter().enumerate().fold(row![].spacing(2), |r, (day, letter)| {
                let on = q.days[day];
                r.push(
                    button(container(text(*letter).size(12).font(if on { style::SEMIBOLD } else { style::INTER })).center_x(Fill))
                        .width(26)
                        .padding([4, 0])
                        .style(style::choice(c, on, 5.0))
                        .on_press(Message::QueueDay(i, day, !on)),
                )
            });
            rows.push(line("Days", "", container(days).padding(3).style(style::segmented(c)).into(), c));
        }
        if q.id != 0 {
            rows.push(line(
                "Delete queue",
                "Its downloads move to Main",
                button(row![icon(Icon::Trash, 12), text("Delete").size(12)].spacing(6).align_y(Alignment::Center))
                    .style(style::danger(c))
                    .padding([6, 10])
                    .on_press(Message::DeleteQueue(i))
                    .into(),
                c,
            ));
        }
        let label = if q.id == 0 { "Queue: Main".to_string() } else { format!("Queue: {}", if q.name.trim().is_empty() { "new" } else { q.name.trim() }) };
        page = page.push(section(&label, rows, c));
    }
    page = page.push(
        button(row![icon(Icon::Plus, 13), text("Add queue").size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
            .style(style::outline(c))
            .padding([7, 12])
            .on_press(Message::AddQueue),
    );
    page.into()
}

fn extension<'a>(m: &'a Model, phone: Option<(&'a str, &'a iced::widget::qr_code::Data)>, c: Colors) -> Element<'a, Message> {
    let listening = m.bridge_status.starts_with("Listening");
    let status = row![
        container(Space::new()).width(7).height(7).style(style::tag(if listening { c.success } else { c.danger }, c.text)),
        text(if listening { "Ready" } else { "Unavailable" }).size(12.5).color(if listening { c.success } else { c.danger }),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    let token = &m.settings.extension_token;
    let masked = if token.chars().count() > 12 {
        let head: String = token.chars().take(8).collect();
        let tail: String = token.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        format!("{head}••••••••{tail}")
    } else {
        token.clone()
    };
    let code = row![
        container(text(masked).size(12).font(style::MONO)).padding([7, 10]).style(move |t| {
            let mut s = style::card(c)(t);
            s.background = Some(c.canvas.into());
            s
        }),
        button(row![icon(Icon::Copy, 13), text("Copy").size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
            .style(style::secondary(c))
            .padding([7, 12])
            .on_press(Message::CopyToken),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let regenerate = button(row![icon(Icon::ArrowRight, 13), text("Regenerate").size(12.5)].spacing(6).align_y(Alignment::Center))
        .style(style::outline(c))
        .padding([7, 12])
        .on_press(Message::NewToken);
    let mut phone_rows = vec![line(
        "Send from your phone",
        "A page on your home network: scan the code, then paste or share links on the phone",
        toggler(m.settings.phone_sharing).on_toggle(Message::PhoneSharing).size(18).style(style::toggle(c)).into(),
        c,
    )];
    if let Some((link, qr)) = phone {
        let code = iced::widget::qr_code(qr).cell_size(4).style(move |_| iced::widget::qr_code::Style { cell: Color::BLACK, background: Color::WHITE });
        let card = row![
            container(code).padding(8).style(move |_| container::Style { background: Some(Color::WHITE.into()), border: iced::Border { radius: 8.0.into(), ..Default::default() }, ..Default::default() }),
            column![
                text("Scan with your phone's camera").size(13).font(style::MEDIUM),
                small(link.to_string(), c.text3),
                small("Same Wi-Fi as this PC. Anyone with this link can send downloads, so keep it to yourself.", c.text3),
            ]
            .spacing(6)
            .width(Fill),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .padding([10, 14]);
        phone_rows.push(card.into());
    }
    column![
        section(
            "Connection",
            vec![
                line("Status", &m.bridge_status, status.into(), c),
                line("Pairing code", "Paste this once into the extension's popup", code.into(), c),
                line("New code", "Disconnects the extension and your phone until you use the new one", regenerate.into(), c),
            ],
            c,
        ),
        section("Phone", phone_rows, c),
    ]
    .spacing(18)
    .into()
}

fn tools(m: &Model, c: Colors) -> Element<'_, Message> {
    let update = button(row![icon(Icon::Download, 13), text(if m.updating_ytdlp { "Updating…" } else { "Update" }).size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
        .style(style::secondary(c))
        .padding([7, 12])
        .on_press_maybe((!m.updating_ytdlp).then_some(Message::UpdateYtdlp));
    let open = button(row![icon(Icon::FolderOpen, 13), text("Open folder").size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
        .style(style::secondary(c))
        .padding([7, 12])
        .on_press(Message::OpenDataFolder);
    let done = m.items.iter().filter(|i| i.status == Status::Done).count();
    let clear = button(text("Clear finished").size(12.5))
        .style(style::outline(c))
        .padding([7, 12])
        .on_press_maybe((done > 0).then_some(Message::ClearFinished));
    column![
        section(
            "Video tools",
            vec![line("yt-dlp", "Downloads from 1,000+ sites. Updates itself weekly; update now if a site stops working", update.into(), c)],
            c,
        ),
        section(
            "Data",
            vec![
                line("App data", r"%APPDATA%\Snag · state.json, tools", open.into(), c),
                line("Download history", &format!("{done} finished item{} · files stay on disk", if done == 1 { "" } else { "s" }), clear.into(), c),
            ],
            c,
        ),
    ]
    .spacing(18)
    .width(Length::Fill)
    .into()
}
