//! The notification-area (tray) icon: Snag keeps running there when its window is closed.
//! Windows: it lives on the UI thread. Linux: a StatusNotifierItem over D-Bus (ksni, no GTK),
//! served from a thread of its own, which the `Tray` handle sends new icons to; KDE, Cinnamon
//! and Xfce show it, GNOME with the AppIndicator extension. With no tray host it waits quietly
//! for one. macOS: a menu bar item, made on the main thread once the app is running
//! (`Message::MakeTray`), drawn as a template image in the menu bar's own colour.

use crate::update::Message;
use iced::futures::SinkExt;
use iced::{Color, Subscription};
#[cfg(not(target_os = "linux"))]
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
#[cfg(not(target_os = "linux"))]
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub const ICON_32: &[u8] = include_bytes!("../assets/icon-32.rgba");
pub const ICON_64: &[u8] = include_bytes!("../assets/icon-64.rgba");

#[cfg(not(target_os = "linux"))]
const OPEN: &str = "open";
#[cfg(not(target_os = "linux"))]
const PAUSE_ALL: &str = "pause-all";
#[cfg(not(target_os = "linux"))]
const RESUME_ALL: &str = "resume-all";
#[cfg(not(target_os = "linux"))]
const QUIT: &str = "quit";

/// Keeps the icon alive (dropping it removes it from the tray).
#[cfg(not(target_os = "linux"))]
pub struct Tray(TrayIcon);

#[cfg(not(target_os = "linux"))]
impl Tray {
    /// Redraws the tray icon in a new accent. (macOS keeps its template.)
    pub fn set_icon(&self, accent: Color) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let Ok(icon) = Icon::from_rgba(crate::ui::icon::tray_icon(accent), 32, 32) {
            let _ = self.0.set_icon(Some(icon));
        }
    }
}

/// Must run on the UI thread (it owns the Windows message loop the icon talks to). macOS: on the
/// main thread after the app has started (NSStatusItem), i.e. from `update`, not at boot.
#[cfg(not(target_os = "linux"))]
pub fn create() -> Option<Tray> {
    build().map(Tray)
}

/// Linux: the icon's thread takes new pictures from here (dropping it removes the icon).
#[cfg(target_os = "linux")]
pub struct Tray(std::sync::mpsc::Sender<Vec<ksni::Icon>>);

#[cfg(target_os = "linux")]
impl Tray {
    /// Redraws the tray icon in a new accent.
    pub fn set_icon(&self, accent: Color) {
        let _ = self.0.send(pixmaps(&crate::ui::icon::tray_icon(accent), &crate::ui::icon::taskbar_icon(accent)));
    }
}

/// Linux: puts the icon on the session bus from a thread of its own (ksni's blocking calls can't
/// run inside the UI's runtime). Without a tray host yet (or ever) it waits for one, quietly.
/// `None` when there is no session bus or the tray watcher turns the icon down.
#[cfg(target_os = "linux")]
pub fn create() -> Option<Tray> {
    use ksni::blocking::TrayMethods;
    let (icons, rx) = std::sync::mpsc::channel::<Vec<ksni::Icon>>();
    let (ready, started) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            let tray = LinuxTray { icons: pixmaps(ICON_32, ICON_64) }; // replaced by the accent's at once
            let handle = match tray.assume_sni_available(true).spawn() {
                Ok(handle) => handle,
                Err(e) => {
                    eprintln!("tray: {e}");
                    let _ = ready.send(false);
                    return;
                }
            };
            let _ = ready.send(true);
            while let Ok(icons) = rx.recv() {
                handle.update(|tray| tray.icons = icons);
            }
            handle.shutdown().wait();
        })
        .ok()?;
    // A session bus that doesn't answer mustn't hold up the window: the icon comes when it can.
    match started.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(true) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Some(Tray(icons)),
        Ok(false) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => None,
    }
}

/// Linux: the icon at 32 and 64 px (RGBA in), as StatusNotifierItem pixmaps want them: ARGB32 in
/// network byte order.
#[cfg(target_os = "linux")]
fn pixmaps(small: &[u8], big: &[u8]) -> Vec<ksni::Icon> {
    [(small, 32), (big, 64)]
        .into_iter()
        .map(|(rgba, size)| ksni::Icon {
            width: size,
            height: size,
            data: rgba.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| [a, r, g, b]).collect(),
        })
        .collect()
}

