//! The Figma views. Logic lives in `state.rs` / `view.rs` / `update.rs`; this module only draws.

pub mod icon;
mod inspector;
mod list;
mod picker;
mod popover;
mod settings;
mod sidebar;
pub mod style;
pub mod theme;
mod toolbar;

use crate::format;
use crate::state::{Model, Screen};
use crate::update::{App, Message};
use crate::view;
use icon::{Icon, icon};
use iced::widget::{Space, button, column, container, mouse_area, opaque, row, stack, text};
use iced::window::Direction;
use iced::{Alignment, Color, Element, Fill, Length, Padding, mouse};
use theme::Colors;

pub fn view(app: &App) -> Element<'_, Message> {
    let m = &app.model;
    let c = theme::colors(m.accent_hex());

    let mut body = row![].spacing(8).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 }).height(Fill);
    if m.sidebar_open {
        body = body.push(sidebar::view(m, c));
    }
    body = body.push(list::view(m, c));
    if let Some(item) = m.inspected() {
        body = body.push(inspector::view(m, item, c));
    }
    let base = container(column![toolbar::view(m, c), body, footer(m, c)]).style(style::window(c)).width(Fill).height(Fill);

    let mut layers = stack![base].width(Fill).height(Fill);
    if m.speed_open {
        // A click anywhere else closes the popover.
        layers = layers.push(mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::ToggleSpeed));
        layers = layers.push(
            container(opaque(popover::view(m, c)))
                .width(Fill)
                .height(Fill)
                .align_x(Alignment::End)
                .padding(Padding { top: 46.0, right: 214.0, bottom: 0.0, left: 0.0 }),
        );
    }
    match (m.screen, &m.picker) {
        (Screen::Picker, Some(p)) => layers = layers.push(modal(picker::view(m, p, c), Message::CancelPick)),
        (Screen::Settings, _) => layers = layers.push(modal(settings::view(m, app.phone.as_ref().map(|(_, link, qr)| (link.as_str(), qr)), c), Message::CloseSettings)),
        _ => {}
    }
    if m.confirm_quit {
        layers = layers.push(modal(confirm_quit(m, c), Message::KeepDownloading));
    }
    if m.screen == Screen::Downloads
        && let Some(t) = toast(m, c)
    {
        layers = layers.push(container(opaque(t)).width(Fill).height(Fill).align_x(Alignment::End).align_y(Alignment::End).padding([40, 20]));
    }
    if m.maximized { layers.into() } else { layers.push(resize_grips()).into() }
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
fn modal<'a>(content: Element<'a, Message>, on_blur: Message) -> Element<'a, Message> {
    opaque(mouse_area(container(opaque(content)).width(Fill).height(Fill).center(Fill).style(style::scrim)).on_press(on_blur))
}

fn footer(m: &Model, c: Colors) -> Element<'_, Message> {
    let (_, speed) = m.totals();
    let limit = match m.settings.speed_limit_bps {
        0 => "Limit: none".to_string(),
        bps => format!("Limit: {}", format::speed(bps)),
    };
    let next = view::next_queue_start(&m.queues, rdm_core::Now::local()).map(|t| format!("  ·  Next queue {t}")).unwrap_or_default();
    row![
        small(format!("↓ {}", format::speed(speed)), c.text3),
        Space::new().width(Fill),
        small(format!("{limit}{next}"), c.text3),
    ]
    .padding([6, 16])
    .align_y(Alignment::Center)
    .into()
}

/// The bottom-right toast: a clipboard link to download, or the latest notice.
fn toast(m: &Model, c: Colors) -> Option<Element<'_, Message>> {
    let close = |msg: Message| button(icon(Icon::X, 13)).style(style::ghost(c)).padding(6).on_press(msg);
    let card = |content: Element<'static, Message>| container(content).style(style::sheet(c)).padding([10, 12]).max_width(460);
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
    let notice = m.notice.clone()?;
    let body = row![text(notice).size(12.5).width(Fill), close(Message::DismissNotice)].spacing(10).align_y(Alignment::Center);
    Some(card(body.into()).into())
}

/// Small secondary text (Figma: Inter Regular 11.5).
pub fn small<'a>(s: impl text::IntoFragment<'a>, color: Color) -> text::Text<'a> {
    text(s).size(11.5).color(color)
}

/// Counts, subtitles and On/Off (Figma: Inter Regular 11).
pub fn tiny<'a>(s: impl text::IntoFragment<'a>, color: Color) -> text::Text<'a> {
    text(s).size(11).color(color)
}
