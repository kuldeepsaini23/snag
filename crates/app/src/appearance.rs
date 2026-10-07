//! The window's look from Windows: the app mode (light or dark, for "Follow Windows") and the
//! Mica backdrop behind a translucent window.

use iced::Subscription;
use std::time::Duration;

/// How often "Follow Windows" looks at the app mode (a registry read: next to free).
const POLL: Duration = Duration::from_secs(2);

/// `AppsUseLightTheme` as read from the registry: 0 is dark; anything else, or no value at all
/// (Windows' own default), is light.
pub fn light_from_registry(value: Option<u32>) -> bool {
    value.is_none_or(|v| v != 0)
}

/// Windows' "Choose your app mode" is Light.
#[cfg(windows)]
pub fn system_light() -> bool {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    light_from_registry(key.ok().and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme").ok()))
}

#[cfg(not(windows))]
pub fn system_light() -> bool {
    false
}

/// The app mode now, then again each time it changes (only subscribed while following Windows).
pub fn system_theme() -> Subscription<bool> {
    Subscription::run(|| {
        futures_util::stream::unfold(None, |last: Option<bool>| async move {
            loop {
                if last.is_some() {
                    tokio::time::sleep(POLL).await;
                }
                let now = system_light();
                if last != Some(now) {
                    return Some((now, Some(now)));
                }
            }
        })
    })
}

/// Puts Mica (Windows 11) or acrylic (Windows 10) behind the window, tinted `light` or dark, or
/// takes it away (`on` false). True when a backdrop is behind the window now.
pub fn set_backdrop(window: &dyn iced::window::Window, on: bool, light: bool) -> bool {
    if !on {
        let _ = window_vibrancy::clear_mica(window);
        let _ = window_vibrancy::clear_acrylic(window);
        return false;
    }
    if window_vibrancy::apply_mica(window, Some(!light)).is_ok() {
        return true;
    }
    // Windows 10: acrylic, tinted like the canvas.
    let tint = if light { (0xec, 0xea, 0xe6, 0xa0) } else { (0x1a, 0x18, 0x16, 0xa0) };
    window_vibrancy::apply_acrylic(window, Some(tint)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_value_to_mode() {
        assert!(!light_from_registry(Some(0)));
        assert!(light_from_registry(Some(1)));
        assert!(light_from_registry(None), "no value: Windows' default, light");
    }
}