#[cfg(target_os = "linux")]
struct LinuxTray {
    icons: Vec<ksni::Icon>,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        "snag".into()
    }

    fn title(&self) -> String {
        "Snag".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: "Snag".into(), ..Default::default() }
    }

    /// A left click opens the window.
    fn activate(&mut self, _x: i32, _y: i32) {
        send(Message::TrayOpen);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let item = |label: &str, message: fn() -> Message| {
            ksni::menu::StandardItem { label: label.into(), activate: Box::new(move |_: &mut Self| send(message())), ..Default::default() }.into()
        };
        vec![
            item("Open Snag", || Message::TrayOpen),
            item("Pause all", || Message::PauseAll),
            item("Resume all", || Message::ResumeAll),
            ksni::MenuItem::Separator,
            item("Quit Snag", || Message::TrayQuit),
        ]
    }
}

#[cfg(not(target_os = "linux"))]
fn build() -> Option<TrayIcon> {
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id(OPEN, "Open Snag", true, None),
        &MenuItem::with_id(PAUSE_ALL, "Pause all", true, None),
        &MenuItem::with_id(RESUME_ALL, "Resume all", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT, "Quit Snag", true, None),
    ])
    .ok()?;
    let tray = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("Snag");
    // macOS: the mark's ink as a template (the menu bar tints it); its menu opens on a click, as
    // menu bar items do there.
    #[cfg(target_os = "macos")]
    let tray = tray.with_menu_on_left_click(true).with_icon_templated(Icon::from_rgba(crate::ui::icon::tray_template(), 32, 32).ok()?);
    #[cfg(not(target_os = "macos"))]
    let tray = tray.with_menu_on_left_click(false).with_icon(Icon::from_rgba(ICON_32.to_vec(), 32, 32).ok()?); // replaced by the accent's at once
    tray.build().ok()
}

/// Where tray clicks go (the running `events` stream).
static SHOW: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Message>>> = std::sync::Mutex::new(None);

/// Hands a tray click to the app.
fn send(message: Message) {
    if let Some(tx) = SHOW.lock().ok().and_then(|o| o.clone()) {
        let _ = tx.send(message);
    }
}

/// Shows the window, as the tray's Open Snag does (a click on a notification; macOS: on the Dock
/// icon). Any thread.
pub fn open_window() {
    send(Message::TrayOpen);
}

pub fn subscription() -> Subscription<Message> {
    Subscription::run(events)
}

fn events() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, async |mut out: iced::futures::channel::mpsc::Sender<Message>| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        if let Ok(mut open) = SHOW.lock() {
            *open = Some(tx.clone());
        }
        // Linux: the icon's thread sends its clicks through `SHOW`.
        #[cfg(not(target_os = "linux"))]
        listen(tx);
        while let Some(m) = rx.recv().await {
            if out.send(m).await.is_err() {
                break;
            }
        }
    })
}

/// Windows and macOS: tray-icon's menu and click events, into the `events` stream.
#[cfg(not(target_os = "linux"))]
fn listen(tx: tokio::sync::mpsc::UnboundedSender<Message>) {
    let clicks = tx.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let message = match e.id.0.as_str() {
            OPEN => Some(Message::TrayOpen),
            PAUSE_ALL => Some(Message::PauseAll),
            RESUME_ALL => Some(Message::ResumeAll),
            QUIT => Some(Message::TrayQuit),
            _ => None,
        };
        if let Some(m) = message {
            let _ = tx.send(m);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
        let open = matches!(
            e,
            TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
        );
        if open {
            let _ = clicks.send(Message::TrayOpen);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_window_asks_the_app_to_show_its_window() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        *SHOW.lock().unwrap() = Some(tx);
        open_window();
        *SHOW.lock().unwrap() = None;
        assert!(matches!(rx.try_recv(), Ok(Message::TrayOpen)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pixmaps_are_argb_at_32_and_64_px() {
        let icons = super::pixmaps(&[1, 2, 3, 4].repeat(32 * 32), &[5, 6, 7, 8].repeat(64 * 64));
        assert_eq!((icons[0].width, icons[0].height, icons[0].data.len()), (32, 32, 32 * 32 * 4));
        assert_eq!((icons[1].width, icons[1].height, icons[1].data.len()), (64, 64, 64 * 64 * 4));
        assert_eq!(icons[0].data[..4], [4, 1, 2, 3]);
        assert_eq!(icons[1].data[..4], [8, 5, 6, 7]);
    }
}
