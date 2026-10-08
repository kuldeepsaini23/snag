//! The Figma views. Logic lives in `state.rs` / `view.rs` / `update.rs`; this module only draws.

pub mod icon;
mod charts;
mod color_picker;
mod grid;
mod help;
mod inspector;
mod list;
mod picker;
mod popover;
mod context_menu;
mod settings;
#[cfg(debug_assertions)]
pub use help::SHEET_SCROLL;
#[cfg(debug_assertions)] // only the debug snapshot tool scrolls it
pub use settings::SCROLL as SETTINGS_SCROLL;
mod sidebar;
mod stats;
pub mod style;
pub mod theme;
mod toolbar;
mod tour;

use crate::format;
use crate::motion::Motion;
use crate::state::{Model, Screen};
use crate::update::{App, DuplicateChoice, Message};
use crate::view;
use icon::{Icon, icon};
use iced::widget::{Space, button, column, container, float, mouse_area, opaque, row, scrollable, stack, text};
use iced::window::Direction;
use iced::{Alignment, Color, Element, Fill, Length, Padding, Vector, mouse};
use std::time::Instant;
use theme::Colors;

/// The sidebar's and inspector's full widths (they slide between 0 and these).
const SIDEBAR_WIDTH: f32 = 208.0;
const INSPECTOR_WIDTH: f32 = 290.0;
const GAP: f32 = 8.0;

/// Where each animation is at the instant being drawn (0 = closed … 1 = open).
#[derive(Clone, Copy, Debug)]
pub struct Anim {
    /// The tab underline, in tab units.
    pub tab: f32,
    pub sidebar: f32,
    pub inspector: f32,
    pub popover: f32,
    pub toast: f32,
    pub search: f32,
    /// The stats charts rising (0 … 1).
    pub stats: f32,
}

impl Anim {
    fn at(m: &Motion, now: Instant) -> Self {
        Anim {
            tab: m.tab_at(now),
            sidebar: m.open(&m.sidebar, now),
            inspector: m.open(&m.inspector, now),
            popover: m.open(&m.popover, now),
            toast: m.open(&m.toast, now),
            search: m.open(&m.search, now),
            stats: m.open(&m.stats, now),
        }
    }
}

