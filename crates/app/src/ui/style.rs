//! Widget styles from the Figma components (Button, Toggle, Checkbox, Radio, Text Field, rows, panels).

use super::theme::Colors;
use iced::border::Radius;
use iced::font::Weight;
use iced::widget::{button, checkbox, container, pick_list, progress_bar, scrollable, slider, text_input, toggler};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};

pub const INTER: Font = Font::with_name("Inter");
pub const MEDIUM: Font = Font { weight: Weight::Medium, ..INTER };
pub const SEMIBOLD: Font = Font { weight: Weight::Semibold, ..INTER };
/// Numbers (speed, time left, sizes), as in the Figma file.
pub const MONO: Font = Font::with_name("JetBrains Mono");

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border { color, width, radius: Radius::from(radius) }
}

fn bg(c: Color) -> Option<Background> {
    Some(Background::Color(c))
}

/// Mixes `top` (with its alpha) over `base`.
pub fn over(base: Color, top: Color) -> Color {
    let a = top.a;
    Color::from_rgb(base.r + (top.r - base.r) * a, base.g + (top.g - base.g) * a, base.b + (top.b - base.b) * a)
}

// ---------- containers ----------

pub fn panel(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(c.panel), border: border(c.line, 1.0, 12.0), text_color: Some(c.text), ..Default::default() }
}

pub fn window(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(c.canvas), text_color: Some(c.text), ..Default::default() }
}

/// Grouped settings rows, the inspector table, the picker's title card.
pub fn card(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(c.surface), border: border(c.line, 1.0, 10.0), ..Default::default() }
}

pub fn sheet(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: bg(c.panel),
        border: border(c.line_strong, 1.0, 14.0),
        shadow: Shadow { color: c.fixed(Color::from_rgba(0.0, 0.0, 0.0, 0.55)), offset: Vector::new(0.0, 18.0), blur_radius: 48.0 },
        text_color: Some(c.text),
        ..Default::default()
    }
}

/// The dim behind a sheet (`a`: 0 … 1 as it fades in).
pub fn scrim(a: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(Color::from_rgba(0.0, 0.0, 0.0, 0.5 * a)), ..Default::default() }
}

/// A small coloured tag ("Video detected", "4K").
pub fn tag(fill: Color, text: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(fill), border: border(Color::TRANSPARENT, 0.0, 4.0), text_color: Some(text), ..Default::default() }
}

/// The square tile in front of a row (thumbnail stand-in).
pub fn tile(top: Color, bottom: Color, line: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(std::f32::consts::FRAC_PI_4 * 3.0).add_stop(0.0, top).add_stop(1.0, bottom),
        ))),
        border: border(line, 1.0, 7.0),
        ..Default::default()
    }
}

pub fn error_card(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: bg(over(c.panel, Color { a: 0.12, ..c.danger })),
        border: border(Color { a: 0.35, ..c.danger }, 1.0, 10.0),
        text_color: Some(c.text),
        ..Default::default()
    }
}

/// The URL bar: accent outline when it holds a link.
pub fn url_bar(c: Colors, active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: bg(if active { over(c.surface, Color { a: 0.06, ..c.accent }) } else { c.surface }),
        border: border(if active { c.accent } else { c.line_strong }, if active { 1.5 } else { 1.0 }, 10.0),
        ..Default::default()
    }
}

pub fn segmented(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(c.canvas), border: border(c.line, 1.0, 8.0), ..Default::default() }
}

// ---------- buttons ----------

fn button_style(background: Option<Color>, text: Color, line: Color, radius: f32) -> button::Style {
    button::Style { background: background.map(Background::Color), text_color: text, border: border(line, 1.0, radius), ..Default::default() }
}

/// White button (Add URL, Show in folder, Download now).
pub fn primary(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = match s {
            button::Status::Hovered => c.fixed(Color::from_rgb8(0xe8, 0xe6, 0xe3)),
            button::Status::Disabled => c.raised,
            _ => c.fixed(Color::WHITE),
        };
        let text = if s == button::Status::Disabled { c.text3 } else { c.canvas };
        button_style(Some(fill), text, Color::TRANSPARENT, 8.0)
    }
}

pub fn accent(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = match s {
            button::Status::Hovered => over(c.accent, Color { a: 0.12, ..Color::WHITE }),
            button::Status::Disabled => c.raised,
            _ => c.accent,
        };
        button_style(Some(fill), if s == button::Status::Disabled { c.text3 } else { c.on_accent }, Color::TRANSPARENT, 8.0)
    }
}

