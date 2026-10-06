//! The Add download / Add playlist sheet (Figma frames 02–04).

use super::icon::{Icon, icon};
use super::style;
use super::theme::Colors;
use super::small;
use crate::choices::{OptionChoice, QueueChoice};
use crate::format;
use crate::state::{MediaTab, Model, Picker};
use crate::update::Message;
use crate::view;
use iced::widget::{Space, button, checkbox, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Color, Element, Fill, Length};
use rdm_core::{Category, MediaFormat};

pub fn view<'a>(m: &'a Model, p: &'a Picker, c: Colors) -> Element<'a, Message> {
    let playlist = !p.info.entries.is_empty();
    let host = view::host(&p.url);
    let (title, subtitle) = if playlist {
        ("Add playlist", format!("{} · {} videos · {host}", view::ellipsize(&p.info.title, 40), p.info.entries.len()))
    } else {
        ("Add download", "Pick quality and where to save it".to_string())
    };
    let close = button(container(icon(Icon::X, 13)).center(Fill)).width(28).height(28).padding(0).style(style::secondary(c)).on_press(Message::CancelPick);
    let head = row![column![text(title).size(16).font(style::SEMIBOLD), small(subtitle, c.text3)].spacing(2).width(Fill), close].align_y(Alignment::Start);

    let mut sheet = column![head].spacing(14);
    if playlist {
        sheet = sheet.push(playlist_list(p, c));
    } else {
        sheet = sheet.push(title_card(m, p, &host, c));
        if p.has_both_tabs() {
            sheet = sheet.push(tabs(p, c));
        }
        sheet = sheet.push(options(p, c));
    }

    // Save to + Queue.
    let category = match p.info.options.get(p.choice).map(|o| &o.format) {
        Some(MediaFormat::AudioMp3) => Category::Music,
        _ => Category::Video,
    };
    let folder = view::save_dir(&m.settings, category).display().to_string();
    let save_to = container(row![text(view::ellipsize_left(&folder, 60)).size(12.5).width(Fill), icon(Icon::FolderOpen, 14).color(c.text3)].align_y(Alignment::Center))
        .padding([8, 10])
        .clip(true)
        .style(move |t| {
            let mut s = style::card(c)(t);
            s.background = Some(c.canvas.into());
            s
        });
    let mut where_row = row![column![small("Save to", c.text2), save_to].spacing(6).width(Fill)].spacing(10);
    if m.queues.len() > 1 {
        let choices: Vec<QueueChoice> = m.queues.iter().map(|q| QueueChoice { id: q.id, name: q.name.clone() }).collect();
        let current = choices.iter().find(|q| q.id == p.queue).cloned();
        let pick = pick_list(choices, current, |q: QueueChoice| Message::PickQueue(q.id))
            .text_size(12.5)
            .padding([8, 10])
            .width(170)
            .style(style::pick(c))
            .menu_style(style::menu(c));
        where_row = where_row.push(column![small("Queue", c.text2), pick].spacing(6));
    }
    sheet = sheet.push(where_row);

    // Footer: hint + action.
    let requests = p.requests().len();
    let hint = match (playlist, category) {
        (true, _) => format!("{requests} of {} videos · one quality for all", p.info.entries.len()),
        (false, Category::Music) => "Audio is extracted with ffmpeg".to_string(),
        _ => "Video and audio are merged with ffmpeg".to_string(),
    };
    let label = match (playlist, m.queues.iter().find(|q| q.id == p.queue).filter(|q| q.id != 0)) {
        (true, None) => format!("Download {requests} video{}", if requests == 1 { "" } else { "s" }),
        (_, Some(q)) => format!("Add to {}", view::ellipsize(&q.name, 18)),
        (false, None) => "Download now".to_string(),
    };
    let go = button(row![icon(Icon::Download, 13), text(label).size(12.5).font(style::SEMIBOLD)].spacing(7).align_y(Alignment::Center))
        .style(style::primary(c))
        .padding([8, 14])
        .on_press_maybe((requests > 0).then_some(Message::DownloadPicked));
    let cancel = button(text("Cancel").size(12.5).font(style::SEMIBOLD)).style(style::secondary(c)).padding([8, 14]).on_press(Message::CancelPick);
    sheet = sheet.push(row![small(hint, c.text3).width(Fill), cancel, go].spacing(8).align_y(Alignment::Center));

    container(sheet).width(560).padding(20).style(style::sheet(c)).into()
}

fn title_card<'a>(m: &'a Model, p: &'a Picker, host: &str, c: Colors) -> Element<'a, Message> {
    let cached = p.info.thumbnail.as_ref().and_then(|u| m.thumbs.get(u));
    let thumb: Element<'a, Message> = match cached {
        Some(path) => iced::widget::image(iced::widget::image::Handle::from_path(path))
            .width(136)
            .height(76)
            .content_fit(iced::ContentFit::Cover)
            .border_radius(7.0)
            .into(),
        None => container(icon(Icon::PlayFill, 26).color(Color::WHITE))
            .width(136)
            .height(76)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(style::tile(Color::from_rgb8(0x8a, 0x55, 0x14), Color::from_rgb8(0x24, 0x1b, 0x12), c.line))
            .into(),
    };
    let thumb = super::list::with_duration(thumb, p.info.duration, 136.0, 76.0);
    let mut meta = host.to_string();
    if let Some(d) = p.info.duration {
        meta.push_str(&format!("  ·  {}", view::duration_label(d)));
    }
    let best = p.info.options.first().map(|o| o.label.clone()).unwrap_or_default();
    let detected = container(row![icon(Icon::FilmStrip, 11), text(format!("Video detected · up to {best}")).size(11)].spacing(5).align_y(Alignment::Center))
        .padding([2, 7])
        .style(style::tag(c.accent_soft, c.accent));
    let info = column![text(&p.info.title).size(14).font(style::SEMIBOLD), small(meta, c.text2), detected].spacing(5).width(Fill);
    container(row![thumb, info].spacing(14).align_y(Alignment::Center)).padding(10).style(style::card(c)).into()
}