/// The tokens the window draws with: the accent, light or dark, and see-through panels while
/// Mica is behind the window.
pub fn colors(m: &Model) -> Colors {
    let c = theme::colors_for(m.accent_hex(), m.light());
    if m.translucent() { c.translucent() } else { c }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let m = &app.model;
    let c = colors(m);
    let a = Anim::at(&app.motion, app.now);

    let mut body = row![].padding(Padding { top: 0.0, right: GAP, bottom: 0.0, left: GAP }).height(Fill);
    if a.sidebar > 0.0 {
        // Slides in from the left edge: the panel keeps its size, the strip showing it grows.
        let panel = row![sidebar::view(m, |key| app.motion.selection(key, app.now), c), Space::new().width(GAP)];
        body = body.push(slide(panel.into(), (SIDEBAR_WIDTH + GAP) * a.sidebar, true));
    }
    body = body.push(if m.stats_open { stats::view(m, a.stats, c) } else { list::view(m, &app.motion, app.now, c) });
    // Kept on screen while it slides closed (the selection is already gone by then).
    let inspected = m.inspected().or_else(|| app.inspector_item.and_then(|id| m.items.iter().find(|i| i.id == id)));
    if let Some(item) = inspected.filter(|_| a.inspector > 0.0) {
        let panel = row![Space::new().width(GAP), inspector::view(m, item, app.now, c)];
        body = body.push(slide(panel.into(), (INSPECTOR_WIDTH + GAP) * a.inspector, false));
    }
    let base = container(column![toolbar::view(m, a, c), body, footer(m, app.now, c)]).style(style::window(c)).width(Fill).height(Fill);

    // The mouse position, for the right-click menu (see `Message::Cursor`).
    let base = mouse_area(base).on_move(Message::Cursor);
    let mut layers = stack![base].width(Fill).height(Fill);
    if m.speed_open {
        // A click anywhere else closes the popover.
        layers = layers.push(mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::ToggleSpeed));
        // Drops down from the gauge button.
        let pop = float(popover::view(m, c.faded(a.popover))).translate(move |_, _| Vector::new(0.0, -8.0 * (1.0 - a.popover)));
        layers = layers.push(container(opaque(pop)).width(Fill).height(Fill).align_x(Alignment::End).padding(Padding {
            top: 46.0,
            right: 268.0,
            bottom: 0.0,
            left: 0.0,
        }));
    }
    if let Some((id, at)) = m.row_menu
        && let Some(item) = m.items.iter().find(|i| i.id == id)
    {
        let on_disk = item.dest.as_ref().is_some_and(|d| d.exists());
        let menu = context_menu::view(item, on_disk, c);
        // Window coordinates are in physical-ish units; the layout is scaled by UI_SCALE.
        let (w, h) = (m.window.width / theme::UI_SCALE, m.window.height / theme::UI_SCALE);
        let x = at.x.min(w - context_menu::WIDTH - 8.0).max(4.0);
        let y = at.y.min(h - 300.0).max(4.0);
        layers = layers.push(mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseMenu).on_right_press(Message::CloseMenu));
        layers = layers.push(container(opaque(menu)).width(Fill).height(Fill).padding(Padding { top: y, right: 0.0, bottom: 0.0, left: x }));
    }
    if m.help_open {
        // Like the speed popover: a click anywhere else closes it; it drops down from the "?".
        layers = layers.push(mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::ToggleHelp));
        let menu = float(help::menu(c.faded(a.popover))).translate(move |_, _| Vector::new(0.0, -8.0 * (1.0 - a.popover)));
        layers = layers.push(container(opaque(menu)).width(Fill).height(Fill).align_x(Alignment::End).padding(Padding {
            top: 46.0,
            right: 228.0,
            bottom: 0.0,
            left: 0.0,
        }));
    }
    // Sheets open at once: fading and scaling a sheet over a dimmed window repaints all of it
    // every frame, which stutters when the CPU draws.
    let sheet_c = c;
    match (m.screen, &m.picker) {
        (Screen::Picker, Some(p)) => layers = layers.push(modal(picker::view(m, p, sheet_c), sheet_c, Message::CancelPick)),
        (Screen::Settings, _) => {
            let phone = app.phone.running.as_ref().map(|(_, link, qr)| (link.as_str(), qr));
            layers = layers.push(modal(settings::view(m, phone, sheet_c), sheet_c, Message::CloseSettings));
        }
        _ => {}
    }
    if m.confirm_quit {
        layers = layers.push(modal(confirm_quit(m, sheet_c), sheet_c, Message::KeepDownloading));
    }
    if let Some(info) = &m.info {
        // Help, What's new and the bug report go over anything else that is open.
        layers = layers.push(modal(help::sheet(m, info, &app.bug_text, sheet_c), sheet_c, Message::CloseInfo));
    }
    if let Some(step) = m.tour {
        // The first-run tour: its sheet (a click beside it does nothing: Skip is in it), then
        // tooltips under the toolbar buttons.
        match step {
            crate::tour::Step::Coach(mark) => {
                let bubble = float(tour::coach(m, mark, c.faded(a.popover))).translate(move |_, _| Vector::new(0.0, -8.0 * (1.0 - a.popover)));
                layers = layers.push(container(opaque(bubble)).width(Fill).height(Fill).align_x(Alignment::End).padding(tour::coach_padding(mark)));
            }
            _ => layers = layers.push(modal(tour::sheet(m, step, sheet_c), sheet_c, Message::Done)),
        }
    }
    if let Some((_, origin)) = &m.pair_request {
        layers = layers.push(modal(confirm_pair(origin, sheet_c), sheet_c, Message::AnswerPair(false)));
    }
    if m.drop_hover {
        layers = layers.push(drop_target(c));
    }
    if (m.screen == Screen::Downloads || m.update.is_some())
        && let Some(t) = toast(m, c.faded(a.toast))
    {
        // Rises into place from the bottom edge.
        let t = float(t).translate(move |_, _| Vector::new(0.0, 14.0 * (1.0 - a.toast)));
        layers = layers.push(container(opaque(t)).width(Fill).height(Fill).align_x(Alignment::End).align_y(Alignment::End).padding([40, 20]));
    }
    if m.maximized { layers.into() } else { layers.push(resize_grips()).into() }
}

