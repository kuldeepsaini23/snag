//! The 52 px toolbar: it is also the window's title bar (drag, double-click to maximise).

use super::icon::{Icon, bold};
use super::style;
use super::theme::Colors;
use super::{Anim, tiny};
use crate::format;
use crate::motion;
use crate::state::{Model, SettingsTab};
use crate::tour::Mark;
use crate::update::{Message, search_input};
use crate::view::{FILTERS, Filter};
use iced::widget::{Space, button, column, container, mouse_area, row, sensor, text, text_input};
use iced::{Alignment, Element, Fill, Font};

/// Gap between the filter tabs, and how far in from a tab's sides its underline starts.
const TAB_GAP: f32 = 2.0;
const TAB_INSET: f32 = 10.0;
/// The search field's width once it is open.
const SEARCH_WIDTH: f32 = 190.0;
/// macOS: room for the close, minimise and zoom buttons left of the logo.
const TRAFFIC_LIGHTS: f32 = 76.0;
/// Windows 11's own caption glyphs (thin, sized for 46×32 buttons).
const CAPTION_FONT: Font = Font::with_name("Segoe Fluent Icons");

pub fn view(m: &Model, a: Anim, c: Colors) -> Element<'_, Message> {
    let (running, speed) = m.totals();
    let subtitle = if running == 0 { "Nothing downloading".to_string() } else { format!("{running} active · {}", format::speed(speed)) };
    let logo = iced::widget::image(super::icon::mark(c.accent)).width(26).height(26);
    let title = column![text(if m.stats_open { "Stats" } else { "Downloads" }).size(13).font(style::SEMIBOLD), tiny(subtitle, c.text3)].spacing(1);
    // The filter tabs belong to the list; the stats screen has its own range switch.
    let tabs: Element<'_, Message> = if m.stats_open { Space::new().into() } else { tabs(m, a, c) };

    let middle = container(row![title, Space::new().width(Fill), tabs, Space::new().width(Fill)].spacing(10).align_y(Alignment::Center))
        .width(Fill)
        .clip(true);
    let add = button(row![bold(Icon::Plus, 13), text("Add").size(12.5).font(style::SEMIBOLD)].spacing(5).align_y(Alignment::Center))
        .style(style::pill(c))
        .padding([7, 14])
        .on_press(Message::AddUrl);
    let bar = row![
        logo,
        tool(Icon::Sidebar, false, Message::ToggleSidebar, c),
        middle,
        search(m, a, c),
        tool(Icon::Gauge, m.speed_open || m.coach() == Some(Mark::Speed), Message::ToggleSpeed, c),
        tool(Icon::Gear, false, Message::OpenSettings(SettingsTab::General), c),
        tool(Icon::Question, m.help_open || m.coach() == Some(Mark::Help), Message::ToggleHelp, c),
        add,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // macOS: the system's traffic lights sit at the left end, and its own buttons are the captions.
    let left = if cfg!(target_os = "macos") { TRAFFIC_LIGHTS } else { 12.0 };
    let content = container(bar).height(52).width(Fill).padding(iced::Padding { left, right: 12.0, ..Default::default() }).align_y(Alignment::Center);
    let bar = if cfg!(target_os = "macos") { row![content] } else { row![content, captions(m, c)] };
    mouse_area(bar)
        .on_press(Message::WinDrag)
        .on_double_click(Message::WinMaximize)
        .into()
}

/// An 18 px Bold icon in a 32 px borderless hit area.
fn tool<'a>(i: Icon, on: bool, msg: Message, c: Colors) -> Element<'a, Message> {
    button(container(bold(i, 18)).center(Fill)).width(32).height(32).padding(0).style(style::tool(c, on)).on_press(msg).into()
}

/// Text tabs with a muted count; an accent line slides under the active one.
fn tabs(m: &Model, a: Anim, c: Colors) -> Element<'_, Message> {
    let counts = m.counts();
    let labels = FILTERS.iter().enumerate().fold(row![].spacing(TAB_GAP), |r, (k, &f)| {
        let name = match f {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Done => "Done",
            Filter::Scheduled => "Scheduled",
        };
        let on = m.filter == f;
        let label = row![text(name).size(13).font(if on { style::MEDIUM } else { style::INTER }), tiny(counts.of(f).to_string(), c.text3)]
            .spacing(5)
            .align_y(Alignment::Center);
        let tab = button(label).style(style::tab(c, on)).padding([6, TAB_INSET as u16]).on_press(Message::SetFilter(f));
        // The underline needs each tab's real width (fonts, counts): measured as drawn.
        r.push(sensor(tab).on_resize(move |size| Message::TabMeasured(k, size.width)))
    });
    let targets = motion::tab_indicator_targets(&m.tab_widths, TAB_GAP, TAB_INSET);
    let (x, w) = motion::indicator_at(&targets, a.tab);
    let line = row![Space::new().width(x), container(Space::new()).width(w).height(2).style(style::indicator(c))];
    column![labels, line].spacing(2).into()
}

/// A magnifier that opens into the search field (click or Ctrl+K); empty, it folds back.
fn search(m: &Model, a: Anim, c: Colors) -> Element<'_, Message> {
    if !m.search_open && a.search == 0.0 {
        return tool(Icon::MagnifyingGlass, false, Message::FocusSearch, c);
    }
    let field = text_input("Search downloads", &m.search)
        .id(search_input())
        .on_input(Message::Search)
        .icon(text_input::Icon { font: super::icon::BOLD_FONT, code_point: Icon::MagnifyingGlass.ch(), size: Some(14.0.into()), spacing: 8.0, side: text_input::Side::Left })
        .size(12.5)
        .padding([7, 10])
        .style(style::search(c));
    // Grows from the magnifier's 32 px to the full field.
    container(field).width(32.0 + (SEARCH_WIDTH - 32.0) * a.search).clip(true).into()
}

/// Minimise, maximise/restore and close, flush with the window's top-right corner.
fn captions(m: &Model, c: Colors) -> Element<'_, Message> {
    // Windows 11's own glyphs; Linux has no Segoe, so the same three come from Phosphor.
    let glyph = |code: char, other: Icon| {
        if cfg!(windows) { text(code.to_string()).font(CAPTION_FONT).size(10).line_height(1.0) } else { super::icon::icon(other, 13).line_height(1.0) }
    };
    let win = |code: char, other: Icon, msg: Message| button(container(glyph(code, other)).center(Fill)).width(46).height(32).padding(0).on_press(msg);
    let (max, max_other) = if m.maximized { ('\u{E923}', Icon::Copy) } else { ('\u{E922}', Icon::Square) };
    let buttons = row![
        win('\u{E921}', Icon::Minus, Message::WinMinimize).style(style::caption(c)),
        win(max, max_other, Message::WinMaximize).style(style::caption(c)),
        win('\u{E8BB}', Icon::X, Message::WinClose).style(style::close_button(c)),
    ];
    column![buttons].height(52).into()
}
