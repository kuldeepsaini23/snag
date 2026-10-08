//! The toolbar's "?" menu and its sheets: keyboard shortcuts, help, what's new, report a bug.

use super::icon::{Icon, bold, icon};
use super::style;
use super::theme::Colors;
use super::{small, tiny};
use crate::changelog;
use crate::state::{Info, Model};
use crate::update::Message;
use iced::widget::{Space, button, checkbox, column, container, row, scrollable, text, text_editor};
use iced::{Alignment, Color, Element, Fill, Length};

/// The menu under the "?" button.
pub fn menu(c: Colors) -> Element<'static, Message> {
    let entry = |glyph: Icon, label: &'static str, hint: &'static str, info: Info| {
        let content = row![bold(glyph, 15).color(c.text2), text(label).size(13).width(Fill), tiny(hint, c.text3)].spacing(10).align_y(Alignment::Center);
        button(content).width(Fill).padding([8, 10]).style(style::choice(c, false, 7.0)).on_press(Message::OpenInfo(info))
    };
    let items = column![
        entry(Icon::Keyboard, "Keyboard shortcuts", "", Info::Shortcuts),
        entry(Icon::Lifebuoy, "Help", "F1", Info::Help),
        entry(Icon::Sparkle, "What's new", "", Info::WhatsNew { since: None }),
        entry(Icon::Bug, "Report a bug", "", Info::BugReport),
        button(row![bold(Icon::ArrowClockwise, 15).color(c.text2), text("Check for updates").size(13).width(Fill)].spacing(10).align_y(Alignment::Center))
            .width(Fill)
            .padding([8, 10])
            .style(style::choice(c, false, 7.0))
            .on_press(Message::CheckUpdateNow),
    ]
    .spacing(2);
    container(items).width(236).padding(6).style(style::sheet(c)).into()
}

/// The open sheet. `bug_text`: what the user typed into the bug report.
pub fn sheet<'a>(m: &'a Model, info: &'a Info, bug_text: &'a text_editor::Content, c: Colors) -> Element<'a, Message> {
    match info {
        Info::Shortcuts => frame("Keyboard shortcuts", "Snag works with the mouse; these save a few clicks", shortcuts(c), Length::Shrink, c),
        Info::Help => frame("Help", "The short version of everything", help(c), Length::Fixed(540.0), c),
        Info::WhatsNew { since } => {
            // After an update: what changed since the version before. From the menu: every version.
            let (title, subtitle) = match since {
                Some(_) => (format!("What's new in Snag {}", changelog::VERSION), "Updated: here's what changed. The ? menu has this list any time"),
                None => ("What's new".to_string(), "Every version of Snag, newest first"),
            };
            frame(&title, subtitle, whats_new(since.as_deref(), c), Length::Fixed(540.0), c)
        }
        Info::BugReport => frame("Report a bug", "Opens a GitHub issue for you to check and send, or just saves a text file. Nothing is sent without you", bug_report(m, bug_text, c), Length::Shrink, c),
    }
}

/// The scrolling body of a sheet (What's new, Help).
pub const SHEET_SCROLL: &str = "sheet-scroll";

/// A sheet with a title, a close button and `body` (scrolls when `height` is fixed).
fn frame<'a>(title: &str, subtitle: &str, body: Element<'a, Message>, height: Length, c: Colors) -> Element<'a, Message> {
    let close = button(container(icon(Icon::X, 13)).center(Fill)).width(28).height(28).padding(0).style(style::secondary(c)).on_press(Message::CloseInfo);
    let head = row![column![text(title.to_string()).size(17).font(style::SEMIBOLD), small(subtitle.to_string(), c.text2)].spacing(3).width(Fill), close];
    let body: Element<'a, Message> = if height == Length::Shrink {
        body
    } else {
        scrollable(container(body).padding(iced::Padding { right: 12.0, ..Default::default() })).id(SHEET_SCROLL).style(style::scroll(c)).height(Fill).into()
    };
    container(column![head, body].spacing(16)).width(580).height(height).padding([20, 22]).style(style::sheet(c)).into()
}

/// A heading in the style of the settings sections.
fn label<'a>(s: &str, c: Colors) -> Element<'a, Message> {
    text(s.to_uppercase()).size(10.5).font(style::SEMIBOLD).color(Color { a: 0.42, ..c.text }).into()
}

/// One point of a list: a small dot, then the text wrapping beside it.
fn point<'a>(s: &str, c: Colors) -> Element<'a, Message> {
    // The dot sits on the first line's middle.
    let dot = container(container(Space::new()).width(5).height(5).style(style::tag(c.text3, c.text3))).padding(iced::Padding { top: 7.0, ..Default::default() });
    row![dot, text(s.to_string()).size(12.5).color(c.text2).width(Fill)].spacing(10).align_y(Alignment::Start).into()
}

const SHORTCUTS: [(&str, &str); 6] = [
    ("Ctrl + K", "Search downloads"),
    ("Enter", "Add the link typed or pasted in the link bar"),
    ("Esc", "Close the open sheet or menu, else deselect"),
    ("F1", "Help"),
    ("Double-click", "On the title bar: maximise or restore the window"),
    ("Alt + F4", "Hide Snag to the tray (it keeps downloading)"),
];

