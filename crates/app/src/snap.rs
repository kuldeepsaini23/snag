//! Debug builds only: renders the window to BMP files without touching the screen (works even
//! when other windows cover RDM). Used to check the Figma look.
//!
//! `RDM_SNAP_DIR=<dir>`: writes `<dir>/snap.bmp` every second.
//! `RDM_SNAP_SCENES=demo,selected,popover,…`: steps through those scenes, one per second,
//! writing `<dir>/<scene>.bmp`. Scenes only change what the window shows; when the list ends the
//! window reloads the real state from the manager, so nothing a scene set can be saved by a later
//! click (e.g. closing Settings).

use crate::state::{Info, Model, Picker, Screen, SettingsTab};
use crate::update::{App, Message};
use iced::widget::text_editor;
use iced::{Subscription, Task, window};
use rdm_core::{Category, Item, ItemId, Kind, MediaFormat, MediaInfo, Status};
use rdm_media::{Entry, QualityOption};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// (next scene index, scene being shown, captured yet)
static STATE: Mutex<(usize, Option<String>, bool)> = Mutex::new((0, None, true));
/// `mid-*` scenes: the instant the view draws at, so a capture shows an animation halfway.
static FROZEN: Mutex<Option<Instant>> = Mutex::new(None);

/// How far into its animation a `mid-*` scene is captured.
const MID: Duration = Duration::from_millis(60);

pub fn frozen() -> Option<Instant> {
    FROZEN.lock().ok().and_then(|f| *f)
}

