//! Notifications for finished and failed downloads: Windows toasts, desktop notifications over
//! D-Bus on Linux, the Notification Center on macOS.

use crate::state::Note;
use std::path::Path;

/// The identity Windows shows toasts under (registered per user at start-up).
#[cfg(windows)]
pub const APP_ID: &str = "Snag.DownloadManager";

/// Lets an unpackaged app send toasts: registers its name and icon under HKCU.
#[cfg(windows)]
pub fn register(data_dir: &Path) {
    let icon = data_dir.join("icon.ico");
    let _ = std::fs::create_dir_all(data_dir);
    let _ = std::fs::write(&icon, ico_from_rgba(64, 64, crate::tray::ICON_64));
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}")) {
        let _ = key.set_value("DisplayName", &"Snag");
        let _ = key.set_value("IconUri", &icon.display().to_string());
    }
}

/// Shows one toast (blocking: call off the UI thread).
#[cfg(windows)]
pub fn show(note: &Note) {
    let _ = tauri_winrt_notification::Toast::new(APP_ID).title(&note.title).text1(&note.body).show();
}

/// Linux: the icon notifications show, as a PNG in the data folder (an installed `snag` icon
/// isn't there for a bare AppImage).
#[cfg(target_os = "linux")]
static ICON: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "linux")]
pub fn register(data_dir: &Path) {
    let icon = data_dir.join("icon.png");
    let _ = std::fs::create_dir_all(data_dir);
    if image::save_buffer(&icon, crate::tray::ICON_64, 64, 64, image::ColorType::Rgba8).is_ok() {
        let _ = ICON.set(icon);
    }
}

/// macOS: notifications come from Snag.app (its name and icon) by its bundle identifier. Not set
/// when that app isn't installed (a bare `cargo run`): they then come from the default app.
#[cfg(target_os = "macos")]
pub fn register(_data_dir: &Path) {
    let _ = notify_rust::set_application(crate::platform::BUNDLE_ID);
}

/// macOS: one Notification Center notification (blocking: call off the UI thread).
#[cfg(target_os = "macos")]
pub fn show(note: &Note) {
    let _ = notify_rust::Notification::new().summary(&note.title).body(&note.body).show();
}

/// One desktop notification (blocking: call off the UI thread).
#[cfg(target_os = "linux")]
pub fn show(note: &Note) {
    let mut n = notify_rust::Notification::new();
    n.appname("Snag").summary(&note.title).body(&note.body);
    match ICON.get() {
        Some(icon) => n.icon(&icon.to_string_lossy()),
        None => n.icon("snag"),
    };
    let _ = n.show();
}

/// A single-image .ico (32-bit BMP payload) from top-down RGBA pixels.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn ico_from_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mask_row = width.div_ceil(32) * 4;
    let image_len = 40 + width * height * 4 + mask_row * height;
    let mut out = Vec::with_capacity(22 + image_len as usize);
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // ICONDIR: icon, 1 image
    out.push(if width >= 256 { 0 } else { width as u8 });
    out.push(if height >= 256 { 0 } else { height as u8 });
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&image_len.to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    // BITMAPINFOHEADER (height counts the AND mask too).
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&((height * 2) as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            let px = rgba.get(i..i + 4).unwrap_or(&[0, 0, 0, 0]);
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    out.extend(std::iter::repeat_n(0u8, (mask_row * height) as usize));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_has_one_32bit_image() {
        let ico = ico_from_rgba(64, 64, crate::tray::ICON_64);
        assert_eq!(&ico[0..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!((ico[6], ico[7]), (64, 64));
        assert_eq!(u16::from_le_bytes([ico[12], ico[13]]), 32, "32 bits per pixel");
        let len = u32::from_le_bytes([ico[14], ico[15], ico[16], ico[17]]) as usize;
        assert_eq!(ico.len(), 22 + len, "directory size matches the payload");
        // Top-left pixel of the icon is transparent (rounded corner); it is the first pixel of the last BMP row.
        let last_row = 22 + 40 + 63 * 64 * 4;
        assert_eq!(ico[last_row + 3], crate::tray::ICON_64[3]);
    }
}

#[cfg(all(test, windows))]
mod real {
    /// Shows a real toast on this PC: `cargo test -p rdm-app real_toast -- --ignored`.
    #[test]
    #[ignore]
    fn real_toast() {
        let dir = rdm_core::store::data_dir(&std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_default());
        super::register(&dir);
        let r = tauri_winrt_notification::Toast::new(super::APP_ID).title("Snag notifications work").text1("You'll see one of these when a download finishes.").show();
        assert!(r.is_ok(), "{r:?}");
    }
}
