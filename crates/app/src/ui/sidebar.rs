//! Library categories, queues and sources, with counts.

use super::icon::{Icon, icon};
use super::style;
use super::theme::Colors;
use super::tiny;
use crate::state::{Model, SettingsTab};
use crate::update::Message;
use crate::view::{CATEGORIES, Library};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Fill};
use rdm_core::Category;

pub fn view(m: &Model, c: Colors) -> Element<'_, Message> {
    let counts = m.counts();
    let entry = |glyph: Icon, label: String, trailing: Element<'static, Message>, selected: bool, msg: Message| {
        let content = row![
            icon(glyph, 15).color(if selected { c.text } else { c.text2 }),
            text(label).size(13).width(Fill),
            trailing,
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        button(content).style(style::choice(c, selected, 7.0)).padding([7, 10]).width(Fill).on_press(msg)
    };
    let count = |n: usize| -> Element<'static, Message> { tiny(n.to_string(), c.text3).into() };
    let on_off = |on: bool| -> Element<'static, Message> { tiny(if on { "On" } else { "Off" }, if on { c.accent } else { c.text3 }).into() };

    let mut list = column![heading("Library", c)].spacing(1);
    list = list.push(entry(Icon::Folder, "All downloads".into(), count(counts.all), m.library == Library::All, Message::SetLibrary(Library::All)));
    for cat in CATEGORIES {
        let (glyph, name) = match cat {
            Category::Video => (Icon::FilmStrip, "Videos"),
            Category::Music => (Icon::MusicNotes, "Music"),
            Category::Archive => (Icon::FileZip, "Archives"),
            Category::Document => (Icon::FileText, "Documents"),
            _ => (Icon::AppWindow, "Programs"),
        };
        let lib = Library::Category(cat);
        list = list.push(entry(glyph, name.into(), count(counts.category(cat)), m.library == lib, Message::SetLibrary(lib)));
    }

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
        list = list.push(entry(glyph, q.name.clone(), count(counts.queue(q.id)), m.library == lib, Message::SetLibrary(lib)));
    }

    list = list.push(heading("Sources", c));
    let extension_on = m.bridge_status.starts_with("Listening");
    list = list.push(entry(Icon::Browser, "Chrome extension".into(), on_off(extension_on), false, Message::OpenSettings(SettingsTab::Extension)));
    list = list.push(entry(Icon::ClipboardText, "Clipboard watch".into(), on_off(m.settings.clipboard_watch), false, Message::OpenSettings(SettingsTab::General)));

    container(scrollable(list.padding([4, 0])).style(style::scroll(c)).height(Fill))
        .width(208)
        .height(Fill)
        .padding(8)
        .style(style::panel(c))
        .into()
}

fn heading<'a>(label: &str, c: Colors) -> Element<'a, Message> {
    let caps = text(label.to_uppercase()).size(10.5).font(style::SEMIBOLD).color(Color { a: 0.42, ..c.text });
    column![Space::new().height(10), container(caps).padding([4, 10])].into()
}
