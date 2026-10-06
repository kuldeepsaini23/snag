use crate::queues::QueueDraft;
use crate::state::{MediaTab, Model, Screen, SettingsTab, explorer_select_arg};
use crate::view::{Filter, Library};
use iced::widget::{Id, operation};
use iced::{Subscription, Task, keyboard, window};
use rdm_core::{AppState, Event, ItemId, Manager, MediaFormat, MediaInfo, QueueId, Status};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use tokio::sync::broadcast::error::RecvError;

pub struct App {
    pub model: Model,
    pub manager: Manager,
    /// %APPDATA%\rdm: state.json, tools, cookies.
    pub data_dir: PathBuf,
}

/// Text inputs the app focuses itself.
pub fn url_input() -> Id {
    Id::new("url")
}

pub fn search_input() -> Id {
    Id::new("search")
}

/// The limit the popover's switch turns on when there was none.
const DEFAULT_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum Message {
    UrlChanged(String),
    Add,
    /// Toolbar / empty-state "Add URL": submit the link, or focus the empty URL bar.
    AddUrl,
    Added(ItemId),
    Select(ItemId),
    Pause(ItemId),
    Resume(ItemId),
    Redownload(ItemId),
    Remove(ItemId),
    Delete(ItemId),
    ShowInFolder(ItemId),
    Core(Event),
    Loaded(AppState),
    /// The event feed fell behind; reload the full state.
    Lagged,
    // Window
    WinDrag,
    WinMinimize,
    WinMaximize,
    WinClose,
    /// Pressed on a window edge: resize from there (the window has no native frame).
    WinResize(window::Direction),
    // List
    SetFilter(Filter),
    SetLibrary(Library),
    Search(String),
    FocusSearch,
    ToggleSidebar,
    /// Escape: close whatever is on top.
    Escape,
    // Speed popover
    ToggleSpeed,
    SpeedOn(bool),
    SpeedSlide(f32),
    SpeedRelease,
    /// A preset (bytes/s, 0 = none).
    QuickLimit(u64),
    // Settings sheet
    OpenSettings(SettingsTab),
    CloseSettings,
    DraftDir(String),
    DraftConnections(usize),
    DraftMax(usize),
    DraftLimit(String),
    DraftSort(bool),
    DraftStart(bool),
    DraftClipboard(bool),
    DraftQuality(Option<MediaFormat>),
    DraftAsk(bool),
    DraftAccent(String),
    CopyToken,
    NewToken,
    AddQueue,
    DeleteQueue(usize),
    QueueName(usize, String),
    QueueMax(usize, String),
    QueueScheduled(usize, bool),
    QueueStart(usize, String),
    QueueStop(usize, String),
    QueueDay(usize, usize, bool),
    UpdateYtdlp,
    YtdlpUpdated(Result<String, String>),
    OpenDataFolder,
    /// Removes finished items from the list (their files stay).
    ClearFinished,
    MoveToQueue(ItemId, QueueId),
    // Picker
    /// yt-dlp finished reading a video link.
    Probed(String, Result<MediaInfo, String>),
    PickOption(usize),
    PickTab(MediaTab),
    PickEntry(usize),
    PickAll(bool),
    PickQueue(QueueId),
    DownloadPicked,
    CancelPick,
    // Clipboard and toasts
    ClipboardTick,
    ClipboardText(Option<String>),
    ToastDownload,
    ToastClose,
    DismissNotice,
    /// Debug builds: render the window to a file (see `snap.rs`).
    #[cfg(debug_assertions)]
    Snap,
    #[cfg(debug_assertions)]
    Snapped(window::Screenshot),
    Done,
}

pub fn boot(manager: Manager, bridge_status: String, data_dir: PathBuf) -> (App, Task<Message>) {
    let m = manager.clone();
    let model = Model { bridge_status, ..Model::default() };
    (App { model, manager, data_dir }, Task::perform(async move { m.snapshot().await }, Message::Loaded))
}