/// Shows `width` of a panel laid out at its full size: from its right edge (`from_left`: the
/// panel slides in from the window's left edge) or from its left edge (slides in from the right).
/// A horizontal scrollable is what lets the panel overflow without being squeezed.
fn slide(panel: Element<'_, Message>, width: f32, from_left: bool) -> Element<'_, Message> {
    let strip = scrollable(panel).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::hidden())).width(width).height(Fill);
    if from_left { strip.anchor_right().into() } else { strip.into() }
}

/// Thin strips along the edges and corners that resize the frameless window.
fn resize_grips<'a>() -> Element<'a, Message> {
    const EDGE: f32 = 5.0;
    let grip = |edge: Direction, width: Length, height: Length, cursor: mouse::Interaction| {
        mouse_area(Space::new().width(width).height(height)).interaction(cursor).on_press(Message::WinResize(edge))
    };
    let e = Length::Fixed(EDGE);
    column![
        row![
            grip(Direction::NorthWest, e, e, mouse::Interaction::ResizingDiagonallyDown),
            grip(Direction::North, Fill, e, mouse::Interaction::ResizingVertically),
            grip(Direction::NorthEast, e, e, mouse::Interaction::ResizingDiagonallyUp),
        ],
        row![
            grip(Direction::West, e, Fill, mouse::Interaction::ResizingHorizontally),
            Space::new().width(Fill).height(Fill),
            grip(Direction::East, e, Fill, mouse::Interaction::ResizingHorizontally),
        ]
        .height(Fill),
        row![
            grip(Direction::SouthWest, e, e, mouse::Interaction::ResizingDiagonallyUp),
            grip(Direction::South, Fill, e, mouse::Interaction::ResizingVertically),
            grip(Direction::SouthEast, e, e, mouse::Interaction::ResizingDiagonallyDown),
        ],
    ]
    .into()
}

/// Files dragged over the window: what dropping them does.
fn drop_target<'a>(c: Colors) -> Element<'a, Message> {
    let badge = container(icon(Icon::TrayDown, 26).color(c.accent)).center(56).style(style::tag(c.accent_soft, c.accent));
    let card = column![
        badge,
        text("Drop to download").size(17).font(style::SEMIBOLD),
        small("A list of links (.txt), a .torrent file or a web shortcut (.url)", c.text2),
    ]
    .spacing(8)
    .align_x(Alignment::Center);
    let card = container(card).padding([22, 30]).style(move |t| {
        let mut s = style::sheet(c)(t);
        s.border = iced::Border { color: c.accent, width: 1.5, radius: 14.0.into() };
        s
    });
    container(card).width(Fill).height(Fill).center(Fill).style(style::scrim(c.scrim, 0.7)).into()
}

/// A browser extension asks to connect (one-click pairing). Its id isn't known in advance, so
/// the question shows who asks and leaves the call to the user.
fn confirm_pair(origin: &str, c: Colors) -> Element<'_, Message> {
    let badge = container(icon(Icon::Browser, 22).color(c.accent)).center(44).style(style::tag(c.accent_soft, c.accent));
    let content = column![
        row![
            badge,
            column![
                text("Connect a browser extension?").size(16).font(style::SEMIBOLD),
                text(format!("{} wants to send downloads here.", crate::view::ellipsize(origin, 72))).size(12.5).font(style::SEMIBOLD),
                text("Allow it only if you just clicked Connect in Snag's extension. If you didn't, choose Don't allow.").size(12.5).color(c.text2),
            ]
            .spacing(4)
            .width(Fill),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
        row![
            Space::new().width(Fill),
            button(text("Don't allow").size(12.5).font(style::SEMIBOLD)).style(style::secondary(c)).padding([8, 14]).on_press(Message::AnswerPair(false)),
            button(text("Allow").size(12.5).font(style::SEMIBOLD)).style(style::primary(c)).padding([8, 18]).on_press(Message::AnswerPair(true)),
        ]
        .spacing(8),
    ]
    .spacing(18);
    container(content).width(440).padding(20).style(style::sheet(c)).into()
}

