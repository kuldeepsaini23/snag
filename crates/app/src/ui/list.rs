//! The centre panel: URL bar, "Downloading" and "Recent" groups, empty state.

use super::icon::{Icon, icon};
use super::style;
use super::theme::Colors;
use super::small;
use crate::format;
use crate::state::{Model, SettingsTab, file_missing};
use crate::update::{Message, url_input};
use crate::view;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Fill, Length};
use rdm_core::{Category, Item, Kind, MediaFormat, Status};

pub fn view(m: &Model, c: Colors) -> Element<'_, Message> {
    let mut page = column![url_bar(m, c)].spacing(10);
    page = page.push(container(text(view::url_hint(m)).size(12).color(c.text3)).padding([0, 2]));

    if m.items.is_empty() {
        page = page.push(empty_state(c));
    } else {
        let (down, recent) = m.visible();
        let nothing = down.is_empty() && recent.is_empty();
        let mut groups = column![].spacing(4);
        if !down.is_empty() {
            let speed: u64 = down.iter().map(|i| i.speed_bps).sum();
            let n = down.len();
            groups = groups.push(group_title("Downloading", format!("{n} item{} · {}", plural(n), format::speed(speed)), c));
            groups = down.into_iter().fold(groups, |g, i| g.push(item_row(m, i, c)));
        }
        if !recent.is_empty() {
            let size: u64 = recent.iter().map(|i| i.total.unwrap_or(i.downloaded)).sum();
            groups = groups.push(group_title("Recent", format!("{} completed · {}", recent.len(), format::size(size)), c));
            groups = recent.into_iter().fold(groups, |g, i| g.push(item_row(m, i, c)));
        }
        if nothing {
            groups = groups.push(container(small("Nothing here matches.", c.text3)).padding([24, 4]));
        }
        page = page.push(scrollable(groups.padding(iced::Padding { right: 10.0, ..Default::default() })).style(style::scroll(c)).height(Fill));
    }

    container(page).width(Fill).height(Fill).padding(14).style(style::panel(c)).into()
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn url_bar(m: &Model, c: Colors) -> Element<'_, Message> {
    let tag = view::link_tag(&m.url);
    let input = text_input("Paste a link, or just copy one anywhere", &m.url)
        .id(url_input())
        .on_input(Message::UrlChanged)
        .on_submit(Message::Add)
        .size(13)
        .padding([10, 4])
        .style(style::bare_input(c));
    let mut bar = row![icon(Icon::Link, 15).color(if tag.is_some() { c.accent } else { c.text3 }), input].spacing(8).align_y(Alignment::Center);
    if let Some(t) = tag {
        bar = bar.push(container(text(t.label()).size(11.5).font(style::MEDIUM)).padding([2, 7]).style(style::tag(c.accent_soft, c.accent)));
    }
    let go = button(icon(Icon::ArrowRight, 14)).style(style::ghost(c)).padding(6).on_press_maybe((!m.probing).then_some(Message::Add));
    container(bar.push(go)).padding([0, 12]).style(style::url_bar(c, tag.is_some())).into()
}

fn group_title<'a>(title: &'a str, meta: String, c: Colors) -> Element<'a, Message> {
    row![text(title).size(13).font(style::SEMIBOLD), text(meta).size(12).color(c.text3)].spacing(8).align_y(Alignment::Center).padding([10, 2]).into()
}

/// The video's thumbnail (with its length), or a coloured square with a kind icon.
pub fn tile<'a>(item: &Item, thumb: Option<&std::path::Path>, width: f32, height: f32, glyph_size: u16, c: Colors) -> Element<'a, Message> {
    if let Some(path) = thumb {
        let picture = iced::widget::image(iced::widget::image::Handle::from_path(path))
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .content_fit(iced::ContentFit::Cover)
            .border_radius(7.0);
        return with_duration(picture.into(), item.duration, width, height);
    }
    let plain = plain_tile(item, width, height, glyph_size, c);
    with_duration(plain, item.duration, width, height)
}