/// Runs a manager call in the background; its result isn't needed (events report it).
fn fire<Fut>(manager: &Manager, call: impl FnOnce(Manager) -> Fut) -> Task<Message>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    Task::perform(call(manager.clone()), |_| Message::Done)
}

fn on_window<T: Send + 'static>(action: fn(window::Id) -> Task<T>) -> Task<T> {
    window::latest().and_then(action)
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    let model = &mut app.model;
    match message {
        Message::UrlChanged(url) => model.url = url,
        Message::AddUrl => {
            if model.url.trim().is_empty() {
                model.screen = Screen::Downloads;
                return operation::focus(url_input());
            }
            return update(app, Message::Add);
        }
        Message::Add => {
            let url = model.url.trim().to_string();
            if url.is_empty() {
                return Task::none();
            }
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                model.notice = Some("Paste a link that starts with http:// or https://".into());
                return Task::none();
            }
            model.url.clear();
            model.notice = None;
            let m = app.manager.clone();
            if rdm_media::is_media_url(&url) {
                model.probing = true;
                model.notice = Some("Reading video info… (the first time also fetches yt-dlp)".into());
                return Task::perform(async move { let r = m.probe_media(url.clone()).await; (url, r) }, |(url, r)| Message::Probed(url, r));
            }
            return Task::perform(async move { m.add(url).await }, Message::Added);
        }
        Message::Added(id) | Message::Select(id) => model.selected = Some(id),
        Message::Pause(id) => return fire(&app.manager, move |m| async move { m.pause(id).await }),
        Message::Resume(id) => return fire(&app.manager, move |m| async move { m.resume(id).await }),
        Message::Redownload(id) => return fire(&app.manager, move |m| async move { m.redownload(id).await }),
        Message::Remove(id) => return fire(&app.manager, move |m| async move { m.remove(id, false).await }),
        Message::Delete(id) => {
            if model.confirm_delete(id) {
                return fire(&app.manager, move |m| async move { m.remove(id, true).await });
            }
        }
        Message::ShowInFolder(id) => {
            if let Some(path) = model.dest_of(id) {
                reveal(&path);
            }
        }
        Message::Core(event) => {
            let pick = matches!(event, Event::PickMedia { .. } | Event::Focus);
            model.apply(event);
            if pick {
                // The extension sent a video: bring the window forward for the quality choice.
                return window::latest().and_then(window::gain_focus);
            }
        }
        Message::Loaded(state) => model.load(state),
        Message::Lagged => {
            let m = app.manager.clone();
            return Task::perform(async move { m.snapshot().await }, Message::Loaded);
        }
        Message::WinDrag => return on_window(window::drag),
        Message::WinMinimize => return window::latest().and_then(|id| window::minimize(id, true)),
        Message::WinMaximize => return on_window(window::toggle_maximize),
        Message::WinClose => return on_window(window::close),
        Message::WinResize(edge) => return window::latest().and_then(move |id| window::drag_resize(id, edge)),
        Message::SetFilter(f) => model.filter = f,
        Message::SetLibrary(l) => model.library = l,
        Message::Search(s) => model.search = s,
        Message::FocusSearch => return operation::focus(search_input()),
        Message::ToggleSidebar => model.sidebar_open = !model.sidebar_open,
        Message::Escape => {
            if model.speed_open {
                model.speed_open = false;
            } else if model.screen == Screen::Settings {
                return update(app, Message::CloseSettings);
            } else if model.screen == Screen::Picker {
                return update(app, Message::CancelPick);
            } else {
                model.toast = None;
            }
        }
        Message::ToggleSpeed => {
            model.speed_open = !model.speed_open;
            model.speed_preview = None;
        }
        Message::SpeedOn(on) => {
            let bps = if on { model.speed_preview.unwrap_or(DEFAULT_LIMIT) } else { 0 };
            return set_limit(app, bps);
        }
        Message::SpeedSlide(v) => model.preview_speed(v),
        Message::SpeedRelease => {
            if let Some(bps) = model.release_speed() {
                return set_limit(app, bps);
            }
        }
        Message::QuickLimit(bps) => {
            model.speed_preview = None;
            return set_limit(app, bps);
        }
        Message::OpenSettings(tab) => model.open_settings(tab),
        Message::CloseSettings => {
            if let Ok((settings, queues)) = model.close_settings() {
                let (old_settings, old_queues) = (model.settings.clone(), model.queues.clone());
                return fire(&app.manager, move |m| async move {
                    if settings != old_settings {
                        m.update_settings(settings).await;
                    }
                    if queues != old_queues {
                        m.set_queues(queues).await;
                    }
                });
            }
        }
        Message::DraftDir(v) => model.draft.download_dir = v,
        Message::DraftConnections(n) => model.draft.connections = n.to_string(),
        Message::DraftMax(n) => model.draft.max_concurrent = n.to_string(),
        Message::DraftLimit(v) => model.draft.speed_limit_kbps = v,
        Message::DraftSort(v) => model.draft.sort_into_folders = v,
        Message::DraftStart(v) => model.draft.start_immediately = v,
        Message::DraftClipboard(v) => model.draft.clipboard_watch = v,
        Message::DraftQuality(q) => model.draft.preferred_quality = q,
        Message::DraftAsk(v) => model.draft.ask_quality = v,
        Message::DraftAccent(v) => model.draft.accent = v,
        Message::CopyToken => {
            model.notice = Some("Pairing code copied".into());
            return iced::clipboard::write(model.settings.extension_token.clone());
        }
        Message::NewToken => {
            let settings = rdm_core::Settings { extension_token: rdm_core::new_token(), ..model.settings.clone() };
            return fire(&app.manager, move |m| async move { m.update_settings(settings).await });
        }
        Message::AddQueue => {
            let fresh = QueueDraft::new(&model.queue_drafts, &model.queues);
            model.queue_drafts.push(fresh);
        }
        Message::DeleteQueue(i) => {
            if model.queue_drafts.get(i).is_some_and(|d| d.id != 0) {
                model.queue_drafts.remove(i);
            }
        }
        Message::QueueName(i, v) => edit_queue(model, i, |d| d.name = v),
        Message::QueueMax(i, v) => edit_queue(model, i, |d| d.max_concurrent = v),
        Message::QueueScheduled(i, v) => edit_queue(model, i, |d| d.scheduled = v),
        Message::QueueStart(i, v) => edit_queue(model, i, |d| d.start = v),
        Message::QueueStop(i, v) => edit_queue(model, i, |d| d.stop = v),
        Message::QueueDay(i, day, v) => edit_queue(model, i, |d| d.days[day % 7] = v),
        Message::UpdateYtdlp => {
            model.updating_ytdlp = true;
            model.notice = Some("Updating yt-dlp…".into());
            let m = app.manager.clone();
            return Task::perform(async move { m.update_ytdlp().await }, Message::YtdlpUpdated);
        }
        Message::YtdlpUpdated(result) => {
            model.updating_ytdlp = false;
            model.notice = Some(match result {
                Ok(said) => said,
                Err(e) => format!("yt-dlp update failed: {e}"),
            });
        }
        Message::OpenDataFolder => {
            let _ = std::process::Command::new("explorer").arg(&app.data_dir).spawn();
        }
        Message::ClearFinished => {
            let done: Vec<ItemId> = model.items.iter().filter(|i| i.status == Status::Done).map(|i| i.id).collect();
            return fire(&app.manager, move |m| async move {
                for id in done {
                    m.remove(id, false).await;
                }
            });
        }
        Message::MoveToQueue(id, queue) => return fire(&app.manager, move |m| async move { m.move_to_queue(id, queue).await }),
        Message::Probed(url, result) => {
            model.probing = false;
            match result {
                Ok(info) if !info.options.is_empty() => {
                    model.notice = None;
                    // Same path as links from the browser: picker, or straight in with the preferred quality.
                    return fire(&app.manager, move |m| async move { m.offer_media(url, info).await });
                }
                Ok(_) => model.notice = Some("No downloadable video or audio found on that page.".into()),
                Err(e) => model.notice = Some(format!("Couldn't read that link: {e}")),
            }
        }
        Message::PickOption(i) => edit_picker(model, |p| p.choice = i),
        Message::PickTab(tab) => edit_picker(model, |p| p.set_tab(tab)),
        Message::PickEntry(i) => edit_picker(model, |p| p.toggle_entry(i)),
        Message::PickAll(on) => edit_picker(model, |p| p.select_all(on)),
        Message::PickQueue(q) => edit_picker(model, |p| p.queue = q),
        Message::DownloadPicked => {
            let Some(picker) = model.picker.take() else { return Task::none() };
            let (requests, queue) = (picker.requests(), picker.queue);
            model.screen = Screen::Downloads;
            return fire(&app.manager, move |m| async move {
                for (url, title, format) in requests {
                    m.add_media_to(url, title, format, queue).await;
                }
            });
        }
        Message::CancelPick => {
            model.picker = None;
            model.screen = Screen::Downloads;
        }
        Message::ClipboardTick => return iced::clipboard::read().map(Message::ClipboardText),
        Message::ClipboardText(text) => model.clipboard_seen(text),
        Message::ToastDownload => {
            if let Some(link) = model.toast.take() {
                model.url = link;
                return update(app, Message::Add);
            }
        }
        Message::ToastClose => model.toast = None,
        Message::DismissNotice => model.notice = None,
        #[cfg(debug_assertions)]
        Message::Snap => return crate::snap::take(model),
        #[cfg(debug_assertions)]
        Message::Snapped(shot) => crate::snap::save(&shot),
        Message::Done => {}
    }
    Task::none()
}

