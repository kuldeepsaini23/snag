//! The right-click menu on a download: the actions that make sense for it, where it was clicked.

use super::icon::{Icon, bold};
use super::style;
use super::theme::Colors;
use crate::update::Message;
use crate::view;
use iced::widget::{button, column, container, row, rule, text};
use iced::{Alignment, Element, Fill};
use rdm_core::{Item, Status};

/// One line in the menu: what it says and what it does. `danger` lines are red.
#[derive(Clone, Debug)]
pub struct Entry {
    pub glyph: Icon,
    pub label: &'static str,
    pub message: Message,
    pub danger: bool,
}

/// The menu for `i`, in the order shown; `None` marks a divider. `on_disk`: the finished file is
/// still there.
pub fn entries(i: &Item, on_disk: bool) -> Vec<Option<Entry>> {
    let id = i.id;
    let entry = |glyph, label, message| Some(Entry { glyph, label, message, danger: false });
    let copy_link = entry(Icon::Link, "Copy link", Message::CopyText(i.url.clone()));
    match (&i.status, view::main_action(i)) {
        (Status::Done, _) if on_disk => vec![
            entry(Icon::Play, "Open", Message::OpenFile(id)),
            entry(Icon::FolderOpen, "Show in folder", Message::ShowInFolder(id)),
            None,
            copy_link,
            entry(Icon::Copy, "Copy file path", Message::CopyText(i.dest.as_ref().map(|d| d.display().to_string()).unwrap_or_default())),
            None,
            entry(Icon::X, "Remove from list", Message::Remove(id)),
            // Like the Delete button: this asks for a second click.
            Some(Entry { glyph: Icon::Trash, label: "Delete file", message: Message::Delete(id), danger: true }),
        ],
        (Status::Done, _) => vec![entry(Icon::ArrowClockwise, "Download again", Message::Redownload(id)), None, copy_link, None, entry(Icon::X, "Remove from list", Message::Remove(id))],
        (_, action) => {
            let control = match action {
                Some(view::MainAction::Pause) => entry(Icon::Pause, "Pause", Message::Pause(id)),
                _ => entry(Icon::Play, "Resume", Message::Resume(id)),
            };
            vec![control, None, copy_link, None, entry(Icon::X, "Cancel download", Message::Cancel(id))]
        }
    }
}

/// The menu itself (240 px wide).
pub fn view<'a>(i: &Item, on_disk: bool, c: Colors) -> Element<'a, Message> {
    let mut list = column![].spacing(2);
    for entry in entries(i, on_disk) {
        list = match entry {
            None => list.push(container(rule::horizontal(1).style(move |_| rule::Style { color: c.line, radius: 0.0.into(), fill_mode: rule::FillMode::Full, snap: true })).padding([3, 6])),
            Some(e) => {
                let color = if e.danger { c.danger } else { c.text2 };
                let label = text(e.label).size(13).width(Fill);
                let label = if e.danger { label.color(c.danger) } else { label };
                let content = row![bold(e.glyph, 15).color(color), label].spacing(10).align_y(Alignment::Center);
                list.push(button(content).width(Fill).padding([7, 10]).style(style::choice(c, false, 7.0)).on_press(Message::Menu(Box::new(e.message))))
            }
        };
    }
    container(list).width(WIDTH).padding(6).style(style::sheet(c)).into()
}

pub const WIDTH: f32 = 240.0;

#[cfg(test)]
mod tests {
    use super::*;
    use rdm_core::{Category, ItemId, Kind};

    fn item(status: Status) -> Item {
        Item {
            id: ItemId(4),
            url: "https://example.com/a.zip".into(),
            name: "a.zip".into(),
            category: Category::Archive,
            status,
            dest: Some(r"C:\dl\a.zip".into()),
            downloaded: 0,
            total: None,
            speed_bps: 0,
            queue: 0,
            added: 0,
            kind: Kind::Http,
            referrer: None,
            work_dir: None,
            thumbnail: None,
            duration: None,
            retry_at: None,
            subtitle_links: Vec::new(),
            subtitle_files: Vec::new(),
        }
    }

    fn labels(entries: &[Option<Entry>]) -> Vec<&'static str> {
        entries.iter().map(|e| e.as_ref().map_or("—", |e| e.label)).collect()
    }

    #[test]
    fn a_finished_download_offers_open_and_its_file() {
        let e = entries(&item(Status::Done), true);
        assert_eq!(labels(&e), ["Open", "Show in folder", "—", "Copy link", "Copy file path", "—", "Remove from list", "Delete file"]);
        assert!(matches!(e[0].as_ref().unwrap().message, Message::OpenFile(ItemId(4))));
        assert!(e[7].as_ref().unwrap().danger, "deleting is red");
        // The file is gone: download it again instead.
        assert_eq!(labels(&entries(&item(Status::Done), false)), ["Download again", "—", "Copy link", "—", "Remove from list"]);
    }

    #[test]
    fn an_unfinished_download_offers_its_controls() {
        assert_eq!(labels(&entries(&item(Status::Running), false)), ["Pause", "—", "Copy link", "—", "Cancel download"]);
        assert_eq!(labels(&entries(&item(Status::Paused), false)), ["Resume", "—", "Copy link", "—", "Cancel download"]);
        let failed = entries(&item(Status::Failed("x".into())), false);
        assert!(matches!(failed[0].as_ref().unwrap().message, Message::Resume(ItemId(4))), "retry");
    }
}
