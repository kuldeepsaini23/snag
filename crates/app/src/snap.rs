//! Debug builds only: renders the window to BMP files without touching the screen (works even
//! when other windows cover RDM). Used to check the Figma look.
//!
//! `RDM_SNAP_DIR=<dir>`: writes `<dir>/snap.bmp` every second.
//! `RDM_SNAP_SCENES=demo,selected,popover,…`: steps through those scenes, one per second,
//! writing `<dir>/<scene>.bmp`. Scenes only change what the window shows, never the downloads.

use crate::state::{Model, Picker, Screen, SettingsTab};
use crate::update::Message;
use iced::{Subscription, Task, window};
use rdm_core::{Category, Item, ItemId, Kind, MediaFormat, MediaInfo, Status};
use rdm_media::{Entry, QualityOption};
use std::sync::Mutex;

/// (next scene index, scene being shown, captured yet)
static STATE: Mutex<(usize, Option<String>, bool)> = Mutex::new((0, None, true));

pub fn dir() -> Option<std::path::PathBuf> {
    std::env::var_os("RDM_SNAP_DIR").map(Into::into)
}

fn scenes() -> Vec<String> {
    std::env::var("RDM_SNAP_SCENES").map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}

pub fn subscription() -> Subscription<Message> {
    if dir().is_some() { iced::time::every(std::time::Duration::from_millis(1200)).map(|_| Message::Snap) } else { Subscription::none() }
}

/// Alternates: set up the next scene, then (next tick, once it has been drawn) capture it.
pub fn take(model: &mut Model) -> Task<Message> {
    let list = scenes();
    if let Ok(mut st) = STATE.lock() {
        if st.2 {
            if let Some(name) = list.get(st.0).cloned() {
                apply(model, &name);
                st.0 += 1;
                st.1 = Some(name);
                st.2 = false;
                return Task::none();
            }
            st.1 = None;
        }
        st.2 = true;
    }
    window::latest().and_then(window::screenshot).map(Message::Snapped)
}

fn apply(m: &mut Model, scene: &str) {
    m.screen = Screen::Downloads;
    m.speed_open = false;
    m.toast = None;
    m.notice = None;
    m.picker = None;
    match scene {
        "demo" => m.items = demo_items(),
        "selected" => m.selected = m.items.iter().rev().find(|i| i.status == Status::Running).map(|i| i.id),
        "failed" => m.selected = m.items.iter().find(|i| matches!(i.status, Status::Failed(_))).map(|i| i.id),
        "popover" => {
            m.speed_open = true;
            m.settings.speed_limit_bps = 2 * 1024 * 1024;
        }
        "picker" => open_picker(m, info(vec![]), 2),
        "audio" => open_picker(m, info(vec![]), 5),
        "playlist" => {
            let entries = ["Why async Rust is different", "Futures and the Poll trait", "Pinning without pain", "Building a tiny executor", "Tokio runtime internals", "Channels, select! and cancellation", "Q&A livestream (unedited)"]
                .iter()
                .enumerate()
                .map(|(i, t)| Entry { url: format!("https://www.youtube.com/watch?v={i}"), title: t.to_string() })
                .collect();
            open_picker(m, info(entries), 2);
            if let Some(p) = &mut m.picker {
                p.toggle_entry(3);
                p.toggle_entry(6);
            }
        }
        "toast" => m.toast = Some("https://vimeo.com/824123456".into()),
        "notice" => m.notice = Some("Added “Rust Async Explained” (1080p)".into()),
        "empty" => m.items.clear(),
        "blue" => m.settings.accent = "#0a84ff".into(),
        "white" => m.settings.accent = "#f5f5f7".into(),
        "orange" => m.settings.accent = "#ff9f0a".into(),
        s if s.starts_with("settings-") => {
            let tab = match &s[9..] {
                "appearance" => SettingsTab::Appearance,
                "connections" => SettingsTab::Connections,
                "speed" => SettingsTab::Speed,
                "extension" => SettingsTab::Extension,
                "tools" => SettingsTab::Tools,
                _ => SettingsTab::General,
            };
            m.open_settings(tab);
        }
        _ => {}
    }
}

fn open_picker(m: &mut Model, info: MediaInfo, choice: usize) {
    m.picker = Some(Picker::new("https://www.youtube.com/watch?v=rust-async-explained".into(), info, choice, 0));
    m.screen = Screen::Picker;
}

fn info(entries: Vec<Entry>) -> MediaInfo {
    const MB: u64 = 1024 * 1024;
    let video = |h: u32, label: &str, size: u64| QualityOption { label: label.into(), format: MediaFormat::Video { max_height: h }, approx_size: Some(size * MB) };
    MediaInfo {
        title: if entries.is_empty() { "Rust Async Explained — Full Course".into() } else { "Tokio Deep Dive".into() },
        duration: Some(2538.0),
        options: vec![
            video(2160, "2160p", 3891),
            video(1440, "1440p", 2150),
            video(1080, "1080p", 1229),
            video(720, "720p", 640),
            video(480, "480p", 310),
            QualityOption { label: "MP3 audio".into(), format: MediaFormat::AudioMp3, approx_size: Some(96 * MB) },
        ],
        entries,
    }
}

fn demo_items() -> Vec<Item> {
    const MB: u64 = 1024 * 1024;
    let now = chrono::Local::now().timestamp();
    #[allow(clippy::too_many_arguments)]
    let item = |id: u64, name: &str, category: Category, status: Status, kind: Kind, done: u64, total: Option<u64>, speed: u64, ago: i64| Item {
        id: ItemId(1000 + id),
        url: format!("https://www.youtube.com/watch?v={id}"),
        name: name.into(),
        category,
        status,
        dest: None,
        downloaded: done,
        total,
        speed_bps: speed,
        queue: 0,
        added: now - ago,
        kind,
        referrer: None,
        work_dir: None,
    };
    let video = Kind::Media(MediaFormat::Video { max_height: 1080 });
    vec![
        item(1, "system-design-notes.pdf", Category::Document, Status::Done, Kind::Http, 12 * MB, Some(12 * MB), 0, 5000),
        item(2, "Tokio Deep Dive · Part 1.mp4", Category::Video, Status::Done, Kind::Media(MediaFormat::Video { max_height: 720 }), 412 * MB, Some(412 * MB), 0, 3000),
        item(3, "ubuntu-24.04-desktop-amd64.iso", Category::Archive, Status::Paused, Kind::Http, 2150 * MB, Some(5939 * MB), 0, 900),
        item(4, "Lo-fi Coding Mix.mp3", Category::Music, Status::Failed("Link expired (HTTP 403) · 41 of 96 MB kept".into()), Kind::Media(MediaFormat::AudioMp3), 41 * MB, Some(96 * MB), 0, 600),
        item(5, "Rust Async Explained — Full Course.mp4", Category::Video, Status::Running, video, 642 * MB, Some(1229 * MB), 8_810_000, 60),
    ]
}

/// Writes a top-down 32-bit BMP.
pub fn save(shot: &window::Screenshot) {
    let Some(dir) = dir() else { return };
    let name = STATE.lock().ok().and_then(|st| st.1.clone()).unwrap_or_else(|| "snap".into());
    let (w, h) = (shot.size.width, shot.size.height);
    let mut out = Vec::with_capacity(54 + shot.rgba.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + w * h * 4).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for px in shot.rgba.as_chunks::<4>().0 {
        out.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }
    let _ = std::fs::write(dir.join(format!("{name}.bmp")), out);
}