/// Puts "42:18" in the bottom-right corner of a tile.
pub fn with_duration<'a>(tile: Element<'a, Message>, duration: Option<f64>, width: f32, height: f32) -> Element<'a, Message> {
    let Some(secs) = duration else { return tile };
    let badge = container(text(view::duration_label(secs)).size(10).font(style::MONO).color(Color::WHITE))
        .padding([1, 4])
        .style(style::tag(Color::from_rgba(0.0, 0.0, 0.0, 0.65), Color::WHITE));
    let corner = container(badge).width(Length::Fixed(width)).height(Length::Fixed(height)).align_x(Alignment::End).align_y(Alignment::End).padding(4);
    iced::widget::stack![tile, corner].into()
}

fn plain_tile<'a>(item: &Item, width: f32, height: f32, glyph_size: u16, c: Colors) -> Element<'a, Message> {
    let (top, bottom, glyph, fg) = match (&item.kind, item.category) {
        (Kind::Media(MediaFormat::AudioMp3), _) | (_, Category::Music) => {
            (Color::from_rgb8(0x6b, 0x3a, 0x8c), Color::from_rgb8(0x23, 0x18, 0x33), Icon::MusicNote, Color::WHITE)
        }
        (Kind::Media(_), _) | (_, Category::Video) => (Color::from_rgb8(0x8a, 0x55, 0x14), Color::from_rgb8(0x24, 0x1b, 0x12), Icon::PlayFill, Color::WHITE),
        (_, Category::Archive) => (c.raised, c.surface, Icon::FileZip, c.text2),
        (_, Category::Image) => (Color::from_rgb8(0x1f, 0x5c, 0x55), Color::from_rgb8(0x16, 0x26, 0x24), Icon::Image, Color::WHITE),
        (_, Category::Document) => (c.raised, c.surface, if item.name.to_lowercase().ends_with(".pdf") { Icon::FilePdf } else { Icon::FileText }, c.text2),
        (_, Category::Program) => (c.raised, c.surface, Icon::AppWindow, c.text2),
        _ => (c.raised, c.surface, Icon::File, c.text2),
    };
    container(icon(glyph, glyph_size).color(fg))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(style::tile(top, bottom, c.line))
        .into()
}