/// Quit from the tray while something downloads.
fn confirm_quit(m: &Model, c: Colors) -> Element<'_, Message> {
    let n = m.busy_count();
    let (what, them) = if n == 1 { ("1 download is".to_string(), "it") } else { (format!("{n} downloads are"), "them") };
    let badge = container(icon(Icon::WarningCircle, 22).color(c.accent)).center(44).style(style::tag(c.accent_soft, c.accent));
    let content = column![
        row![
            badge,
            column![
                text(format!("{what} still running")).size(16).font(style::SEMIBOLD),
                text(format!("Quitting pauses {them}. Snag continues from where it stopped the next time you open it.")).size(12.5).color(c.text2),
            ]
            .spacing(4)
            .width(Fill),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
        row![
            Space::new().width(Fill),
            button(text("Quit anyway").size(12.5).font(style::SEMIBOLD)).style(style::danger(c)).padding([8, 14]).on_press(Message::QuitAnyway),
            button(text("Keep downloading").size(12.5).font(style::SEMIBOLD)).style(style::primary(c)).padding([8, 14]).on_press(Message::KeepDownloading),
        ]
        .spacing(8),
    ]
    .spacing(18);
    container(content).width(440).padding(20).style(style::sheet(c)).into()
}

/// A sheet over a dimmed window; clicking the dim area sends `on_blur`.
fn modal<'a>(content: Element<'a, Message>, c: Colors, on_blur: Message) -> Element<'a, Message> {
    opaque(mouse_area(container(opaque(content)).width(Fill).height(Fill).center(Fill).style(style::scrim(c.scrim, 1.0))).on_press(on_blur))
}

fn footer(m: &Model, now: Instant, c: Colors) -> Element<'_, Message> {
    let (_, speed) = m.totals();
    // The last minute of the total speed, once anything has downloaded.
    let history = m.speeds.total.values(m.speeds.sec(now));
    let graph: Element<'_, Message> = if history.iter().any(|v| *v > 0) { charts::sparkline(history, 72.0, 14.0, c) } else { Space::new().into() };
    let limit = match m.settings.speed_limit_bps {
        0 => "Limit: none".to_string(),
        bps => format!("Limit: {}", format::speed(bps)),
    };
    let next = view::next_queue_start(&m.queues, rdm_core::Now::local()).map(|t| format!("  ·  Next queue {t}")).unwrap_or_default();
    row![
        small(format!("↓ {}", format::speed(speed)), c.text3),
        graph,
        Space::new().width(Fill),
        small(format!("{limit}{next}"), c.text3),
    ]
    .spacing(10)
    .padding([6, 16])
    .align_y(Alignment::Center)
    .into()
}