fn set_limit(app: &App, bps: u64) -> Task<Message> {
    let settings = rdm_core::Settings { speed_limit_bps: bps, ..app.model.settings.clone() };
    fire(&app.manager, move |m| async move { m.update_settings(settings).await })
}

fn edit_queue(model: &mut Model, i: usize, change: impl FnOnce(&mut QueueDraft)) {
    if let Some(d) = model.queue_drafts.get_mut(i) {
        change(d);
    }
}

fn edit_picker(model: &mut Model, change: impl FnOnce(&mut crate::state::Picker)) {
    if let Some(p) = &mut model.picker {
        change(p);
    }
}

/// Opens Explorer with the file selected; if the file isn't there, opens its folder.
fn reveal(path: &std::path::Path) {
    let mut cmd = std::process::Command::new("explorer");
    if path.exists() {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.raw_arg(explorer_select_arg(path));
        }
    } else if let Some(dir) = path.parent().filter(|d| d.exists()) {
        cmd.arg(dir);
    } else {
        return;
    }
    let _ = cmd.spawn();
}

/// Identity for the core event subscription (there is only ever one feed).
struct Feed(Manager);

impl Hash for Feed {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "rdm-core-events".hash(state);
    }
}

pub fn subscription(app: &App) -> Subscription<Message> {
    let mut subs = vec![core_events(app), keyboard::listen().filter_map(on_key)];
    #[cfg(debug_assertions)]
    subs.push(crate::snap::subscription());
    if app.model.settings.clipboard_watch {
        subs.push(iced::time::every(std::time::Duration::from_millis(700)).map(|_| Message::ClipboardTick));
    }
    Subscription::batch(subs)
}

/// Ctrl+K focuses search; Escape closes whatever is on top.
fn on_key(event: keyboard::Event) -> Option<Message> {
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else { return None };
    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Escape),
        keyboard::Key::Character("k") if modifiers.command() => Some(Message::FocusSearch),
        _ => None,
    }
}

fn core_events(app: &App) -> Subscription<Message> {
    Subscription::run_with(Feed(app.manager.clone()), |feed| {
        futures_util::stream::unfold(feed.0.subscribe(), |mut rx| async move {
            let message = match rx.recv().await {
                Ok(event) => Message::Core(event),
                Err(RecvError::Lagged(_)) => Message::Lagged,
                Err(RecvError::Closed) => return None,
            };
            Some((message, rx))
        })
    })
}