/// Raised grey button (Pause, Add to queue, Copy).
pub fn secondary(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = if s == button::Status::Hovered { c.hover } else { c.raised };
        let text = if s == button::Status::Disabled { c.text3 } else { c.text };
        button_style(Some(fill), text, c.line, 8.0)
    }
}

/// Outlined button (Retry, Regenerate, Change…).
pub fn outline(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = (s == button::Status::Hovered).then_some(c.hover);
        button_style(fill, if s == button::Status::Disabled { c.text3 } else { c.text }, c.line_strong, 8.0)
    }
}

/// Text-only button that lights up on hover (row actions, window buttons).
pub fn ghost(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = matches!(s, button::Status::Hovered | button::Status::Pressed).then_some(c.hover);
        button_style(fill, if s == button::Status::Disabled { c.text3 } else { c.text2 }, Color::TRANSPARENT, 7.0)
    }
}

/// Borderless toolbar icon with a soft hover; `on` = its popover is open (accent).
pub fn tool(c: Colors, on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let hot = matches!(s, button::Status::Hovered | button::Status::Pressed);
        let fill = if on { Some(c.accent_soft) } else { hot.then_some(Color { a: 0.07, ..Color::WHITE }) };
        let text = if on { c.accent } else if hot { c.text } else { c.text2 };
        button_style(fill, text, Color::TRANSPARENT, 8.0)
    }
}

/// A filter tab: text only, brighter when active or hovered (the underline marks the active one).
pub fn tab(c: Colors, on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let text = if on || matches!(s, button::Status::Hovered | button::Status::Pressed) { c.text } else { c.text2 };
        button_style(None, text, Color::TRANSPARENT, 6.0)
    }
}

/// The toolbar's "+ Add": one accent pill.
pub fn pill(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |t, s| {
        let mut b = accent(c)(t, s);
        b.border.radius = 16.0.into();
        b
    }
}

/// Minimise / maximise: Windows 11 caption buttons (square, a faint hover).
pub fn caption(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = match s {
            button::Status::Hovered => Some(Color { a: 0.06, ..Color::WHITE }),
            button::Status::Pressed => Some(Color { a: 0.04, ..Color::WHITE }),
            _ => None,
        };
        button_style(fill, if fill.is_some() { c.text } else { c.text2 }, Color::TRANSPARENT, 0.0)
    }
}

pub fn close_button(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = match s {
            button::Status::Hovered => Some(Color::from_rgb8(0xc4, 0x2b, 0x1c)),
            button::Status::Pressed => Some(Color::from_rgb8(0xb2, 0x27, 0x1a)),
            _ => None,
        };
        button_style(fill, if fill.is_some() { Color::WHITE } else { c.text2 }, Color::TRANSPARENT, 0.0)
    }
}

/// The accent line under the active filter tab.
pub fn indicator(c: Colors) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: bg(c.accent), border: border(Color::TRANSPARENT, 0.0, 1.5), ..Default::default() }
}

pub fn danger(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = if s == button::Status::Hovered { Color { a: 0.25, ..c.danger } } else { Color { a: 0.14, ..c.danger } };
        button_style(Some(fill), c.danger, Color { a: 0.35, ..c.danger }, 8.0)
    }
}

/// A filter pill / settings nav item / sidebar row / segmented choice.
pub fn choice(c: Colors, selected: bool, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = if selected { Some(c.raised) } else { (s == button::Status::Hovered).then_some(Color { a: 0.05, ..Color::WHITE }) };
        button_style(fill, if selected { c.text } else { c.text2 }, Color::TRANSPARENT, radius)
    }
}

/// A download row; selected rows get the raised surface and a hairline. `glow` (1 → 0) tints it
/// with the accent just after it finished.
pub fn row(c: Colors, selected: bool, failed: bool, glow: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let mut fill = if selected { Some(c.surface) } else { (s == button::Status::Hovered).then_some(Color { a: 0.03, ..Color::WHITE }) };
        let mut line = match (selected, failed) {
            (true, true) => Color { a: 0.45, ..c.danger },
            (true, false) => c.line_strong,
            _ => Color::TRANSPARENT,
        };
        if glow > 0.0 {
            fill = Some(over(fill.map_or(c.panel, |f| over(c.panel, f)), Color { a: 0.2 * glow, ..c.accent }));
            line = Color { a: 0.7 * glow, ..c.accent };
        }
        button_style(fill, c.text, line, 10.0)
    }
}

/// A quality option in the picker; the chosen one is accent-soft with an accent outline.
pub fn option(c: Colors, chosen: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, s| {
        let fill = if chosen { Some(Color { a: 0.16, ..c.accent }) } else { (s == button::Status::Hovered).then_some(Color { a: 0.04, ..Color::WHITE }) };
        button_style(fill, c.text, if chosen { c.accent } else { Color::TRANSPARENT }, 8.0)
    }
}