fn tabs<'a>(p: &'a Picker, c: Colors) -> Element<'a, Message> {
    let tab = |tab: MediaTab, glyph: Icon, label: &'static str| {
        let on = p.tab == tab;
        button(container(row![icon(glyph, 14), text(label).size(12.5).font(if on { style::SEMIBOLD } else { style::INTER })].spacing(7).align_y(Alignment::Center)).center_x(Fill))
            .width(Fill)
            .padding([7, 10])
            .style(style::choice(c, on, 7.0))
            .on_press(Message::PickTab(tab))
    };
    container(row![tab(MediaTab::Video, Icon::FilmStrip, "Video"), tab(MediaTab::Audio, Icon::MusicNotes, "Audio only")].spacing(4))
        .padding(3)
        .style(style::segmented(c))
        .into()
}

fn options<'a>(p: &'a Picker, c: Colors) -> Element<'a, Message> {
    let list = p.visible_options().into_iter().fold(column![].spacing(4), |col, (i, o)| {
        let chosen = p.choice == i;
        let radio = container(if chosen { container(Space::new()).width(8).height(8).style(style::tag(c.accent, c.accent)) } else { container(Space::new()) })
            .center(16)
            .style(move |_| container::Style {
                border: iced::Border { color: if chosen { c.accent } else { c.line_strong }, width: 1.5, radius: 8.0.into() },
                ..Default::default()
            });
        let mut label = row![radio, text(&o.label).size(13.5).font(style::MEDIUM)].spacing(12).align_y(Alignment::Center);
        if let MediaFormat::Video { max_height } = o.format {
            let badge = match max_height {
                h if h >= 2160 => Some("4K"),
                h if h >= 1440 => Some("2K"),
                _ => None,
            };
            if let Some(b) = badge {
                label = label.push(container(text(b).size(10).font(style::SEMIBOLD)).padding([1, 5]).style(style::tag(c.raised, c.text2)));
            }
        }
        let what = match o.format {
            MediaFormat::Video { .. } => "MP4",
            MediaFormat::AudioMp3 => "MP3",
        };
        let size = o.approx_size.map(|s| format!("≈ {}", format::size(s))).unwrap_or_default();
        let row = row![label, Space::new().width(Fill), small(what, c.text3), text(size).size(12).font(style::MONO).color(c.text2)].spacing(12).align_y(Alignment::Center);
        col.push(button(row).width(Fill).padding([9, 12]).style(style::option(c, chosen)).on_press(Message::PickOption(i)))
    });
    // Many qualities (8K…144p) would push the buttons off a short window: cap the list.
    let height = if p.visible_options().len() > 5 { Length::Fixed(5.0 * 46.0) } else { Length::Shrink };
    scrollable(list).height(height).style(style::scroll(c)).into()
}

fn playlist_list<'a>(p: &'a Picker, c: Colors) -> Element<'a, Message> {
    let all = p.selected_count() == p.selected.len();
    let choices: Vec<OptionChoice> = p.info.options.iter().enumerate().map(|(index, o)| OptionChoice { index, label: o.label.clone() }).collect();
    let current = choices.iter().find(|o| o.index == p.choice).cloned();
    let quality = pick_list(choices, current, |o: OptionChoice| Message::PickOption(o.index))
        .text_size(12.5)
        .padding([6, 10])
        .style(style::pick(c))
        .menu_style(style::menu(c));
    let head = row![
        checkbox(all).label("Select all").on_toggle(Message::PickAll).size(16).text_size(13).style(style::check(c)),
        small(format!("{} of {}", p.selected_count(), p.selected.len()), c.text3),
        Space::new().width(Fill),
        small("Quality for all", c.text3),
        quality,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let rows = p.info.entries.iter().enumerate().fold(column![].spacing(2), |col, (i, e)| {
        let on = p.selected.get(i).copied().unwrap_or(false);
        let number = container(text((i + 1).to_string()).size(11).font(style::SEMIBOLD))
            .width(58)
            .height(32)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(style::tile(Color::from_rgb8(0x2c, 0x4a, 0x6e), Color::from_rgb8(0x1b, 0x22, 0x2e), c.line));
        let line = row![
            checkbox(on).on_toggle(move |_| Message::PickEntry(i)).size(16).style(style::check(c)),
            number,
            text(view::ellipsize(&e.title, 70)).size(13).color(if on { c.text } else { c.text3 }),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        col.push(container(line).width(Fill).padding([6, 10]))
    });
    let list = container(scrollable(rows.width(Fill)).width(Fill).height(Length::Fixed(300.0)).style(style::scroll(c))).width(Fill).padding(4).style(style::card(c));
    column![head, list].spacing(12).into()
}