fn freeze(at: Option<Instant>) {
    if let Ok(mut f) = FROZEN.lock() {
        *f = at;
    }
}

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
pub fn take(app: &mut App) -> Task<Message> {
    let model = &mut app.model;
    let list = scenes();
    if let Ok(mut st) = STATE.lock() {
        if st.2 {
            if let Some(name) = list.get(st.0).cloned() {
                apply(model, &name);
                if name == "bug-report" {
                    let typed = "I pressed Download on a YouTube video and nothing happened.\nThe row stayed at 0 B for a minute, then said HTTP 403.";
                    app.bug_text = text_editor::Content::with_text(typed);
                }
                st.0 += 1;
                st.1 = Some(name.clone());
                st.2 = false;
                if name == "narrow" {
                    return window::latest().and_then(|id| window::resize(id, iced::Size::new(960.0, 600.0)));
                }
                return Task::none();
            }
            if st.1.take().is_some() {
                // Last scene captured: back to the real state.
                st.2 = true;
                apply(model, "");
                model.selected = None;
                model.speed_preview = None;
                return Task::done(Message::Lagged);
            }
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
    m.confirm_quit = false;
    m.sidebar_open = true;
    m.search_open = false;
    m.hovered = None;
    m.pending_cancel = None;
    m.more_open = false;
    m.help_open = false;
    m.info = None;
    freeze(scene.starts_with("mid-").then(|| Instant::now() + MID));
    // The row that shows Pause/Resume and Cancel: the paused ISO.
    let paused = m.items.iter().find(|i| i.status == Status::Paused).map(|i| i.id);
    match scene {
        "hover" => m.hovered = paused,
        "cancel" => {
            m.selected = paused;
            m.pending_cancel = paused;
        }
        "search" => {
            m.search_open = true;
            m.search = String::new();
        }
        "more" => m.more_open = true,
        "no-sidebar" => {
            m.sidebar_open = false;
            m.selected = None;
        }
        // Halfway: the sidebar closing while the tab line slides from All to Done.
        "mid-slide" => {
            m.sidebar_open = false;
            m.filter = if m.filter == crate::view::Filter::Done { crate::view::Filter::All } else { crate::view::Filter::Done };
        }
        "mid-inspector" => {
            m.filter = crate::view::Filter::All;
            m.selected = m.items.iter().rev().find(|i| i.status == Status::Running).map(|i| i.id);
        }
        // A download that just arrived fades in; one that just finished pulses.
        "mid-new" => {
            let mut fresh = m.items[0].clone();
            (fresh.id, fresh.name, fresh.status) = (ItemId(3000), "Just added — release-notes.pdf".into(), Status::Queued);
            m.items.push(fresh);
        }
        "mid-done" => {
            if let Some(i) = m.items.iter_mut().find(|i| i.status == Status::Running) {
                i.status = Status::Done;
            }
        }
        "mid-picker" => open_picker(m, info(vec![]), 2),
        "mid-toast" => m.toast = Some("https://vimeo.com/824123456".into()),
        "all" => {
            m.filter = crate::view::Filter::All;
            m.selected = None;
        }
        "demo" => m.items = demo_items(),
        "selected" => m.selected = m.items.iter().rev().find(|i| i.status == Status::Running).map(|i| i.id),
        "failed" => m.selected = m.items.iter().find(|i| matches!(i.status, Status::Failed(_))).map(|i| i.id),
        "popover" => {
            m.speed_open = true;
            m.speed_preview = Some(2 * 1024 * 1024);
        }
        "picker" => open_picker(m, info(vec![]), 2),
        "audio" => open_picker(m, info(vec![]), 5),
        "playlist" => {
            let entries = ["Why async Rust is different", "Futures and the Poll trait", "Pinning without pain", "Building a tiny executor", "Tokio runtime internals", "Channels, select! and cancellation", "Q&A livestream (unedited)"]
                .iter()
                .enumerate()
                .map(|(i, t)| Entry { url: format!("https://www.youtube.com/watch?v={i}"), title: t.to_string(), thumbnail: None, duration: Some(1100.0 + i as f64 * 97.0) })
                .collect();
            open_picker(m, info(entries), 2);
            if let Some(p) = &mut m.picker {
                p.toggle_entry(3);
                p.toggle_entry(6);
            }
        }
        "long" => {
            let mut items = demo_items();
            let long = "An extremely long video title that keeps going and going — part 1 of the complete series about async Rust, tokio, pinning, executors and cancellation (4K remaster, director's cut, extended edition)";
            items.push(Item { id: ItemId(2000), name: long.into(), status: Status::Failed(format!("yt-dlp: ERROR: [youtube] abc: {}", "Sign in to confirm you're not a bot. ".repeat(8))), ..items[3].clone() });
            items.push(Item { id: ItemId(2001), name: long.into(), ..items[4].clone() });
            m.items = items;
            m.selected = Some(ItemId(2000));
        }
        "quit" => {
            m.items = demo_items();
            m.confirm_quit = true;
        }
        "thumbs" => {
            let mut items = demo_items();
            for (item, id) in items.iter_mut().filter(|i| matches!(i.kind, Kind::Media(_))).zip(["aqz-KE-bpKQ", "jNQXAC9IVRw", "dQw4w9WgXcQ"]) {
                item.thumbnail = Some(format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg"));
                item.duration = Some(2538.0);
            }
            m.items = items;
            m.selected = m.items.iter().rev().find(|i| i.status == Status::Running).map(|i| i.id);
        }
        // The inspector's segment map and speed graph, and the footer's sparkline.
        "graph" => {
            m.items = demo_items();
            m.selected = m.items.iter().find(|i| i.status == Status::Running).map(|i| i.id);
            demo_speeds(m);
        }
        "toast" => m.toast = Some("https://vimeo.com/824123456".into()),
        "notice" => m.notice = Some("Added “Rust Async Explained” (1080p)".into()),
        "empty" => m.items.clear(),
        "blue" => m.settings.accent = "#0a84ff".into(),
        "white" => m.settings.accent = "#f5f5f7".into(),
        "orange" => m.settings.accent = "#ff9f0a".into(),
        "help-menu" => m.help_open = true,
        "shortcuts" => m.open_info(Info::Shortcuts),
        "help" => m.open_info(Info::Help),
        // As shown once after an update from a build that didn't record the version yet.
        "whats-new" => m.open_info(Info::WhatsNew { since: Some(String::new()) }),
        "whats-new-all" => m.open_info(Info::WhatsNew { since: None }),
        "bug-report" => m.open_info(Info::BugReport),
        // The colour picker after a drag: purple from the strip, a little muted in the square.
        "accent-picker" => {
            m.open_settings(SettingsTab::Appearance);
            m.drag_accent_hue(265.0);
            m.drag_accent_sv(0.62, 0.9);
        }
        // A grey typed into the box: the strip keeps the purple hue it had.
        "accent-grey" => {
            m.open_settings(SettingsTab::Appearance);
            m.drag_accent_hue(265.0);
            m.type_accent("#8a8a8a".into());
        }
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

/// A minute of a running download's speed (a wavy 6 – 11 MB/s with a dip) and its 8 connections.
fn demo_speeds(m: &mut Model) {
    let Some(id) = m.items.iter().find(|i| i.status == Status::Running).map(|i| i.id) else { return };
    let now = Instant::now();
    m.speeds = crate::speed_history::Speeds::since(now - Duration::from_secs(120));
    let mut items = m.items.clone();
    for k in 0..60u64 {
        let x = k as f64;
        let mb = 8.5 + 1.6 * (x / 4.0).sin() + 0.8 * (x / 1.7).cos() - if (34..40).contains(&k) { 4.0 } else { 0.0 };
        if let Some(i) = items.iter_mut().find(|i| i.id == id) {
            i.speed_bps = (mb * 1024.0 * 1024.0) as u64;
        }
        m.speeds.sample(&items, now - Duration::from_secs(59 - k));
    }
    let total = m.items.iter().find(|i| i.id == id).and_then(|i| i.total).unwrap_or(0);
    let part = total / 8;
    let done = [1.0, 1.0, 0.86, 0.62, 0.55, 0.4, 0.17, 0.08];
    let segments = (0..8u64)
        .map(|k| {
            let end = if k == 7 { total } else { (k + 1) * part };
            let mut s = rdm_engine::segments::Segment::new(k * part, end);
            s.written = ((end - k * part) as f64 * done[k as usize]) as u64;
            s
        })
        .collect();
    m.segments.insert(id, segments);
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
        thumbnail: None,
        live: false,
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
        thumbnail: None,
        duration: None,
        retry_at: None,
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