fn item_row<'a>(m: &'a Model, i: &'a Item, c: Colors) -> Element<'a, Message> {
    let failed = matches!(i.status, Status::Failed(_));
    let missing = file_missing(i);
    let waiting = m.waiting_for_schedule(i);
    let percent = match (i.total, &i.status) {
        (_, Status::Done) => 100.0,
        (Some(t), _) if t > 0 => i.downloaded as f32 * 100.0 / t as f32,
        _ => 0.0,
    };

    // Second line, coloured by state.
    let meta: Element<'a, Message> = if missing {
        small("File missing · download it again", c.danger).wrapping(Wrapping::None).into()
    } else if waiting {
        let start = m.queues.iter().find(|q| q.id == i.queue).and_then(|q| q.schedule.as_ref()).map(|s| crate::queues::fmt_hhmm(s.start)).unwrap_or_default();
        dot_line(format!("Scheduled {start} · {}", view::kind_label(i)), c.accent)
    } else if i.status == Status::Done {
        dot_line(view::row_meta(i), c.success)
    } else if failed {
        let mut text = view::row_meta(i);
        if let Some(note) = view::retry_note(i, chrono::Utc::now().timestamp()) {
            text = format!("{text} · {note}");
        }
        small(view::ellipsize(&text, 90), c.danger).wrapping(Wrapping::None).into()
    } else {
        small(view::row_meta(i), c.text3).wrapping(Wrapping::None).into()
    };

    let mut middle = column![text(view::ellipsize(&i.name, 90)).size(13).font(style::MEDIUM).wrapping(Wrapping::None), meta].spacing(3).width(Fill);
    if !matches!(i.status, Status::Done) {
        let fill = if failed { c.danger } else if i.status == Status::Running { c.accent } else { c.text3 };
        middle = middle.push(container(progress_bar(0.0..=100.0, percent).girth(3).style(style::progress(c.hover, fill))).padding([3, 0]));
    }

    let mono = |s: String, size: f32, color: Color| text(s).size(size).font(style::MONO).color(color);
    let live = matches!(i.kind, Kind::Media(MediaFormat::Live { .. }));
    let (top, bottom): (String, String) = match &i.status {
        Status::Running if live => ("● REC".into(), format::size(i.downloaded)),
        Status::Running => (
            format::speed(i.speed_bps),
            i.total.and_then(|t| format::eta(t.saturating_sub(i.downloaded), i.speed_bps)).map(|e| format!("{e} left")).unwrap_or_default(),
        ),
        Status::Failed(_) if i.retry_at.is_some() => ("Waiting".into(), "auto-retry".into()),
        Status::Failed(_) => ("Error".into(), "retry".into()),
        Status::Paused => ("—".into(), "paused".into()),
        Status::Queued if waiting => ("Scheduled".into(), String::new()),
        Status::Queued => ("—".into(), "queued".into()),
        Status::Done => view::when_label(i.added, chrono::Local::now()),
    };
    let top_color = if failed || (live && i.status == Status::Running) { c.danger } else { c.text2 };
    let right = column![mono(top, 11.5, top_color), mono(bottom, 11.0, c.text3)].align_x(Alignment::End).spacing(2).width(96);

    let (glyph, action) = match &i.status {
        Status::Done if missing => (Icon::ArrowClockwise, Some(Message::Redownload(i.id))),
        Status::Done => (Icon::FolderOpen, i.dest.as_ref().map(|_| Message::ShowInFolder(i.id))),
        Status::Running | Status::Queued if waiting => (Icon::Clock, Some(Message::Pause(i.id))),
        Status::Running if live => (Icon::Square, Some(Message::Pause(i.id))),
        Status::Running | Status::Queued => (Icon::Pause, Some(Message::Pause(i.id))),
        Status::Paused => (Icon::Play, Some(Message::Resume(i.id))),
        Status::Failed(_) => (Icon::ArrowClockwise, Some(Message::Resume(i.id))),
    };
    let act = button(container(icon(glyph, 15)).center(Fill)).width(30).height(30).padding(0).style(style::ghost(c)).on_press_maybe(action);

    let middle = container(middle).width(Fill).clip(true);
    let thumb = i.thumbnail.as_ref().and_then(|u| m.thumbs.get(u)).map(|p| p.as_path());
    let content = row![tile(i, thumb, 76.0, 44.0, 16, c), middle, right, act].spacing(14).align_y(Alignment::Center);
    button(content).width(Fill).padding([9, 10]).style(style::row(c, m.selected == Some(i.id), failed)).on_press(Message::Select(i.id)).into()
}

fn dot_line<'a>(s: String, color: Color) -> Element<'a, Message> {
    row![container(Space::new()).width(7).height(7).style(style::tag(color, color)), text(s).size(11.5).color(color).wrapping(Wrapping::None)]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}

fn empty_state<'a>(c: Colors) -> Element<'a, Message> {
    let badge = container(icon(Icon::TrayDown, 26).color(c.accent)).center(56).style(style::tag(c.accent_soft, c.accent));
    let buttons = row![
        button(text("Add URL").size(12.5).font(style::SEMIBOLD)).style(style::primary(c)).padding([8, 26]).on_press(Message::AddUrl),
        button(row![icon(Icon::Browser, 14), text("Set up Chrome extension").size(12.5).font(style::SEMIBOLD)].spacing(7).align_y(Alignment::Center))
            .style(style::secondary(c))
            .padding([8, 14])
            .on_press(Message::OpenSettings(SettingsTab::Extension)),
    ]
    .spacing(8);
    let content = column![
        badge,
        Space::new().height(6),
        text("No downloads yet").size(19).font(style::SEMIBOLD),
        text("Copy a link from anywhere, paste it above, or click a download in Chrome.\nVideos from 1,000+ sites work too.").size(13).color(c.text2).center(),
        Space::new().height(6),
        buttons,
    ]
    .spacing(8)
    .align_x(Alignment::Center);
    container(content).width(Fill).height(Fill).center(Fill).into()
}
