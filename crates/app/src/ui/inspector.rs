//! The right panel: preview, details table, error card, actions.

use super::icon::{Icon, icon};
use super::list::tile;
use super::style;
use super::theme::Colors;
use crate::choices::QueueChoice;
use crate::format;
use crate::state::{Model, file_missing};
use crate::update::Message;
use crate::view;
use iced::widget::{Space, button, column, container, pick_list, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill};
use rdm_core::{Item, Kind, MediaFormat, Status};

pub fn view<'a>(m: &'a Model, i: &'a Item, c: Colors) -> Element<'a, Message> {
    let quality = match &i.kind {
        Kind::Media(MediaFormat::Video { max_height }) => format!("Up to {max_height}p"),
        Kind::Media(MediaFormat::AudioMp3) => "MP3 audio".into(),
        Kind::Http => "Original file".into(),
    };
    let size = i.total.or((i.downloaded > 0).then_some(i.downloaded)).map(format::bytes).unwrap_or_else(|| "—".into());
    let speed = if i.status == Status::Running { format::speed(i.speed_bps) } else { "—".into() };
    let folder = i.dest.as_ref().and_then(|d| d.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| view::save_dir(&m.settings, i.category));
    let kind = view::kind_label(i);

    let line = |label: &'static str, value: Element<'a, Message>| -> Element<'a, Message> {
        row![text(label).size(12.5).color(c.text2).width(70), container(value).width(Fill).align_x(Alignment::End).clip(true)].align_y(Alignment::Center).padding([8, 10]).into()
    };
    let value = |s: String| -> Element<'a, Message> { text(s).size(12.5).wrapping(text::Wrapping::None).into() };
    let mut table: Vec<Element<'a, Message>> = vec![
        line("Source", value(view::ellipsize(&view::host(&i.url), 30))),
        line("Quality", value(quality)),
        line("Format", value(if kind.is_empty() { "—".into() } else { kind })),
        line("Size", value(size)),
        line("Speed", value(speed)),
        line("Save to", value(view::ellipsize_left(&folder.display().to_string(), 26))),
    ];
    if m.queues.len() > 1 {
        let choices: Vec<QueueChoice> = m.queues.iter().map(|q| QueueChoice { id: q.id, name: q.name.clone() }).collect();
        let current = choices.iter().find(|q| q.id == i.queue).cloned();
        let id = i.id;
        let pick = pick_list(choices, current, move |q: QueueChoice| Message::MoveToQueue(id, q.id))
            .text_size(12)
            .padding([4, 8])
            .style(style::pick(c))
            .menu_style(style::menu(c));
        table.push(line("Queue", pick.into()));
    }
    let table = interleave(table, c);

    let mut details = column![
        tile(i, 252.0, 140.0, 34, c),
        text(view::ellipsize(&i.name, 120)).size(14).font(style::SEMIBOLD).wrapping(text::Wrapping::WordOrGlyph),
        container(table).style(style::card(c)),
    ]
    .spacing(12);
    if let Some(card) = problem(i, c) {
        details = details.push(card);
    }
    // Details scroll on short windows; the buttons below stay on screen.
    let details = container(details).padding(iced::Padding { right: 8.0, ..Default::default() });
    let mut panel = column![scrollable(details).height(Fill).style(style::scroll(c))].spacing(12);

    // Remove keeps the file; Delete sits apart, is red and needs a second click.
    let armed = m.pending_delete == Some(i.id);
    let delete_label = if armed { "Click again to delete" } else { "Delete file" };
    let manage = row![
        button(row![icon(Icon::X, 12), text("Remove from list").size(12)].spacing(6).align_y(Alignment::Center))
            .style(style::ghost(c))
            .padding([6, 8])
            .on_press(Message::Remove(i.id)),
        Space::new().width(Fill),
        button(row![icon(Icon::Trash, 12), text(delete_label).size(12)].spacing(6).align_y(Alignment::Center))
            .style(move |t, s| if armed { style::danger(c)(t, s) } else { style::ghost(c)(t, s) })
            .padding([6, 8])
            .on_press(Message::Delete(i.id)),
    ];
    panel = panel.push(manage);

    let (glyph, label, action) = match view::main_action(i) {
        Some(view::MainAction::Pause) => (Icon::Pause, "Pause", Some(Message::Pause(i.id))),
        Some(view::MainAction::Resume) => (Icon::Play, "Resume", Some(Message::Resume(i.id))),
        Some(view::MainAction::Redownload) => (Icon::ArrowClockwise, "Download again", Some(Message::Redownload(i.id))),
        None => (Icon::CheckCircle, "Finished", None),
    };
    let wide = |glyph: Icon, label: &'a str| container(row![icon(glyph, 13), text(label).size(12.5).font(style::SEMIBOLD)].spacing(7).align_y(Alignment::Center)).center_x(Fill);
    panel = panel.push(
        row![
            button(wide(glyph, label)).width(Fill).padding([8, 10]).style(style::secondary(c)).on_press_maybe(action),
            button(wide(Icon::FolderOpen, "Show in folder"))
                .width(Fill)
                .padding([8, 10])
                .style(style::primary(c))
                .on_press_maybe(i.dest.as_ref().map(|_| Message::ShowInFolder(i.id))),
        ]
        .spacing(8),
    );

    container(panel).width(290).height(Fill).padding(10).style(style::panel(c)).into()
}

/// Hairlines between table rows.
fn interleave<'a>(rows: Vec<Element<'a, Message>>, c: Colors) -> Element<'a, Message> {
    let n = rows.len();
    let mut out = column![];
    for (k, r) in rows.into_iter().enumerate() {
        out = out.push(r);
        if k + 1 < n {
            out = out.push(rule::horizontal(1).style(move |_| rule::Style { color: c.line, radius: 0.0.into(), fill_mode: rule::FillMode::Full, snap: true }));
        }
    }
    out.into()
}

/// A failed download or a missing file: what happened and how to fix it.
fn problem<'a>(i: &'a Item, c: Colors) -> Option<Element<'a, Message>> {
    let (title, body, action, label) = match &i.status {
        Status::Failed(e) => (
            "Download failed",
            format!("{}\n{} downloaded so far is kept.", if e.is_empty() { "The download stopped." } else { e.as_str() }, format::size(i.downloaded)),
            Message::Resume(i.id),
            "Retry",
        ),
        Status::Done if file_missing(i) => ("File missing", "The finished file is no longer on disk.".to_string(), Message::Redownload(i.id), "Download again"),
        _ => return None,
    };
    let card = column![
        row![icon(Icon::WarningCircle, 15).color(c.danger), text(title).size(13).font(style::SEMIBOLD)].spacing(8).align_y(Alignment::Center),
        text(view::ellipsize(&body, 400)).size(12).color(c.text2).wrapping(text::Wrapping::WordOrGlyph),
        button(text(label).size(12).font(style::SEMIBOLD)).style(style::primary(c)).padding([6, 12]).on_press(action),
    ]
    .spacing(8);
    Some(container(card).padding(12).width(Fill).style(style::error_card(c)).into())
}
