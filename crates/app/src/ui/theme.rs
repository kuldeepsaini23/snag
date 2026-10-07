//! Colour tokens from the Figma file (spec §2.5) and the accent the user picks.

use iced::Color;
use iced::theme::Palette;

pub const DEFAULT_ACCENT: &str = "#ff9f0a";

/// Everything is drawn this much larger than the Figma sizes (the user picked it for readability).
pub const UI_SCALE: f32 = 1.08;

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
    /// A switch's knob and track when it's off.
    pub knob_off: Color,
    pub track: Color,
    /// The colour of hover washes and the primary button: white on dark, near-black on light.
    pub ink: Color,
    /// Text on `ink`.
    pub on_ink: Color,
    /// The glyph on a video, song or picture tile.
    pub tile_ink: Color,
    /// Under sheets and popovers.
    pub shadow: Color,
    /// The dim behind a sheet, fully faded in.
    pub scrim: Color,
    /// The light set (Settings → Appearance → Theme).
    pub light: bool,
    /// How opaque everything drawn with these colours is (below 1 while a sheet fades in).
    pub alpha: f32,
}

impl Colors {
    /// Every token at `a` of its opacity: a sheet, popover or new row fading in.
    pub fn faded(self, a: f32) -> Colors {
        let f = |c: Color| Color { a: c.a * a, ..c };
        Colors {
            canvas: f(self.canvas),
            panel: f(self.panel),
            sidebar: f(self.sidebar),
            surface: f(self.surface),
            raised: f(self.raised),
            hover: f(self.hover),
            line: f(self.line),
            line_strong: f(self.line_strong),
            text: f(self.text),
            text2: f(self.text2),
            text3: f(self.text3),
            success: f(self.success),
            danger: f(self.danger),
            accent: f(self.accent),
            accent_soft: f(self.accent_soft),
            on_accent: f(self.on_accent),
            knob: f(self.knob),
            knob_off: f(self.knob_off),
            track: f(self.track),
            ink: f(self.ink),
            on_ink: f(self.on_ink),
            tile_ink: f(self.tile_ink),
            shadow: f(self.shadow),
            scrim: f(self.scrim),
            light: self.light,
            alpha: self.alpha * a,
        }
    }

    /// Translucent window (Mica behind it): the canvas and the panels let some of it through.
    /// Text, cards and controls stay solid.
    pub fn translucent(self) -> Colors {
        let see = |c: Color, a: f32| Color { a: c.a * a, ..c };
        Colors { canvas: see(self.canvas, 0.5), panel: see(self.panel, 0.86), sidebar: see(self.sidebar, 0.86), ..self }
    }

