//! The notification-area (tray) icon: RDM keeps running there when its window is closed.

use crate::update::Message;
use iced::futures::SinkExt;
use iced::Subscription;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub const ICON_32: &[u8] = include_bytes!("../assets/icon-32.rgba");
pub const ICON_64: &[u8] = include_bytes!("../assets/icon-64.rgba");

const OPEN: &str = "open";
const PAUSE_ALL: &str = "pause-all";
const RESUME_ALL: &str = "resume-all";
const QUIT: &str = "quit";

/// Keeps the icon alive (dropping it removes it from the tray).
pub struct Tray(#[allow(dead_code)] TrayIcon);

/// Must run on the UI thread (it owns the Windows message loop the icon talks to).
pub fn create() -> Option<Tray> {
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id(OPEN, "Open RDM", true, None),
        &MenuItem::with_id(PAUSE_ALL, "Pause all", true, None),
        &MenuItem::with_id(RESUME_ALL, "Resume all", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT, "Quit RDM", true, None),
    ])
    .ok()?;
    let icon = Icon::from_rgba(ICON_32.to_vec(), 32, 32).ok()?;
    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("RDM")
        .with_icon(icon)
        .build()
        .ok()
        .map(Tray)
}

pub fn subscription() -> Subscription<Message> {
    Subscription::run(events)
}

fn events() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, async |mut out: iced::futures::channel::mpsc::Sender<Message>| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
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
        while let Some(m) = rx.recv().await {
            if out.send(m).await.is_err() {
                break;
            }
        }
    })
}
