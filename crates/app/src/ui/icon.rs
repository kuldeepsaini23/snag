//! Phosphor icons (MIT), drawn as glyphs from the bundled icon fonts.

use iced::Font;
use iced::widget::{Text, text};

pub const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
pub const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
/// The Snag logo (Figma: Components → "Logo/Snag").
const LOGO_64: &[u8] = include_bytes!("../../assets/logo/snag-64.png");

/// One handle for the whole run: iced loads images in the background and keys them by handle,
/// so a handle made anew every frame would never finish loading.
pub fn logo() -> iced::widget::image::Handle {
    static LOGO: std::sync::LazyLock<iced::widget::image::Handle> = std::sync::LazyLock::new(|| iced::widget::image::Handle::from_bytes(LOGO_64));
    LOGO.clone()
}
pub const PHOSPHOR: &[u8] = include_bytes!("../../assets/fonts/Phosphor.ttf");
pub const PHOSPHOR_FILL: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Fill.ttf");

pub const PHOSPHOR_FONT: Font = Font::with_name("Phosphor");
const FILL: Font = Font::with_name("Phosphor-Fill");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    AppWindow,
    ArrowClockwise,
    ArrowRight,
    Browser,
    CheckCircle,
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
    Lightning,
    Link,
    ListNumbers,
    MagnifyingGlass,
    Minus,
    MusicNote,
    MusicNotes,
    Palette,
    Pause,
    Play,
    PlayFill,
    Plug,
    Plus,
    Queue,
    Sidebar,
    Square,
    Trash,
    TrayDown,
    WarningCircle,
    Wrench,
    X,
}

impl Icon {
    /// Every icon (the font test checks each one).
    #[cfg(test)]
    pub const ALL: [Icon; 40] = [Icon::AppWindow, Icon::ArrowClockwise, Icon::ArrowRight, Icon::Browser, Icon::CheckCircle, Icon::ClipboardText, Icon::Clock, Icon::Copy, Icon::Download, Icon::File, Icon::FilePdf, Icon::FileText, Icon::FileZip, Icon::FilmStrip, Icon::FolderOpen, Icon::Folder, Icon::Gauge, Icon::Gear, Icon::Image, Icon::Lightning, Icon::Link, Icon::ListNumbers, Icon::MagnifyingGlass, Icon::Minus, Icon::MusicNote, Icon::MusicNotes, Icon::Palette, Icon::Pause, Icon::Play, Icon::PlayFill, Icon::Plug, Icon::Plus, Icon::Queue, Icon::Sidebar, Icon::Square, Icon::Trash, Icon::TrayDown, Icon::WarningCircle, Icon::Wrench, Icon::X];

    pub fn ch(self) -> char {
        let code = match self {
            Self::AppWindow => 0xe5da,
            Self::ArrowClockwise => 0xe036,
            Self::ArrowRight => 0xe06c,
            Self::Browser => 0xe0f4,
            Self::CheckCircle => 0xe184,
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
            Self::Lightning => 0xe2de,
            Self::Link => 0xe2e2,
            Self::ListNumbers => 0xe2f6,
            Self::MagnifyingGlass => 0xe30c,
            Self::Minus => 0xe32a,
            Self::MusicNote => 0xe33c,
            Self::MusicNotes => 0xe340,
            Self::Palette => 0xe6c8,
            Self::Pause => 0xe39e,
            Self::Play | Self::PlayFill => 0xe3d0,
            Self::Plug => 0xe946,
            Self::Plus => 0xe3d4,
            Self::Queue => 0xe6ac,
            Self::Sidebar => 0xec24,
            Self::Square => 0xe45e,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every glyph we use exists in the bundled font (its cmap maps the code point).
    #[test]
    fn icons_exist_in_font() {
        for i in Icon::ALL {
            let font = if i == Icon::PlayFill { PHOSPHOR_FILL } else { PHOSPHOR };
            assert!(maps(font, i.ch() as u32), "{i:?} missing");
        }
        assert!(!maps(PHOSPHOR, 0x41), "plain letters are not icons");
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
