//! The 52 px toolbar: it is also the window's title bar (drag, double-click to maximise).

use super::icon::{Icon, PHOSPHOR_FONT, icon};
use super::style;
use super::theme::Colors;
use super::tiny;
use crate::format;
use crate::state::{Model, SettingsTab};
use crate::update::{Message, search_input};
use crate::view::{FILTERS, Filter};
use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input};
use iced::{Alignment, Element, Fill};

pub fn view(m: &Model, c: Colors) -> Element<'_, Message> {
    let (running, speed) = m.totals();
    let subtitle = if running == 0 { "Nothing downloading".to_string() } else { format!("{running} active · {}", format::speed(speed)) };
    let logo = container(icon(Icon::Download, 15).color(c.on_accent)).center(28).style(style::tag(c.accent, c.on_accent));
    let square = |i: Icon, on: bool, msg: Message| button(container(icon(i, 15)).center(Fill)).width(30).height(28).padding(0).style(style::icon_button(c, on)).on_press(msg);
    let title = column![text("Downloads").size(13).font(style::SEMIBOLD), tiny(subtitle, c.text3)].spacing(1);

    let counts = m.counts();
    let pills = FILTERS.iter().fold(row![].spacing(2), |r, &f| {
        let (glyph, name) = match f {
            Filter::All => (Icon::TrayDown, "All"),
            Filter::Active => (Icon::Lightning, "Active"),
            Filter::Done => (Icon::CheckCircle, "Done"),
            Filter::Scheduled => (Icon::Clock, "Scheduled"),
        };
        let on = m.filter == f;
        let label = row![
            icon(glyph, 13),
            text(name).size(12.5).font(if on { style::MEDIUM } else { style::INTER }),
            tiny(counts.of(f).to_string(), c.text3),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        r.push(button(label).style(style::choice(c, on, 6.0)).padding([5, 10]).on_press(Message::SetFilter(f)))
    });
    let pills = container(pills).padding(3).style(style::segmented(c));

    let search = text_input("Search", &m.search)
        .id(search_input())
        .on_input(Message::Search)
        .icon(text_input::Icon { font: PHOSPHOR_FONT, code_point: Icon::MagnifyingGlass.ch(), size: Some(13.0.into()), spacing: 7.0, side: text_input::Side::Left })
        .size(12)
        .padding([6, 10])
        .width(150)
        .style(style::input(c));

    let add = button(row![icon(Icon::Plus, 13), text("Add URL").size(12.5).font(style::SEMIBOLD)].spacing(6).align_y(Alignment::Center))
        .style(style::primary(c))
        .padding([7, 13])
        .on_press(Message::AddUrl);

    let win = |i: Icon, msg: Message, close: bool| {
        let b = button(container(icon(i, 14)).center(Fill)).width(44).height(32).padding(0).on_press(msg);
        if close { b.style(style::close_button(c)) } else { b.style(style::ghost(c)) }
    };

    let bar = row![
        logo,
        square(Icon::Sidebar, false, Message::ToggleSidebar),
        title,
        Space::new().width(Fill),
        pills,
        Space::new().width(Fill),
        search,
        square(Icon::Gauge, m.speed_open, Message::ToggleSpeed),
        square(Icon::Gear, false, Message::OpenSettings(SettingsTab::General)),
        add,
        row![win(Icon::Minus, Message::WinMinimize, false), win(Icon::Square, Message::WinMaximize, false), win(Icon::X, Message::WinClose, true)].spacing(2),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    mouse_area(container(bar).height(52).padding([0, 12]).align_y(Alignment::Center))
        .on_press(Message::WinDrag)
        .on_double_click(Message::WinMaximize)
        .into()
}