    /// A fixed colour (white buttons, shadows) faded along with the tokens.
    pub fn fixed(&self, c: Color) -> Color {
        Color { a: c.a * self.alpha, ..c }
    }
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

/// Dark text on light accents, white on dark ones (perceived brightness above 0.56 counts as
/// light: orange, green, mint and white get dark text; blue, purple and pink white).
pub fn on_accent(accent: Color) -> Color {
    let brightness = 0.299 * accent.r + 0.587 * accent.g + 0.114 * accent.b;
    if brightness > 0.56 { Color::from_rgb8(0x1a, 0x18, 0x16) } else { Color::WHITE }
}

/// The dark token set for an accent; an invalid accent falls back to the default orange.
#[cfg_attr(not(test), allow(dead_code))]
pub fn colors(accent_hex: &str) -> Colors {
    colors_for(accent_hex, false)
}

/// The dark (Figma) or light token set for an accent.
pub fn colors_for(accent_hex: &str, light: bool) -> Colors {
    let accent = parse_hex(accent_hex).or_else(|| parse_hex(DEFAULT_ACCENT)).unwrap_or(Color::WHITE);
    if light { light_colors(accent) } else { dark_colors(accent) }
}

/// Relative luminance (WCAG), 0 … 1.
fn lum(c: Color) -> f32 {
    let lin = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio (1 … 21) of `fg`, with its alpha, drawn over an opaque `bg`.
pub fn contrast(fg: Color, bg: Color) -> f32 {
    let a = fg.a;
    let fg = Color::from_rgb(bg.r + (fg.r - bg.r) * a, bg.g + (fg.g - bg.g) * a, bg.b + (fg.b - bg.b) * a);
    let (hi, lo) = if lum(fg) > lum(bg) { (lum(fg), lum(bg)) } else { (lum(bg), lum(fg)) };
    (hi + 0.05) / (lo + 0.05)
}

/// The near-black the light set writes with (the dark canvas, so both sets share one ink).
const GRAPHITE: Color = Color { r: 0x1a as f32 / 255.0, g: 0x18 as f32 / 255.0, b: 0x16 as f32 / 255.0, a: 1.0 };

/// The accent on a light panel: a near-white accent becomes graphite, a bright one is deepened
/// (same hue) until it reads as text and outlines (3:1).
fn light_accent(accent: Color, panel: Color) -> Color {
    if accent.r.min(accent.g).min(accent.b) > 0.85 {
        return GRAPHITE;
    }
    let mut c = accent;
    while contrast(c, panel) < 3.0 && c.r.max(c.g).max(c.b) > 0.05 {
        c = Color::from_rgb(c.r * 0.97, c.g * 0.97, c.b * 0.97);
    }
    c
}

/// The Figma colours mapped to a light set: warm off-whites in the same order (canvas behind,
/// panels on it, cards and raised controls a step darker), text as graphite at falling opacity.
fn light_colors(accent: Color) -> Colors {
    let ink = |a: f32| Color { a, ..GRAPHITE };
    let panel = Color::from_rgb8(0xfb, 0xfa, 0xf8);
    let accent = light_accent(accent, panel);
    Colors {
        canvas: Color::from_rgb8(0xec, 0xea, 0xe6),
        panel,
        sidebar: Color::from_rgb8(0xf3, 0xf1, 0xee),
        surface: Color::from_rgb8(0xf3, 0xf1, 0xed),
        raised: Color::from_rgb8(0xe8, 0xe5, 0xe1),
        hover: Color::from_rgb8(0xdf, 0xdb, 0xd6),
        line: ink(0.09),
        line_strong: ink(0.16),
        text: ink(0.94),
        text2: ink(0.74),
        text3: ink(0.6),
        success: Color::from_rgb8(0x1f, 0x8a, 0x3c),
        danger: Color::from_rgb8(0xd4, 0x2a, 0x1f),
        accent,
        accent_soft: Color { a: 0.16, ..accent },
        on_accent: on_accent(accent),
        knob: Color::WHITE,
        knob_off: Color::WHITE,
        track: Color::from_rgb8(0xc9, 0xc4, 0xbe),
        ink: GRAPHITE,
        on_ink: Color::WHITE,
        tile_ink: accent,
        shadow: ink(0.16),
        scrim: ink(0.28),
        light: true,
        alpha: 1.0,
    }
}

fn dark_colors(accent: Color) -> Colors {
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
        // Brighter than Figma's 0x91 / 0x4f: those greys are too faint on 100%-scaled screens.
        text2: white(0xa6 as f32 / 255.0),
        text3: white(0x73 as f32 / 255.0),
        success: Color::from_rgb8(0x32, 0xd7, 0x4b),
        danger: Color::from_rgb8(0xff, 0x45, 0x3a),
        accent,
        accent_soft: Color { a: 0.2, ..accent },
        on_accent: on_accent(accent),
        knob: if accent.r.min(accent.g).min(accent.b) > 0.85 { GRAPHITE } else { Color::WHITE },
        knob_off: Color::from_rgb8(0xb8, 0xb4, 0xaf),
        track: Color::from_rgb8(0x43, 0x3e, 0x39),
        ink: Color::WHITE,
        on_ink: GRAPHITE,
        tile_ink: Color::WHITE,
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.38),
        light: false,
        alpha: 1.0,
    }
}

/// The placeholder tile behind a video, song or picture without a thumbnail: the accent, softened,
/// fading into the panel (light: a pale tint, so the accent glyph on it reads).
pub fn tile_gradient(c: &Colors) -> (Color, Color) {
    let mix = |a: f32| {
        let (p, t) = (c.panel, c.accent);
        Color::from_rgba(p.r + (t.r - p.r) * a, p.g + (t.g - p.g) * a, p.b + (t.b - p.b) * a, c.alpha)
    };
    if c.light { (mix(0.22), mix(0.05)) } else { (mix(0.5), mix(0.06)) }
}