pub fn swatch(color: Color, chosen: bool, ring: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: bg(color),
        border: Border { color: if chosen { ring } else { Color::TRANSPARENT }, width: if chosen { 2.0 } else { 0.0 }, radius: Radius::from(10.0) },
        ..Default::default()
    }
}

// ---------- inputs ----------

pub fn input(c: Colors) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, s| text_input::Style {
        background: Background::Color(c.canvas),
        border: border(if matches!(s, text_input::Status::Focused { .. }) { c.accent } else { c.line_strong }, 1.0, 7.0),
        icon: c.text3,
        placeholder: c.text3,
        value: c.text,
        selection: Color { a: 0.35, ..c.accent },
    }
}

/// The toolbar search: a soft filled field, accent edge while typing.
pub fn search(c: Colors) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, s| {
        let focused = matches!(s, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(Color { a: 0.06, ..Color::WHITE }),
            border: border(if focused { Color { a: 0.7, ..c.accent } } else { Color::TRANSPARENT }, 1.0, 8.0),
            icon: c.text2,
            placeholder: c.text3,
            value: c.text,
            selection: Color { a: 0.35, ..c.accent },
        }
    }
}

/// An input with no box of its own (inside the URL bar).
pub fn bare_input(c: Colors) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, _| text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: border(Color::TRANSPARENT, 0.0, 0.0),
        icon: c.text3,
        placeholder: c.text3,
        value: c.text,
        selection: Color { a: 0.35, ..c.accent },
    }
}

pub fn toggle(c: Colors) -> impl Fn(&Theme, toggler::Status) -> toggler::Style {
    move |_, s| {
        let on = matches!(s, toggler::Status::Active { is_toggled: true } | toggler::Status::Hovered { is_toggled: true });
        toggler::Style {
            background: Background::Color(if on { c.accent } else { c.hover }),
            background_border_width: 0.0,
            background_border_color: Color::TRANSPARENT,
            foreground: Background::Color(if on { c.knob } else { Color::from_rgb8(0xb8, 0xb4, 0xaf) }),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: Some(c.text),
            border_radius: None,
            padding_ratio: 0.12,
        }
    }
}

pub fn check(c: Colors) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style {
    move |_, s| {
        let on = matches!(s, checkbox::Status::Active { is_checked: true } | checkbox::Status::Hovered { is_checked: true });
        checkbox::Style {
            background: Background::Color(if on { c.accent } else { Color::TRANSPARENT }),
            icon_color: c.on_accent,
            border: border(if on { c.accent } else { c.line_strong }, 1.5, 4.0),
            text_color: Some(c.text),
        }
    }
}

pub fn progress(track: Color, fill: Color) -> impl Fn(&Theme) -> progress_bar::Style {
    move |_| progress_bar::Style { background: Background::Color(track), bar: Background::Color(fill), border: border(Color::TRANSPARENT, 0.0, 2.0) }
}

pub fn speed_slider(c: Colors) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(c.accent), Background::Color(c.hover)), width: 4.0, border: border(Color::TRANSPARENT, 0.0, 2.0) },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 7.0 }, background: Background::Color(Color::WHITE), border_width: 0.0, border_color: Color::TRANSPARENT },
    }
}

pub fn pick(c: Colors) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_, s| pick_list::Style {
        text_color: c.text,
        placeholder_color: c.text3,
        handle_color: c.text2,
        background: Background::Color(if s == pick_list::Status::Hovered { c.surface } else { c.canvas }),
        border: border(c.line_strong, 1.0, 7.0),
    }
}

pub fn menu(c: Colors) -> impl Fn(&Theme) -> iced::overlay::menu::Style {
    move |_| iced::overlay::menu::Style {
        background: Background::Color(c.raised),
        border: border(c.line_strong, 1.0, 8.0),
        text_color: c.text,
        selected_text_color: c.on_accent,
        selected_background: Background::Color(c.accent),
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.4), offset: Vector::new(0.0, 8.0), blur_radius: 24.0 },
    }
}

pub fn scroll(c: Colors) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |theme, s| {
        let mut style = scrollable::default(theme, s);
        let rail = scrollable::Rail {
            background: None,
            border: border(Color::TRANSPARENT, 0.0, 3.0),
            scroller: scrollable::Scroller { background: Background::Color(c.hover), border: border(Color::TRANSPARENT, 0.0, 3.0) },
        };
        style.vertical_rail = rail;
        style.horizontal_rail = rail;
        style
    }
}
