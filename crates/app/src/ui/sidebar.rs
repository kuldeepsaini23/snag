//! Library categories and queues, with counts; sources and watched channels fold into "More".

use super::icon::{Icon, bold, filled};
use super::style;
use super::theme::Colors;
use super::tiny;
use crate::state::{Model, SettingsTab};
use crate::update::Message;
use crate::view::{CATEGORIES, Library};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Fill};
use rdm_core::Category;

/// The key of entries that are never selected (they open Settings).
const NEVER: u32 = u32::MAX;

/// `lit(key)`: how lit each entry is (the highlight crosses over when the selection moves; keys
/// as in `view::sidebar_key`).
pub fn view<'a>(m: &'a Model, lit: impl Fn(u32) -> f32, c: Colors) -> Element<'a, Message> {
    let counts = m.counts();
    let entry = |glyph: Icon, label: String, trailing: Element<'static, Message>, key: u32, msg: Message| {
        let amount = lit(key);
        // The selected item's icon is filled (and turns accent as it lights up), the rest Bold.
        let color = style::mix(c.text2, c.accent, amount);
        let glyph = if amount > 0.5 { filled(glyph, 16).color(color) } else { bold(glyph, 16).color(color) };
        let content = row![glyph, text(label).size(13).width(Fill), trailing].spacing(10).align_y(Alignment::Center);
        button(content).style(style::choice_lit(c, amount, 7.0)).padding([7, 10]).width(Fill).on_press(msg)
    };
    let count = |n: usize| -> Element<'static, Message> { tiny(n.to_string(), c.text3).into() };
    let on_off = |on: bool| -> Element<'static, Message> { tiny(if on { "On" } else { "Off" }, if on { c.accent } else { c.text3 }).into() };

    let mut list = column![heading("Library", c)].spacing(1);
    list = list.push(entry(Icon::Folder, "All downloads".into(), count(counts.all), 0, Message::SetLibrary(Library::All)));
    for (k, cat) in CATEGORIES.into_iter().enumerate() {
        let (glyph, name) = match cat {
            Category::Video => (Icon::FilmStrip, "Videos"),
            Category::Music => (Icon::MusicNotes, "Music"),
            Category::Image => (Icon::Image, "Images"),
            Category::Archive => (Icon::FileZip, "Archives"),
            Category::Document => (Icon::FileText, "Documents"),
            _ => (Icon::AppWindow, "Programs"),
        };
        let lib = Library::Category(cat);
        list = list.push(entry(glyph, name.into(), count(counts.category(cat)), 10 + k as u32, Message::SetLibrary(lib)));
    }
    // By how they download, not what they hold (a torrent's movie is also under Videos).
    for (glyph, name, n, key, lib) in [(Icon::Magnet, "Torrents", counts.torrents, 20, Library::Torrents), (Icon::Browser, "Web pages", counts.pages, 21, Library::Pages)] {
        list = list.push(entry(glyph, name.into(), count(n), key, Message::SetLibrary(lib)));
    }
    list = list.push(entry(Icon::ChartBar, "Stats".into(), Space::new().into(), 1, Message::OpenStats));

    // With only the main queue, "Main" would just repeat "All downloads".
    if m.queues.len() > 1 {
        list = list.push(heading("Queues", c));
        for q in &m.queues {
            let glyph = if q.id == 0 {
                Icon::Queue
            } else if q.schedule.is_some() {
                Icon::Clock
            } else {
                Icon::ListNumbers
            };
            let lib = Library::Queue(q.id);
            list = list.push(entry(glyph, q.name.clone(), count(counts.queue(q.id)), 100 + q.id, Message::SetLibrary(lib)));
        }
    }

    // Sources and watched channels: set once, rarely looked at.
    let caret = bold(if m.more_open { Icon::CaretDown } else { Icon::CaretRight }, 11).color(c.text3);
    let more = row![caps("More", c), Space::new().width(Fill), caret].align_y(Alignment::Center);
    list = list.push(Space::new().height(10));
    list = list.push(button(more).style(style::ghost(c)).padding([4, 10]).width(Fill).on_press(Message::ToggleMore));
    if m.more_open {
        for w in &m.watches {
            let remove = button(bold(Icon::X, 11)).style(style::ghost(c)).padding(3).on_press(Message::RemoveWatch(w.id));
            let row = row![bold(Icon::FilmStrip, 16).color(c.text2), text(w.name.clone()).size(13).width(Fill).wrapping(iced::widget::text::Wrapping::None), remove]
                .spacing(10)
                .align_y(Alignment::Center);
            list = list.push(container(row).padding([4, 10]).clip(true));
        }
        let extension_on = m.bridge_status.starts_with("Listening");
        list = list.push(entry(Icon::Browser, "Chrome extension".into(), on_off(extension_on), NEVER, Message::OpenSettings(SettingsTab::Extension)));
        list = list.push(entry(Icon::ClipboardText, "Clipboard watch".into(), on_off(m.settings.clipboard_watch), NEVER, Message::OpenSettings(SettingsTab::General)));
    }

    container(scrollable(list.padding([4, 0])).style(style::scroll(c)).height(Fill))
        .width(208)
        .height(Fill)
        .padding(8)
        .style(style::panel(c))
        .into()
}

fn caps<'a>(label: &str, c: Colors) -> iced::widget::Text<'a> {
    text(label.to_uppercase()).size(10.5).font(style::SEMIBOLD).color(Color { a: 0.42, ..c.text })
}

fn heading<'a>(label: &str, c: Colors) -> Element<'a, Message> {
    column![Space::new().height(10), container(caps(label, c)).padding([4, 10])].into()
}
