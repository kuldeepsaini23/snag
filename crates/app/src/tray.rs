//! The notification-area (tray) icon: Snag keeps running there when its window is closed.
//! Windows: it lives on the UI thread. Linux: on its own GTK thread (an AppIndicator; GNOME
//! shows it with the AppIndicator extension), which the `Tray` handle sends new icons to.
//! macOS: a menu bar item, made on the main thread once the app is running (`Message::MakeTray`),
//! drawn as a template image in the menu bar's own colour.

use crate::update::Message;
use iced::futures::SinkExt;
use iced::Subscription;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub const ICON_32: &[u8] = include_bytes!("../assets/icon-32.rgba");
pub const ICON_64: &[u8] = include_bytes!("../assets/icon-64.rgba");

const OPEN: &str = "open";
const PAUSE_ALL: &str = "pause-all";
const RESUME_ALL: &str = "resume-all";
const QUIT: &str = "quit";

/// Keeps the icon alive (dropping it removes it from the tray).
#[cfg(not(target_os = "linux"))]
pub struct Tray(TrayIcon);

#[cfg(not(target_os = "linux"))]
impl Tray {
    /// Redraws the tray icon (32×32 RGBA), e.g. in a new accent. (macOS keeps its template.)
    pub fn set_icon(&self, rgba: Vec<u8>) {
        if cfg!(target_os = "macos") {
            return;
        }
        if let Ok(icon) = Icon::from_rgba(rgba, 32, 32) {
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

/// Linux: the icon's GTK thread takes new pictures from here (dropping it ends the thread).
#[cfg(target_os = "linux")]
pub struct Tray(std::sync::mpsc::Sender<Vec<u8>>);

#[cfg(target_os = "linux")]
impl Tray {
    /// Redraws the tray icon (32×32 RGBA), e.g. in a new accent.
    pub fn set_icon(&self, rgba: Vec<u8>) {
        let _ = self.0.send(rgba);
    }
}

/// Linux: starts GTK on a thread of its own and builds the icon there (tray-icon needs a GTK
/// main loop on the thread that made it). `None` when there is no display or GTK won't start.
#[cfg(target_os = "linux")]
pub fn create() -> Option<Tray> {
    let (icons, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let (ready, started) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            if gtk::init().is_err() {
                let _ = ready.send(false);
                return;
            }
            let Some(tray) = build() else {
                let _ = ready.send(false);
                return;
            };
            let _ = ready.send(true);
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                loop {
                    match rx.try_recv() {
                        Ok(rgba) => {
                            if let Ok(icon) = Icon::from_rgba(rgba, 32, 32) {
                                let _ = tray.set_icon(Some(icon));
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => return gtk::glib::ControlFlow::Continue,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            gtk::main_quit();
                            return gtk::glib::ControlFlow::Break;
                        }
                    }
                }
            });
            gtk::main();
        })
        .ok()?;
    started.recv().ok()?.then_some(Tray(icons))
}

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

/// Shows the window, as the tray's Open Snag does (macOS: a click on the Dock icon).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn open_window() {
    if let Some(tx) = SHOW.lock().ok().and_then(|o| o.clone()) {
        let _ = tx.send(Message::TrayOpen);
    }
}

pub fn subscription() -> Subscription<Message> {
    Subscription::run(events)
}

fn events() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, async |mut out: iced::futures::channel::mpsc::Sender<Message>| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let clicks = tx.clone();
        if let Ok(mut open) = SHOW.lock() {
            *open = Some(tx.clone());
        }
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
        while let Some(m) = rx.recv().await {
            if out.send(m).await.is_err() {
                break;
            }
        }
    })
}