fn shortcuts<'a>(c: Colors) -> Element<'a, Message> {
    let rows = SHORTCUTS.iter().fold(column![].spacing(0), |col, (keys, what)| {
        let keycap = container(text(*keys).size(12).font(style::MONO)).padding([3, 8]).style(style::tag(c.raised, c.text));
        col.push(row![container(keycap).width(210), text(*what).size(12.5).color(c.text2).width(Fill)].spacing(12).align_y(Alignment::Center).padding([8, 12]))
    });
    container(rows).width(Fill).style(style::card(c)).into()
}

const HELP: [(Icon, &str, &[&str]); 5] = [
    (
        Icon::Download,
        "Basics",
        &[
            "Copy a link anywhere and Snag offers it in the corner. Or paste it into the link bar and press Enter.",
            "Video and music links ask for a quality (or MP3); files start at once. Playlists let you tick the videos you want.",
            "Point at a download for Pause, Resume and Cancel. Click it for details, its folder, and Refresh link when an old link has expired.",
            "Closing the window keeps Snag downloading in the tray. Quit from the tray icon.",
        ],
    ),
    (
        Icon::Browser,
        "Browser extension",
        &[
            "Install the Snag extension, press Connect in it, then Allow in Snag. There's no code to copy.",
            "On a video page the Snag button opens the quality menu. Right-click a link or picture for Download with Snag.",
            "After updating Snag or the extension: reload the extension on the browser's Extensions page, then press F5 on tabs that were already open.",
        ],
    ),
    (
        Icon::DeviceMobile,
        "Phone",
        &[
            "Settings → Extension → Send from your phone. Scan the QR code with the phone's camera, on the same Wi-Fi as this PC.",
            "Paste or share a link on that page and it downloads here. Anyone with the link can send downloads, so keep it to yourself.",
        ],
    ),
    (
        Icon::Magnet,
        "Torrents",
        &[
            "Paste a magnet link or a .torrent link like any other link: it downloads in the same list, queues and speed limit.",
            "Snag stops sharing once a torrent finishes, unless Keep sharing torrents is on (Settings → Connections).",
        ],
    ),
    (
        Icon::Wrench,
        "Troubleshooting",
        &[
            "A video site stopped working: Settings → Tools → Update yt-dlp.",
            "Link expired (HTTP 403): select the download and use Refresh link with a fresh link from the page.",
            "The extension says to reload the page, or does nothing: press F5 on the page (after an update, reload the extension first).",
            crate::platform::LOG_HINT,
        ],
    ),
];

fn help<'a>(c: Colors) -> Element<'a, Message> {
    HELP.iter()
        .fold(column![].spacing(18), |col, (glyph, title, points)| {
            let head = row![bold(*glyph, 15).color(c.accent), text(*title).size(14).font(style::SEMIBOLD)].spacing(8).align_y(Alignment::Center);
            col.push(points.iter().fold(column![head].spacing(8), |list, p| list.push(point(p, c))))
        })
        .into()
}

fn whats_new<'a>(since: Option<&str>, c: Colors) -> Element<'a, Message> {
    let releases = changelog::releases_since(changelog::releases(), since);
    let many = releases.len() > 1;
    releases
        .into_iter()
        .fold(column![].spacing(22), |col, r| {
            let mut block = column![].spacing(14);
            if many || since.is_none() {
                let date = r.date.map(|d| format!(" · {d}")).unwrap_or_default();
                block = block.push(text(format!("Snag {}{date}", r.version)).size(14).font(style::SEMIBOLD));
            }
            for s in r.sections {
                let mut list = column![].spacing(8);
                if !s.title.is_empty() {
                    list = list.push(label(&s.title, c));
                }
                block = block.push(s.items.iter().fold(list, |l, item| l.push(point(item, c))));
            }
            col.push(block)
        })
        .into()
}

fn bug_report<'a>(m: &'a Model, bug_text: &'a text_editor::Content, c: Colors) -> Element<'a, Message> {
    let editor = text_editor(bug_text)
        .placeholder("What did you do, what did you expect, and what happened instead?")
        .on_action(Message::BugEdit)
        .height(130)
        .size(12.5)
        .padding(10)
        .style(style::editor(c));
    let diagnostics = column![
        checkbox(m.bug_diagnostics).label("Include diagnostics").on_toggle(Message::BugDiagnostics).size(16).text_size(13).style(style::check(c)),
        container(small(
            crate::platform::DIAGNOSTICS_HINT,
            c.text3
        ))
        .padding(iced::Padding { left: 26.0, ..Default::default() }),
    ]
    .spacing(6);
    let ready = (!m.bug_saving).then_some(());
    let save = button(text("Save to Desktop").size(12.5).font(style::SEMIBOLD))
        .style(style::secondary(c))
        .padding([8, 14])
        .on_press_maybe(ready.map(|_| Message::SaveBugReport(false)));
    let github = button(text(if m.bug_saving { "Saving…" } else { "Report on GitHub" }).size(12.5).font(style::SEMIBOLD))
        .style(style::accent(c))
        .padding([8, 16])
        .on_press_maybe(ready.map(|_| Message::SaveBugReport(true)));
    let buttons = row![
        button(text("Cancel").size(12.5).font(style::SEMIBOLD)).style(style::secondary(c)).padding([8, 14]).on_press(Message::CloseInfo),
        Space::new().width(Fill),
        save,
        github
    ]
    .spacing(8);
    column![text("What happened?").size(13).font(style::MEDIUM), editor, Space::new().height(4), diagnostics, Space::new().height(6), buttons].spacing(8).into()
}
