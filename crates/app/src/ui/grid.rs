//! Videos and Images as a grid of thumbnail cards: the picture with its title, size and length
//! over it, and a ring that shows how far the download got.

use super::charts;
use super::icon::{Icon, bold};
use super::list::{badge, picture};
use super::style;
use super::theme::Colors;
use crate::state::Model;
use crate::update::Message;
use crate::view;
use iced::border::Radius;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, mouse_area, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Fill, Gradient, Theme, gradient};
use rdm_core::{Category, Item, Status};

const CARD_W: f32 = 212.0;
const CARD_H: f32 = 120.0;
const RING: f32 = 22.0;

/// The list ⇄ grid switch next to the hint line.
pub fn toggle<'a>(library: Category, grid: bool, c: Colors) -> Element<'a, Message> {
    let choice = |glyph: Icon, on: bool| button(container(bold(glyph, 14)).center(Fill)).width(28).height(24).padding(0).style(style::choice(c, on, 6.0)).on_press(Message::SetGrid(library, !grid));
    let both = row![choice(Icon::Rows, !grid), choice(Icon::SquaresFour, grid)].spacing(2);
    container(both).padding(2).style(style::segmented(c)).into()
}

/// The cards, wrapping onto as many lines as the panel needs.
pub fn cards<'a>(m: &'a Model, items: Vec<&'a Item>, c: Colors) -> Element<'a, Message> {
    let cards = items.into_iter().fold(row![].spacing(12), |r, i| r.push(card(m, i, c)));
    container(cards.wrap().vertical_spacing(12)).padding([4, 2]).into()
}

fn card<'a>(m: &'a Model, i: &'a Item, c: Colors) -> Element<'a, Message> {
    let thumb = view::thumb_url(i).and_then(|u| m.thumbs.get(&u)).map(|p| p.as_path());
    let white = |a: f32| c.fixed(Color { a, ..Color::WHITE });
    let length: Element<'a, Message> = match i.duration {
        Some(secs) => badge(view::duration_label(secs)),
        None => Space::new().into(),
    };
    let top = row![length, Space::new().width(Fill), status_ring(i, c)].align_y(Alignment::Start).padding(7);
    let meta = view::ellipsize(&view::card_meta(i), 34);
    let caption = column![
        text(view::ellipsize(&i.name, 30)).size(12.5).font(style::MEDIUM).color(white(1.0)).wrapping(Wrapping::None),
        text(meta).size(11).color(if matches!(i.status, Status::Failed(_)) { c.danger } else { white(0.75) }).wrapping(Wrapping::None),
    ]
    .spacing(1);
    let bottom = container(caption).width(CARD_W).padding(iced::Padding { top: 22.0, right: 10.0, bottom: 8.0, left: 10.0 }).clip(true).style(scrim(c));
    let overlay = column![top, Space::new().height(Fill), bottom].width(CARD_W).height(CARD_H);
    let body = stack![picture(i, thumb, CARD_W, CARD_H, 30, c), overlay];

    let selected = m.selected == Some(i.id);
    let hovered = m.hovered == Some(i.id);
    let edge = if selected { c.accent } else if hovered { c.line_strong } else { Color::TRANSPARENT };
    let framed = container(body).padding(2).style(move |_: &Theme| container::Style { border: Border { color: edge, width: 2.0, radius: Radius::from(9.0) }, ..Default::default() });
    let card = button(framed).padding(0).style(|_, _| button::Style::default()).on_press(Message::Select(i.id));
    mouse_area(card).on_enter(Message::HoverRow(i.id, true)).on_exit(Message::HoverRow(i.id, false)).into()
}

/// The picture's bottom darkened, so the white title reads on any thumbnail.
fn scrim(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let fade = gradient::Linear::new(std::f32::consts::PI)
            .add_stop(0.0, c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.0)))
            .add_stop(0.45, c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.6)))
            .add_stop(1.0, c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.85)));
        container::Style {
            background: Some(Background::Gradient(Gradient::Linear(fade))),
            border: Border { radius: Radius { top_left: 0.0, top_right: 0.0, bottom_right: 7.0, bottom_left: 7.0 }, ..Default::default() },
            ..Default::default()
        }
    }
}

/// How far it got, on a dark disc: the accent while downloading, grey while waiting, red when it
/// failed, green with a check once done.
fn status_ring<'a>(i: &Item, c: Colors) -> Element<'a, Message> {
    let progress = match i.total {
        Some(t) if t > 0 => (i.downloaded as f32 / t as f32).min(1.0),
        _ => 0.0,
    };
    let white = c.fixed(Color::WHITE);
    let (done, color, glyph) = match &i.status {
        Status::Done => (1.0, c.success, Some(Icon::Check)),
        Status::Running => (progress, c.accent, None),
        Status::Paused => (progress, white, Some(Icon::Pause)),
        Status::Queued => (progress, white, Some(Icon::Clock)),
        Status::Failed(_) => (1.0, c.danger, Some(Icon::X)),
    };
    let track = c.fixed(Color::from_rgba(1.0, 1.0, 1.0, 0.22));
    let mut ring = stack![charts::ring(done, color, track, RING)];
    if let Some(g) = glyph {
        ring = ring.push(container(bold(g, 10).color(if i.status == Status::Done { c.success } else { white })).center(RING));
    }
    container(ring).padding(2).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.55)))),
        border: Border { radius: Radius::from(RING), ..Default::default() },
        ..Default::default()
    })
    .into()
}