pub fn theme(c: &Colors) -> iced::Theme {
    iced::Theme::custom(
        "Snag",
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
        assert_ne!(on_accent(parse_hex("#32d74b").unwrap()), Color::WHITE, "white on green is too faint");
        assert_eq!(on_accent(parse_hex("#ff375f").unwrap()), Color::WHITE);
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
    fn faded_colors_scale_alpha_only() {
        let c = colors(DEFAULT_ACCENT);
        let half = c.faded(0.5);
        assert_eq!(half.accent, Color { a: 0.5, ..c.accent });
        assert_eq!(half.text.a, c.text.a * 0.5);
        assert_eq!(half.fixed(Color::WHITE), Color { a: 0.5, ..Color::WHITE });
        assert_eq!(c.faded(1.0), c, "fully in: unchanged");
    }

    #[test]
    fn tile_colors_follow_accent() {
        let hue = |c: Color| {
            let (max, min) = (c.r.max(c.g).max(c.b), c.r.min(c.g).min(c.b));
            if c.b == max && max > min { "blue" } else if c.r == max && max > min { "warm" } else { "grey" }
        };
        let blue = colors("#0a84ff");
        let (top, bottom) = tile_gradient(&blue);
        assert_eq!(hue(top), "blue", "{top:?}");
        assert_ne!(tile_gradient(&colors("#ff9f0a")).0, top, "another accent, another tile");
        // It fades into the panel, so a tile never shouts.
        assert!((bottom.r - blue.panel.r).abs() < 0.08 && (bottom.b - blue.panel.b).abs() < 0.12, "{bottom:?}");
        assert!(top.r + top.g + top.b < blue.accent.r + blue.accent.g + blue.accent.b, "softer than the accent itself");
    }

    #[test]
    fn dark_is_the_figma_set() {
        assert_eq!(colors_for(DEFAULT_ACCENT, false), colors(DEFAULT_ACCENT));
        let c = colors(DEFAULT_ACCENT);
        assert!(!c.light);
        assert_eq!(c.ink, Color::WHITE, "dark: white washes and a white primary button");
    }

    #[test]
    fn light_text_reads_well() {
        let c = colors_for(DEFAULT_ACCENT, true);
        assert!(c.light);
        for (name, bg) in [("panel", c.panel), ("surface", c.surface), ("sidebar", c.sidebar)] {
            assert!(contrast(c.text, bg) >= 10.0, "text on {name}: {}", contrast(c.text, bg));
            assert!(contrast(c.text2, bg) >= 6.0, "text2 on {name}: {}", contrast(c.text2, bg));
            assert!(contrast(c.text3, bg) >= 4.0, "text3 on {name}: {}", contrast(c.text3, bg));
            assert!(contrast(c.success, bg) >= 3.0 && contrast(c.danger, bg) >= 3.0, "status colours on {name}");
        }
        assert!(contrast(c.text3, c.canvas) >= 4.0, "the footer sits on the canvas");
        assert!(contrast(c.on_ink, c.ink) >= 10.0, "primary button label");
        // Hierarchy: each step is lighter than the one before.
        assert!(contrast(c.text, c.panel) > contrast(c.text2, c.panel) && contrast(c.text2, c.panel) > contrast(c.text3, c.panel));
        // Panels stand out from the canvas, and the lines between rows are visible.
        assert!(lum(c.panel) > lum(c.canvas) && contrast(c.line_strong, c.panel) > 1.2);
    }

    #[test]
    fn light_accent_stays_readable() {
        for (name, hex) in SWATCHES {
            let c = colors_for(hex, true);
            assert!(contrast(c.accent, c.panel) >= 3.0, "{name}: {}", contrast(c.accent, c.panel));
            assert!(contrast(c.on_accent, c.accent) >= 2.5, "{name}: label on the accent");
            assert!(contrast(c.tile_ink, tile_gradient(&c).0) >= 2.0, "{name}: tile glyph");
        }
        // A colour already dark enough is left alone; white turns into graphite.
        assert_eq!(colors_for("#0a84ff", true).accent, Color::from_rgb8(0x0a, 0x84, 0xff));
        let white = colors_for("#f5f5f7", true).accent;
        assert!(lum(white) < 0.05, "{white:?}");
        // Orange keeps its hue, only deeper.
        let orange = colors_for(DEFAULT_ACCENT, true).accent;
        assert!(orange.r > orange.g && orange.g > orange.b && orange.r > 0.7, "{orange:?}");
    }

    #[test]
    fn translucent_panels_only() {
        let c = colors(DEFAULT_ACCENT);
        let t = c.translucent();
        assert!(t.panel.a < 1.0 && t.panel.a > 0.7 && t.canvas.a < t.panel.a);
        assert_eq!((t.text, t.accent, t.raised), (c.text, c.accent, c.raised), "text and controls stay solid");
        assert_eq!(Colors { canvas: c.canvas, panel: c.panel, sidebar: c.sidebar, ..t }, c);
    }

    #[test]
    fn swatches_parse() {
        for (name, hex) in SWATCHES {
            assert!(parse_hex(hex).is_some(), "{name}");
        }
        assert_eq!(SWATCHES[0].1, DEFAULT_ACCENT);
    }
}
