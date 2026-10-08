//! Phosphor icons (MIT), drawn as glyphs from the bundled icon fonts.

use iced::widget::image::Handle;
use iced::widget::{Text, text};
use iced::{Color, Font};
use std::sync::Mutex;

pub const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
pub const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
/// A logo as two 64×64 RGBA layers: the whole tile as drawn on copper #d9682b, and the drawing alone
/// (transparent around it).
type Layers = (&'static [u8], &'static [u8]);

/// The engraved logo: a hook catching a download arrow (the welcome tour, the app icon).
const LOGO: Layers = (crate::tray::ICON_64, include_bytes!("../../assets/logo-subject-64.rgba"));
/// The bold mark of the same hook and arrow, for small places (toolbar, settings).
const MARK: Layers = (include_bytes!("../../assets/mark-64.rgba"), include_bytes!("../../assets/mark-subject-64.rgba"));

/// A logo in the accent: the tile takes the accent, the drawing keeps its own ink and paper
/// (drawn over the tile as it is over the orange). The tile's rounded edge is kept.
fn tint((base, subject): Layers, accent: Color) -> Vec<u8> {
    let tile = [accent.r, accent.g, accent.b].map(|v| v * 255.0);
    let mut out = base.to_vec();
    let pixels = out.as_chunks_mut::<4>().0.iter_mut().zip(subject.as_chunks::<4>().0);
    for (px, sub) in pixels.filter(|(px, _)| px[3] > 0) {
        let a = sub[3] as f32 / 255.0;
        for k in 0..3 {
            px[k] = (sub[k] as f32 * a + tile[k] * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// The window's icon (taskbar, Alt+Tab), 64×64 RGBA: the bold mark in the accent.
pub fn taskbar_icon(accent: Color) -> Vec<u8> {
    tint(MARK, Color { a: 1.0, ..accent })
}

/// The tray icon, 32×32 RGBA: the taskbar icon at half size (each pixel a 2×2 average).
pub fn tray_icon(accent: Color) -> Vec<u8> {
    let big = taskbar_icon(accent);
    let mut out = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            for k in 0..4 {
                let at = |dx: usize, dy: usize| big[((2 * y + dy) * 64 + 2 * x + dx) * 4 + k] as u32;
                out.push(((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8);
            }
        }
    }
    out
}

/// Logos kept for recent accents. Dragging the colour picker passes through hundreds of colours;
/// only the last few are kept (the least recently drawn goes first).
const LOGO_CACHE: usize = 8;

/// Which logo (bold mark or not) in which accent.
type LogoKey = ([u8; 4], bool);

static LOGOS: Mutex<Vec<(LogoKey, Handle)>> = Mutex::new(Vec::new());

/// The engraved logo for an accent.
pub fn logo(accent: Color) -> Handle {
    tinted(LOGO, false, accent)
}

/// The bold mark for an accent (small sizes, where the engraving's fine lines blur).
pub fn mark(accent: Color) -> Handle {
    tinted(MARK, true, accent)
}

/// One handle per logo and accent colour, reused on every frame it's drawn at: iced keys uploaded
/// images by handle, so a handle made anew every frame would be uploaded anew.
fn tinted(layers: Layers, is_mark: bool, accent: Color) -> Handle {
    // A fading sheet draws the accent see-through; the image takes that as its opacity instead.
    let accent = Color { a: 1.0, ..accent };
    let key = (accent.into_rgba8(), is_mark);
    let Ok(mut cache) = LOGOS.lock() else { return Handle::from_rgba(64, 64, tint(layers, accent)) };
    let handle = match cache.iter().position(|(k, _)| *k == key) {
        Some(i) => cache.remove(i).1,
        None => Handle::from_rgba(64, 64, tint(layers, accent)),
    };
    cache.push((key, handle.clone()));
    if cache.len() > LOGO_CACHE {
        cache.remove(0);
    }
    handle
}
pub const PHOSPHOR: &[u8] = include_bytes!("../../assets/fonts/Phosphor.ttf");
pub const PHOSPHOR_FILL: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Fill.ttf");
/// Toolbar, caption and sidebar icons: Regular reads thin and cheap at these sizes.
pub const PHOSPHOR_BOLD: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Bold.ttf");

pub const PHOSPHOR_FONT: Font = Font::with_name("Phosphor");
const FILL: Font = Font::with_name("Phosphor-Fill");
pub const BOLD_FONT: Font = Font::with_name("Phosphor-Bold");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    AppWindow,
    ArrowClockwise,
    ArrowRight,
    Browser,
    Bug,
    CaretDown,
    ChartBar,
    CaretRight,
    Check,
    CheckCircle,
    DeviceMobile,
    ClipboardText,
    Clock,
    Copy,
    Download,
    File,
    FilePdf,
    FileText,
    FileZip,
    FilmStrip,
    FolderOpen,
    Folder,
    Gauge,
    Gear,
    Image,
    Keyboard,
    Lifebuoy,
    Link,
    ListNumbers,
    Magnet,
    MagnifyingGlass,
    MusicNote,
    MusicNotes,
    Palette,
    Pause,
    Play,
    PlayFill,
    Plug,
    Plus,
    Question,
    Queue,
    Rows,
    Sidebar,
    Sparkle,
    Square,
    SquaresFour,
    Trash,
    TrayDown,
    WarningCircle,
    Wrench,
    X,
}

impl Icon {
    /// Every icon (the font test checks each one).
    #[cfg(test)]
    pub const ALL: [Icon; 51] = [Icon::ChartBar, Icon::Rows, Icon::SquaresFour, Icon::AppWindow, Icon::ArrowClockwise, Icon::ArrowRight, Icon::Browser, Icon::Bug, Icon::CaretDown, Icon::CaretRight, Icon::Check, Icon::CheckCircle, Icon::DeviceMobile, Icon::ClipboardText, Icon::Clock, Icon::Copy, Icon::Download, Icon::File, Icon::FilePdf, Icon::FileText, Icon::FileZip, Icon::FilmStrip, Icon::FolderOpen, Icon::Folder, Icon::Gauge, Icon::Gear, Icon::Image, Icon::Keyboard, Icon::Lifebuoy, Icon::Link, Icon::ListNumbers, Icon::Magnet, Icon::MagnifyingGlass, Icon::MusicNote, Icon::MusicNotes, Icon::Palette, Icon::Pause, Icon::Play, Icon::PlayFill, Icon::Plug, Icon::Plus, Icon::Question, Icon::Queue, Icon::Sidebar, Icon::Sparkle, Icon::Square, Icon::Trash, Icon::TrayDown, Icon::WarningCircle, Icon::Wrench, Icon::X];

    pub fn ch(self) -> char {
        let code = match self {
            Self::AppWindow => 0xe5da,
            Self::ArrowClockwise => 0xe036,
            Self::ArrowRight => 0xe06c,
            Self::Browser => 0xe0f4,
            Self::Bug => 0xe5f4,
            Self::CaretDown => 0xe136,
            Self::ChartBar => 0xe150,
            Self::CaretRight => 0xe13a,
            Self::Check => 0xe182,
            Self::CheckCircle => 0xe184,
            Self::DeviceMobile => 0xe1e0,
            Self::ClipboardText => 0xe198,
            Self::Clock => 0xe19a,
            Self::Copy => 0xe1ca,
            Self::Download => 0xe20c,
            Self::File => 0xe230,
            Self::FilePdf => 0xe702,
            Self::FileText => 0xe23a,
            Self::FileZip => 0xe958,
            Self::FilmStrip => 0xe792,
            Self::FolderOpen => 0xe256,
            Self::Folder => 0xe25a,
            Self::Gauge => 0xe628,
            Self::Gear => 0xe272,
            Self::Image => 0xe2ca,
            Self::Keyboard => 0xe2d8,
            Self::Lifebuoy => 0xe63a,
            Self::Link => 0xe2e2,
            Self::ListNumbers => 0xe2f6,
            Self::Magnet => 0xe680,
            Self::MagnifyingGlass => 0xe30c,
            Self::MusicNote => 0xe33c,
            Self::MusicNotes => 0xe340,
            Self::Palette => 0xe6c8,
            Self::Pause => 0xe39e,
            Self::Play | Self::PlayFill => 0xe3d0,
            Self::Plug => 0xe946,
            Self::Plus => 0xe3d4,
            Self::Question => 0xe3e8,
            Self::Queue => 0xe6ac,
            Self::Rows => 0xe5a2,
            Self::Sidebar => 0xec24,
            Self::Sparkle => 0xe6a2,
            Self::Square => 0xe45e,
            Self::SquaresFour => 0xe464,
            Self::Trash => 0xe4a6,
            Self::TrayDown => 0xe010,
            Self::WarningCircle => 0xe4e2,
            Self::Wrench => 0xe5d4,
            Self::X => 0xe4f6,
        };
        char::from_u32(code).unwrap_or('?')
    }

    fn font(self) -> Font {
        if self == Self::PlayFill { FILL } else { PHOSPHOR_FONT }
    }
}

pub fn icon<'a>(i: Icon, size: u16) -> Text<'a> {
    text(i.ch().to_string()).font(i.font()).size(size as f32).line_height(1.0)
}

/// Toolbar, caption and sidebar icons.
pub fn bold<'a>(i: Icon, size: u16) -> Text<'a> {
    text(i.ch().to_string()).font(BOLD_FONT).size(size as f32).line_height(1.0)
}

/// The active filter or nav item.
pub fn filled<'a>(i: Icon, size: u16) -> Text<'a> {
    text(i.ch().to_string()).font(FILL).size(size as f32).line_height(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every glyph we use exists in the bundled font (its cmap maps the code point).
    #[test]
    fn icons_exist_in_font() {
        for i in Icon::ALL {
            let font = if i == Icon::PlayFill { PHOSPHOR_FILL } else { PHOSPHOR };
            assert!(maps(font, i.ch() as u32), "{i:?} missing");
            assert!(maps(PHOSPHOR_BOLD, i.ch() as u32), "{i:?} missing in Bold");
            assert!(maps(PHOSPHOR_FILL, i.ch() as u32), "{i:?} missing in Fill");
        }
        assert!(!maps(PHOSPHOR, 0x41), "plain letters are not icons");
    }

    #[test]
    fn logo_follows_the_accent() {
        let px = |rgba: &[u8], i: usize| [rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], rgba[i * 4 + 3]];
        let (base, subject) = LOGO;
        let alpha = |i: usize| subject[i * 4 + 3];
        // Inside the tile: bare orange (no drawing), and the drawing itself (fully opaque).
        let tile = (0..64 * 64).find(|&i| base[i * 4 + 3] == 255 && alpha(i) == 0).expect("bare tile");
        let drawing = (0..64 * 64).find(|&i| alpha(i) == 255).expect("the hook or arrow");
        let corner = 0; // transparent outside the rounded square

        let orange = tint(LOGO, crate::ui::theme::parse_hex(crate::ui::theme::DEFAULT_ACCENT).unwrap());
        assert!(orange.iter().zip(base).all(|(a, b)| a.abs_diff(*b) <= 10), "the default accent is the logo as drawn");

        let blue = tint(LOGO, crate::ui::theme::parse_hex("#0a84ff").unwrap());
        assert_eq!(px(&blue, tile), [0x0a, 0x84, 0xff, 255], "the tile takes the accent");
        assert_eq!(px(&blue, drawing)[..3], px(subject, drawing)[..3], "the engraving keeps its own ink and paper");
        assert_eq!(px(&blue, corner)[3], 0, "transparency is kept");
    }

    #[test]
    fn the_small_mark_follows_the_accent_too() {
        let px = |rgba: &[u8], i: usize| [rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], rgba[i * 4 + 3]];
        let (base, sub) = (MARK.0, MARK.1);
        let tile = (0..64 * 64).find(|&i| base[i * 4 + 3] == 255 && sub[i * 4 + 3] == 0).expect("bare tile");
        let blue = tint(MARK, crate::ui::theme::parse_hex("#0a84ff").unwrap());
        assert_eq!(px(&blue, tile), [0x0a, 0x84, 0xff, 255]);
        let orange = tint(MARK, crate::ui::theme::parse_hex(crate::ui::theme::DEFAULT_ACCENT).unwrap());
        assert!(orange.iter().zip(base).all(|(a, b)| a.abs_diff(*b) <= 10), "the default accent is the mark as drawn");
        assert_ne!(mark(Color::from_rgb8(1, 2, 3)).id(), logo(Color::from_rgb8(1, 2, 3)).id(), "two different images");
    }

    #[test]
    fn taskbar_and_tray_icons_take_the_accent() {
        let blue = crate::ui::theme::parse_hex("#0a84ff").unwrap();
        let big = taskbar_icon(blue);
        assert_eq!(big, tint(MARK, blue), "the window's icon is the bold mark in the accent");
        let small = tray_icon(blue);
        assert_eq!(small.len(), 32 * 32 * 4);
        // Each tray pixel is the average of a 2×2 block of the big one.
        let at = |rgba: &[u8], w: usize, x: usize, y: usize| rgba[(y * w + x) * 4 + 2] as u32;
        let (x, y) = (16, 2);
        let avg = (at(&big, 64, 2 * x, 2 * y) + at(&big, 64, 2 * x + 1, 2 * y) + at(&big, 64, 2 * x, 2 * y + 1) + at(&big, 64, 2 * x + 1, 2 * y + 1) + 2) / 4;
        assert!(at(&small, 32, x, y).abs_diff(avg) <= 1);
    }

    #[test]
    fn logo_handles_are_reused_and_bounded() {
        let blue = crate::ui::theme::parse_hex("#0a84ff").unwrap();
        assert_eq!(logo(blue).id(), logo(blue).id(), "the same accent draws with the same handle, frame after frame");
        // A drag through many colours: the one in use stays, the cache stays small.
        for k in 0..200u8 {
            let _ = logo(Color::from_rgb8(k, 255 - k, 128));
            assert_eq!(logo(blue).id(), logo(blue).id());
        }
        assert!(LOGOS.lock().unwrap().len() <= LOGO_CACHE);
    }

    /// Minimal cmap (format 4) lookup, enough to prove the code points are real.
    fn maps(font: &[u8], cp: u32) -> bool {
        let u16_at = |o: usize| u16::from_be_bytes([font[o], font[o + 1]]) as usize;
        let u32_at = |o: usize| u32::from_be_bytes([font[o], font[o + 1], font[o + 2], font[o + 3]]) as usize;
        let tables = u16_at(4);
        let Some(cmap) = (0..tables).map(|i| 12 + 16 * i).find(|&r| &font[r..r + 4] == b"cmap").map(|r| u32_at(r + 8)) else { return false };
        for s in 0..u16_at(cmap + 2) {
            let sub = cmap + u32_at(cmap + 4 + 8 * s + 4);
            if u16_at(sub) != 4 {
                continue;
            }
            let segs = u16_at(sub + 6) / 2;
            let ends = sub + 14;
            let starts = ends + 2 * segs + 2;
            if (0..segs).any(|k| (u16_at(starts + 2 * k)..=u16_at(ends + 2 * k)).contains(&(cp as usize))) {
                return true;
            }
        }
        false
    }
}
