//! The window's look from the system: the app mode (light or dark, for "Follow Windows" /
//! "Follow system") and, on Windows, the Mica backdrop behind a translucent window.

use iced::Subscription;
use std::time::Duration;

/// How often "Follow Windows" looks at the app mode (a registry read: next to free).
const POLL: Duration = Duration::from_secs(2);

/// `AppsUseLightTheme` as read from the registry: 0 is dark; anything else, or no value at all
/// (Windows' own default), is light.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn light_from_registry(value: Option<u32>) -> bool {
    value.is_none_or(|v| v != 0)
}

/// Windows' "Choose your app mode" is Light.
#[cfg(windows)]
pub fn system_light() -> bool {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    light_from_registry(key.ok().and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme").ok()))
}

/// Linux: the desktop's style (GNOME's `color-scheme`, which the desktop portal and most
/// desktops follow): light unless it prefers dark. No GNOME settings: dark, Snag's own default.
#[cfg(not(windows))]
pub fn system_light() -> bool {
    gsetting("color-scheme").is_some_and(|v| light_from_color_scheme(&v))
}

/// `'prefer-dark'` is dark; `'default'` and `'prefer-light'` are light.
#[cfg_attr(windows, allow(dead_code))]
pub fn light_from_color_scheme(value: &str) -> bool {
    !value.contains("prefer-dark")
}

/// Linux: one `org.gnome.desktop.interface` key, as `gsettings` prints it.
#[cfg(not(windows))]
pub fn gsetting(key: &str) -> Option<String> {
    let out = crate::platform::host_command("gsettings").args(["get", "org.gnome.desktop.interface", key]).stderr(std::process::Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
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

    #[test]
    fn gnome_color_scheme_to_mode() {
        assert!(!light_from_color_scheme("'prefer-dark'"));
        assert!(light_from_color_scheme("'prefer-light'"));
        assert!(light_from_color_scheme("'default'"), "GNOME's default is light");
    }
}