/// The bottom-right toast: a clipboard link to download, or the latest notice.
fn toast(m: &Model, c: Colors) -> Option<Element<'_, Message>> {
    let close = |msg: Message| button(icon(Icon::X, 13)).style(style::ghost(c)).padding(6).on_press(msg);
    let card = |content: Element<'static, Message>| container(content).style(style::sheet(c)).padding([10, 12]).max_width(460);
    if m.screen != Screen::Downloads {
        return m.update.as_ref().map(|u| card(update_offer(u, c)).into());
    }
    if let Some(link) = &m.toast {
        let what = view::link_tag(link).map(|t| t.label()).unwrap_or("Link");
        let badge = container(icon(Icon::ClipboardText, 16).color(c.accent)).center(32).style(style::tag(c.accent_soft, c.accent));
        let body = row![
            badge,
            column![
                text("Link detected from clipboard").size(13).font(style::SEMIBOLD),
                small(format!("{} · {what}", view::ellipsize(&view::host(link), 32)), c.text3),
            ]
            .spacing(2)
            .width(Fill),
            button(row![icon(Icon::Download, 13), text("Download").size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
                .style(style::accent(c))
                .padding([7, 12])
                .on_press(Message::ToastDownload),
            close(Message::ToastClose),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        return Some(card(body.into()).into());
    }
    if let Some(item) = &m.duplicate {
        let (day, time) = view::when_label(item.added, chrono::Local::now());
        let badge = container(icon(Icon::CheckCircle, 16).color(c.accent)).center(32).style(style::tag(c.accent_soft, c.accent));
        let choice = |label: &'static str, pick: DuplicateChoice| {
            let b = button(text(label).size(12).font(style::SEMIBOLD)).padding([6, 10]).on_press(Message::Duplicate(pick));
            if pick == DuplicateChoice::Show { b.style(style::accent(c)) } else { b.style(style::ghost(c)) }
        };
        let body = row![
            badge,
            column![
                text("Already downloaded").size(13).font(style::SEMIBOLD),
                small(format!("{day} {time} · {}", view::ellipsize(&item.name, 34)), c.text3),
            ]
            .spacing(2)
            .width(Fill),
            choice("Show file", DuplicateChoice::Show),
            choice("Download again", DuplicateChoice::Again),
            close(Message::Duplicate(DuplicateChoice::Skip)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        return Some(card(body.into()).into());
    }
    // A new version comes before notices, so it isn't buried under them.
    if let Some(u) = &m.update {
        return Some(card(update_offer(u, c)).into());
    }
    let notice = m.notice.clone()?;
    // Text in the window can't be selected: errors get a Copy button (and go to snag.log).
    let copy = button(row![icon(Icon::Copy, 12), text("Copy").size(11.5)].spacing(5).align_y(Alignment::Center))
        .style(style::ghost(c))
        .padding([4, 8])
        .on_press(Message::CopyText(notice.clone()));
    let body = row![text(notice).size(12.5).width(Fill), copy, close(Message::DismissNotice)].spacing(6).align_y(Alignment::Center);
    Some(card(body.into()).into())
}

/// "Snag 1.0.2 is ready": What's new (the release page), Update now, or put it off.
fn update_offer(u: &crate::state::UpdateOffer, c: Colors) -> Element<'static, Message> {
    let badge = container(icon(Icon::Sparkle, 16).color(c.accent)).center(32).style(style::tag(c.accent_soft, c.accent));
    let notes = button(text("What's new").size(12).font(style::SEMIBOLD)).style(style::ghost(c)).padding([6, 10]).on_press(Message::UpdateNotes);
    // A copy Snag can't replace itself (Linux, not the AppImage): the release page has the package.
    let installable = crate::updater::can_install();
    let (label, press) = if installable { ("Update now", Message::UpdateNow) } else { ("Download", Message::UpdateNotes) };
    let install = button(text(if u.busy { "Updating…" } else { label }).size(12).font(style::SEMIBOLD))
        .style(style::accent(c))
        .padding([6, 12])
        .on_press_maybe((!u.busy).then_some(press));
    let later = button(icon(Icon::X, 13)).style(style::ghost(c)).padding(6).on_press_maybe((!u.busy).then_some(Message::UpdateLater));
    let line = if u.busy {
        "Downloading and checking it…".to_string()
    } else if installable {
        "Snag closes, updates and opens again".to_string()
    } else {
        "Get the new package from the release page".to_string()
    };
    row![
        badge,
        column![text(format!("Snag {} is ready", u.release.version)).size(13).font(style::SEMIBOLD), small(line, c.text3)].spacing(2).width(Fill),
        notes,
        install,
        later,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// Small secondary text (Figma: Inter Regular 11.5).
pub fn small<'a>(s: impl text::IntoFragment<'a>, color: Color) -> text::Text<'a> {
    text(s).size(11.5).color(color)
}

/// Counts, subtitles and On/Off (Figma: Inter Regular 11).
pub fn tiny<'a>(s: impl text::IntoFragment<'a>, color: Color) -> text::Text<'a> {
    text(s).size(11).color(color)
}
