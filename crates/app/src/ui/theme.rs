//! Colour tokens from the Figma file (spec §2.5) and the accent the user picks.

use iced::Color;
use iced::theme::Palette;

pub const DEFAULT_ACCENT: &str = "#ff9f0a";

/// The accent swatches offered in Settings → Appearance (name, hex).
pub const SWATCHES: [(&str, &str); 7] = [
    ("Orange", "#ff9f0a"),
    ("White", "#f5f5f7"),
    ("Green", "#32d74b"),
    ("Mint", "#63e6e2"),
    ("Blue", "#0a84ff"),
    ("Purple", "#bf5af2"),
    ("Pink", "#ff375f"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub canvas: Color,
    pub panel: Color,
    pub sidebar: Color,
    pub surface: Color,
    pub raised: Color,
    pub hover: Color,
    pub line: Color,
    pub line_strong: Color,
    pub text: Color,
    pub text2: Color,
    pub text3: Color,
    pub success: Color,
    pub danger: Color,
    pub accent: Color,
    /// The accent at 20% opacity: selected rows, tags.
    pub accent_soft: Color,
    /// Text drawn on top of the accent.
    pub on_accent: Color,
    /// A switch's knob when it's on: white, unless the accent itself is near-white.
    pub knob: Color,
}

/// `#rgb` or `#rrggbb` (the `#` is optional, any case). Anything else is `None`.
pub fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.trim().trim_start_matches('#');
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize, len: usize| u8::from_str_radix(&hex[i * len..i * len + len], 16).ok();
    let (r, g, b) = match hex.len() {
        3 => (channel(0, 1)? * 17, channel(1, 1)? * 17, channel(2, 1)? * 17),
        6 => (channel(0, 2)?, channel(1, 2)?, channel(2, 2)?),
        _ => return None,
    };
    Some(Color::from_rgb8(r, g, b))
}

/// Dark text on light accents, white on dark ones (perceived brightness above 0.6 counts as light).
pub fn on_accent(accent: Color) -> Color {
    let brightness = 0.299 * accent.r + 0.587 * accent.g + 0.114 * accent.b;
    if brightness > 0.6 { Color::from_rgb8(0x1a, 0x18, 0x16) } else { Color::WHITE }
}

/// The full token set for an accent; an invalid accent falls back to the default orange.
pub fn colors(accent_hex: &str) -> Colors {
    let accent = parse_hex(accent_hex).or_else(|| parse_hex(DEFAULT_ACCENT)).unwrap_or(Color::WHITE);
    let white = |a: f32| Color { a, ..Color::WHITE };
    Colors {
        canvas: Color::from_rgb8(0x1a, 0x18, 0x16),
        panel: Color::from_rgb8(0x21, 0x1f, 0x1c),
        sidebar: Color::from_rgb8(0x26, 0x22, 0x1f),
        surface: Color::from_rgb8(0x2a, 0x27, 0x24),
        raised: Color::from_rgb8(0x36, 0x32, 0x2e),
        hover: Color::from_rgb8(0x43, 0x3e, 0x39),
        line: white(0x17 as f32 / 255.0),
        line_strong: white(0x26 as f32 / 255.0),
        text: white(0xe5 as f32 / 255.0),
        text2: white(0x91 as f32 / 255.0),
        text3: white(0x4f as f32 / 255.0),
        success: Color::from_rgb8(0x32, 0xd7, 0x4b),
        danger: Color::from_rgb8(0xff, 0x45, 0x3a),
        accent,
        accent_soft: Color { a: 0.2, ..accent },
        on_accent: on_accent(accent),
        knob: if accent.r.min(accent.g).min(accent.b) > 0.85 { Color::from_rgb8(0x1a, 0x18, 0x16) } else { Color::WHITE },
    }
}

pub fn theme(c: &Colors) -> iced::Theme {
    iced::Theme::custom(
        "RDM",
        Palette { background: c.canvas, text: c.text, primary: c.accent, success: c.success, warning: c.accent, danger: c.danger },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_cases() {
        assert_eq!(parse_hex("#ff9f0a"), Some(Color::from_rgb8(0xff, 0x9f, 0x0a)));
        assert_eq!(parse_hex("FF9F0A"), Some(Color::from_rgb8(0xff, 0x9f, 0x0a)));
        assert_eq!(parse_hex(" #fff "), Some(Color::WHITE));
        for bad in ["", "#", "#ff", "orange", "#gggggg", "#ff9f0a0", "#ff 9f0"] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn text_on_accent_by_luminance() {
        let dark = Color::from_rgb8(0x1a, 0x18, 0x16);
        assert_eq!(on_accent(parse_hex("#f5f5f7").unwrap()), dark);
        assert_eq!(on_accent(parse_hex("#ff9f0a").unwrap()), dark);
        assert_eq!(on_accent(parse_hex("#0a84ff").unwrap()), Color::WHITE);
        assert_eq!(on_accent(parse_hex("#bf5af2").unwrap()), Color::WHITE);
    }

    #[test]
    fn colors_fall_back_on_bad_accent() {
        assert_eq!(colors("nope"), colors(DEFAULT_ACCENT));
        assert_eq!(colors("#0a84ff").accent, Color::from_rgb8(0x0a, 0x84, 0xff));
        assert_eq!(colors("#0a84ff").accent_soft.a, 0.2);
    }

    #[test]
    fn knob_contrasts_with_light_accent() {
        assert_eq!(colors("#ff9f0a").knob, Color::WHITE);
        assert_eq!(colors("#0a84ff").knob, Color::WHITE);
        assert_ne!(colors("#f5f5f7").knob, Color::WHITE, "a white knob on a white track is invisible");
    }

    #[test]
    fn swatches_parse() {
        for (name, hex) in SWATCHES {
            assert!(parse_hex(hex).is_some(), "{name}");
        }
        assert_eq!(SWATCHES[0].1, DEFAULT_ACCENT);
    }
}
