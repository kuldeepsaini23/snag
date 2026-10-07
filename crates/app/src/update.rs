use crate::motion::Motion;
use crate::queues::QueueDraft;
use crate::state::{Info, MediaTab, Model, Screen, SettingsTab, explorer_select_arg};
use crate::view::{Filter, Library};
use iced::widget::{Id, operation, text_editor};
use iced::{Subscription, Task, keyboard, window};
use rdm_core::{AppState, Event, ItemId, Manager, MediaFormat, MediaInfo, QueueId, Status, ThemeMode};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::broadcast::error::RecvError;

pub struct App {
    pub model: Model,
    pub manager: Manager,
    /// %APPDATA%\rdm: state.json, tools, cookies.
    pub data_dir: PathBuf,
    /// Held only to keep the tray icon alive (None if Windows refused it).
    #[allow(dead_code)]
    pub tray: Option<crate::tray::Tray>,
    /// The download manager's runtime (servers started from the UI run there).
    pub runtime: tokio::runtime::Handle,
    /// Phone sharing while it's switched on: the server, its link and QR code.
    pub phone: Option<(rdm_bridge::phone::Phone, String, iced::widget::qr_code::Data)>,
    pub motion: Motion,
    /// The instant the view draws (animations are read at it).
    pub now: Instant,
    /// The item the inspector shows, kept while it slides closed.
    pub inspector_item: Option<ItemId>,
    /// Report a bug: "What happened?" as typed (kept if the sheet is closed before saving).
    pub bug_text: text_editor::Content,
    /// The backdrop last asked of the window: (translucent, light).
    pub backdrop_asked: Option<(bool, bool)>,
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
    /// Adds this link (from the URL bar or the clipboard toast).
    AddLink(String),
    /// Toolbar / empty-state "Add URL": submit the link, or focus the empty URL bar.
    AddUrl,
    Added(ItemId),
    Select(ItemId),
    Pause(ItemId),
    Resume(ItemId),
    Redownload(ItemId),
    Remove(ItemId),
    Delete(ItemId),
    /// Stop an unfinished download and throw away what it downloaded (second click confirms).
    Cancel(ItemId),
    /// The mouse entered (true) or left a row.
    HoverRow(ItemId, bool),
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
    /// The window changed size: check whether it is maximised now.
    WinResized,
    WinMaximized(bool),
    /// Close button / Alt+F4: hide to the tray, keep downloading.
    HideWindow,
    TrayOpen,
    TrayQuit,
    QuitAnyway,
    KeepDownloading,
    PauseAll,
    ResumeAll,
    /// Show the notifications collected since the last flush.
    FlushNotes,
    DraftNotify(bool),
    DraftSubtitles(bool),
    DraftAutoRetry(bool),
    DraftKeepSharing(bool),
    /// Puts text on the clipboard (errors, links).
    CopyText(String),
    /// The answer to "a browser extension wants to connect".
    AnswerPair(bool),
    /// The answer to "Already downloaded": show the file, download it again, or skip.
    Duplicate(DuplicateChoice),
    /// Phone sharing on/off (applies at once, like a switch should).
    PhoneSharing(bool),
    /// The picker's playlist/channel: download its new uploads from now on.
    WatchPicked,
    RemoveWatch(u32),
    DraftSubtitleLangs(String),
    DraftVirusTotalKey(String),
    /// The after-download rule editor.
    RuleWhen(crate::rules_form::When),
    RuleValue(String),
    RuleCategory(crate::rules_form::CategoryChoice),
    RuleAction(crate::rules_form::Do),
    RuleFolder(String),
    RuleKeep(bool),
    RuleAdd,
    RuleToggle(u32),
    RuleDelete(u32),
    RefreshTyped(ItemId, String),
    /// Use the typed link for this item.
    RefreshUrl(ItemId),
    /// A thumbnail finished downloading (None: it couldn't be fetched).
    ThumbReady(String, Option<PathBuf>),
    /// A failed thumbnail may be due again (every update fetches the ones due).
    RetryThumbs,
    // List
    SetFilter(Filter),
    SetLibrary(Library),
    /// The sidebar's "Stats" (the day counter is reloaded with it).
    OpenStats,
    StatsRange(crate::stats::Range),
    /// While the stats screen is open and something downloads: fetch the day counter again.
    StatsTick,
    DailyLoaded(std::collections::BTreeMap<String, rdm_core::DayTotal>),
    /// Videos or Images: show the grid of thumbnails (true) or the list.
    SetGrid(rdm_core::Category, bool),
    Search(String),
    /// The magnifier or Ctrl+K: open the search field and focus it.
    FocusSearch,
    /// A click landed somewhere: does the search field still have focus?
    CheckSearchFocus,
    SearchFocused(bool),
    ToggleSidebar,
    /// The sidebar's "More" group.
    ToggleMore,
    /// A filter tab was laid out this wide.
    TabMeasured(usize, f32),
    /// A frame while something animates.
    Frame,
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
    /// The sheet's nav: another tab, keeping edits.
    SettingsTab(SettingsTab),
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
    DraftTheme(ThemeMode),
    DraftTranslucent(bool),
    /// Windows' app mode now (true = light), while the theme follows it.
    SystemTheme(bool),
    /// Mica (or acrylic) is behind the window now (true), or isn't.
    Backdrop(bool),
    // First-run tour
    /// Settings → General → Show the tour again.
    ShowTour,
    TourNext,
    TourBack,
    SkipTour,
    /// The tour's last step: a test link into the link bar.
    TryLink,
    // Drag and drop
    /// Files are dragged over the window (true), or went away without a drop.
    DropHover(bool),
    FileDropped(PathBuf),
    /// The accent picker: saturation and value (0 … 1) from the square, hue (degrees) from the strip.
    AccentSv(f32, f32),
    AccentHue(f32),
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
    // Help
    /// The toolbar's "?" menu.
    ToggleHelp,
    OpenInfo(Info),
    CloseInfo,
    BugEdit(text_editor::Action),
    BugDiagnostics(bool),
    SaveBugReport,
    /// The report's file and text, or why it couldn't be written.
    BugReportSaved(Result<(PathBuf, String), String>),
    DismissNotice,
    /// Debug builds: render the window to a file (see `snap.rs`).
    #[cfg(debug_assertions)]
    Snap,
    #[cfg(debug_assertions)]
    Snapped(window::Screenshot),
    Done,
}

pub fn boot(manager: Manager, bridge_status: String, data_dir: PathBuf, runtime: tokio::runtime::Handle) -> (App, Task<Message>) {
    let m = manager.clone();
    let model = Model { bridge_status, system_light: crate::appearance::system_light(), ..Model::default() };
    let tray = crate::tray::create();
    crate::notify::register(&data_dir);
    let motion = Motion::new(crate::view::motion_targets(&model), crate::motion::system_reduced_motion());
    let app = App {
        model,
        manager,
        data_dir,
        tray,
        runtime,
        phone: None,
        motion,
        now: Instant::now(),
        inspector_item: None,
        bug_text: text_editor::Content::new(),
        backdrop_asked: None,
    };
    (app, Task::perform(async move { m.snapshot().await }, Message::Loaded))
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
    // A full reload puts rows in place; only rows added one by one fade in.
    let animate = !matches!(message, Message::Loaded(_));
    let task = handle(app, message);
    sync_phone(app);
    // Items and the picker may now show videos whose thumbnails aren't here yet.
    let thumbs = fetch_thumbs(app);
    let backdrop = sync_backdrop(app);
    sync_motion(app, animate);
    Task::batch([task, thumbs, backdrop])
}

/// Mica follows the "Translucent window" setting, tinted for the theme drawn (nothing is asked
/// of the window until it's first switched on).
fn sync_backdrop(app: &mut App) -> Task<Message> {
    let wanted = (app.model.settings.translucent, app.model.light());
    if app.backdrop_asked == Some(wanted) || (app.backdrop_asked.is_none() && !wanted.0) {
        return Task::none();
    }
    app.backdrop_asked = Some(wanted);
    let (on, light) = wanted;
    window::latest().and_then(move |id| window::run(id, move |w| crate::appearance::set_backdrop(w, on, light))).map(Message::Backdrop)
}

/// Saves settings changed outside the settings sheet (the tour, the launch checks).
fn save_settings(app: &App, settings: rdm_core::Settings) -> Task<Message> {
    fire(&app.manager, move |m| async move { m.update_settings(settings).await })
}

/// Points the animations at what the model shows now; the view reads them at `app.now`.
fn sync_motion(app: &mut App, animate: bool) {
    let now = Instant::now();
    app.motion.sync(crate::view::motion_targets(&app.model), now);
    app.motion.sync_rows(&crate::view::row_targets(&app.model), now, animate);
    if let Some(item) = app.model.inspected() {
        app.inspector_item = Some(item.id);
    }
    app.now = now;
    #[cfg(debug_assertions)]
    if let Some(frozen) = crate::snap::frozen() {
        app.now = frozen;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateChoice {
    Show,
    Again,
    Skip,
}

/// Errors and notices go to `<data>/snag.log` (kept under 1 MB): text in the window can't be
/// copied, and a bug report needs them.
fn log_event(data_dir: &std::path::Path, event: &Event) {
    let line = match event {
        Event::Notice(text) => text.clone(),
        Event::Updated(item) => match &item.status {
            rdm_core::Status::Failed(e) => format!("{} failed: {e}", item.name),
            _ => return,
        },
        _ => return,
    };
    let path = data_dir.join("snag.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024) {
        let _ = std::fs::rename(&path, data_dir.join("snag.old.log"));
    }
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{stamp}  {line}");
    }
}

/// Phone sharing follows its setting (and a regenerated pairing code).
fn sync_phone(app: &mut App) {
    let s = &app.model.settings;
    if !s.phone_sharing {
        app.phone = None;
        return;
    }
    let token = s.extension_token.clone();
    if let Some((phone, link, _)) = &app.phone
        && link.ends_with(&token)
    {
        let _ = phone.port;
        return;
    }
    app.phone = None;
    let Some(ip) = rdm_bridge::phone::lan_ip() else {
        app.model.notice = Some("Phone sharing: no home network found".into());
        return;
    };
    for port in rdm_bridge::phone::PORTS {
        if let Ok(phone) = rdm_bridge::phone::start_on(&app.runtime, app.manager.clone(), (std::net::Ipv4Addr::UNSPECIFIED, port).into()) {
            let link = rdm_bridge::phone::page_url(ip, phone.port, &token);
            if let Ok(qr) = iced::widget::qr_code::Data::new(&link) {
                app.phone = Some((phone, link, qr));
            }
            return;
        }
    }
    app.model.notice = Some("Phone sharing: no free port (47330–47335)".into());
}

fn fetch_thumbs(app: &mut App) -> Task<Message> {
    let wanted = app.model.missing_thumbs(Instant::now());
    if wanted.is_empty() {
        return Task::none();
    }
    let dir = app.data_dir.join("thumbs");
    Task::batch(wanted.into_iter().map(|url| {
        app.model.thumb_pending.insert(url.clone());
        let file = crate::view::thumb_file(&dir, &url);
        Task::perform(fetch_thumb(url.clone(), file), move |path| Message::ThumbReady(url.clone(), path))
    }))
}

/// Downloads a thumbnail once (it stays cached on disk across restarts).
async fn fetch_thumb(url: String, base: PathBuf) -> Option<PathBuf> {
    if let Some(cached) = ["jpg", "png", "webp"].iter().map(|e| base.with_extension(e)).find(|f| f.exists()) {
        return Some(cached);
    }
    if let Some(src) = crate::view::local_thumb_source(&url).map(PathBuf::from) {
        // A downloaded picture: decoded on a worker, two at a time (a library of photos would
        // otherwise decode them all at once).
        static DECODING: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
        let _turn = DECODING.acquire().await.ok()?;
        return tokio::task::spawn_blocking(move || crate::local_thumb::make(&src, &base)).await.ok()?;
    }
    let get = rdm_engine::default_client().get(&url).timeout(std::time::Duration::from_secs(20)).send();
    let bytes = get.await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
    // A thumbnail is small; anything huge isn't one.
    if bytes.len() > 5 * 1024 * 1024 {
        return None;
    }
    let file = base.with_extension(crate::view::image_ext(&bytes)?);
    tokio::fs::create_dir_all(file.parent()?).await.ok()?;
    tokio::fs::write(&file, &bytes).await.ok()?;
    Some(file)
}

fn handle(app: &mut App, message: Message) -> Task<Message> {
    let model = &mut app.model;
    match message {
        Message::UrlChanged(url) => model.url = url,
        Message::AddUrl => {
            if model.url.trim().is_empty() {
                model.screen = Screen::Downloads;
                model.close_stats();
                return operation::focus(url_input());
            }
            return update(app, Message::Add);
        }
        Message::Add => {
            let url = model.url.trim().to_string();
            if url.is_empty() {
                return Task::none();
            }
            if url.starts_with("http://") || url.starts_with("https://") || rdm_core::is_torrent_link(&url) {
                model.url.clear();
            }
            return update(app, Message::AddLink(url));
        }
        Message::AddLink(url) => {
            // A GitHub repository's page: its code as a ZIP.
            let url = rdm_core::route::github_zip(&url).unwrap_or(url);
            let torrent = rdm_core::is_torrent_link(&url);
            if !(url.starts_with("http://") || url.starts_with("https://") || torrent) {
                model.notice = Some("Paste a web link (http:// or https://) or a magnet link".into());
                return Task::none();
            }
            model.notice = None;
            let m = app.manager.clone();
            match rdm_core::route::route(&url) {
                rdm_core::route::Route::Torrent => return Task::perform(async move { m.add_torrent(url).await }, Message::Added),
                rdm_core::route::Route::Gallery => return Task::perform(async move { m.add_gallery(url).await }, Message::Added),
                rdm_core::route::Route::Media => {
                    // yt-dlp reads it (it knows 1,800+ sites; for any other page it looks for a video).
                    model.probing = true;
                    model.notice = Some("Reading the page…".into());
                    return Task::perform(async move { let r = m.probe_media(url.clone()).await; (url, r) }, |(url, r)| Message::Probed(url, r));
                }
                rdm_core::route::Route::File => {}
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
        Message::Cancel(id) => {
            if model.confirm_cancel(id) {
                // Removing an unfinished item deletes its partial data (never a finished file).
                return fire(&app.manager, move |m| async move { m.remove(id, false).await });
            }
        }
        Message::HoverRow(id, inside) => {
            if inside {
                model.hovered = Some(id);
            } else if model.hovered == Some(id) {
                model.hovered = None;
            }
        }
        Message::ShowInFolder(id) => {
            if let Some(path) = model.dest_of(id) {
                reveal(&path);
            }
        }
        Message::Core(Event::Quit) => return iced::exit(),
        Message::Core(event) => {
            log_event(&app.data_dir, &event);
            let pick = matches!(event, Event::PickMedia { .. } | Event::Focus | Event::PairRequest { .. } | Event::Duplicate(_));
            model.apply(event);
            model.speeds.sample(&model.items, Instant::now());
            if pick {
                // The extension sent a video, or RDM was started again: bring the (maybe hidden) window forward.
                return show_window();
            }
        }
        Message::Loaded(state) => {
            model.load(state);
            // A fresh install starts the tour (decided before the version is noted: a version
            // seen means Snag ran before). After an update, "What's new" shows once.
            let tour = model.check_tour();
            if model.check_version(crate::changelog::VERSION).is_some() || tour.is_some() {
                let settings = model.settings.clone();
                return save_settings(app, settings);
            }
        }
        Message::Lagged => {
            let m = app.manager.clone();
            return Task::perform(async move { m.snapshot().await }, Message::Loaded);
        }
        Message::WinDrag => return on_window(window::drag),
        Message::WinMinimize => return window::latest().and_then(|id| window::minimize(id, true)),
        Message::WinMaximize => return on_window(window::toggle_maximize),
        Message::WinResized => return window::latest().and_then(window::is_maximized).map(Message::WinMaximized),
        Message::WinMaximized(on) => model.maximized = on,
        Message::WinClose | Message::HideWindow => return window::latest().and_then(|id| window::set_mode(id, window::Mode::Hidden)),
        Message::TrayOpen => return show_window(),
        Message::TrayQuit => {
            if model.request_quit() {
                return iced::exit();
            }
            // Something is downloading: show the window with the "still downloading" question.
            return show_window();
        }
        Message::QuitAnyway => return iced::exit(),
        Message::KeepDownloading => model.keep_downloading(),
        Message::FlushNotes => {
            let notes = model.take_notes_at(std::time::Instant::now());
            if !notes.is_empty() {
                return Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || notes.iter().for_each(crate::notify::show)).await;
                    },
                    |_| Message::Done,
                );
            }
        }
        Message::DraftNotify(v) => model.draft.notify = v,
        Message::DraftSubtitles(v) => model.draft.subtitles = v,
        Message::DraftAutoRetry(v) => model.draft.auto_retry = v,
        Message::DraftKeepSharing(v) => model.draft.keep_sharing = v,
        Message::CopyText(text) => {
            model.notice = Some("Copied ✓".into());
            return iced::clipboard::write(text);
        }
        Message::Duplicate(choice) => {
            let Some(item) = model.duplicate.take() else { return Task::none() };
            return match choice {
                DuplicateChoice::Show => update(app, Message::ShowInFolder(item.id)),
                DuplicateChoice::Again => update(app, Message::Redownload(item.id)),
                DuplicateChoice::Skip => Task::none(),
            };
        }
        Message::AnswerPair(allow) => {
            if let Some((id, _)) = model.pair_request.take() {
                if allow {
                    model.notice = Some("Browser extension connected ✓".into());
                }
                return fire(&app.manager, move |m| async move { m.answer_pair(id, allow).await });
            }
        }
        Message::PhoneSharing(on) => {
            model.draft.phone_sharing = on;
            let settings = rdm_core::Settings { phone_sharing: on, ..model.settings.clone() };
            model.settings.phone_sharing = on;
            return fire(&app.manager, move |m| async move { m.update_settings(settings).await });
        }
        Message::WatchPicked => {
            let Some(p) = model.picker.take() else { return Task::none() };
            model.screen = Screen::Downloads;
            let format = p.info.options.get(p.choice).map(|o| o.format.clone()).unwrap_or(MediaFormat::Video { max_height: 1080 });
            model.notice = Some(format!("Watching “{}”: new uploads download by themselves", p.info.title));
            let (url, name) = (p.url, p.info.title);
            return fire(&app.manager, move |m| async move { m.add_watch(url, name, format, None, 6).await });
        }
        Message::RemoveWatch(id) => return fire(&app.manager, move |m| async move { m.remove_watch(id).await }),
        Message::DraftSubtitleLangs(v) => model.draft.subtitle_langs = v,
        Message::DraftVirusTotalKey(v) => model.draft.virustotal_key = v,
        Message::RuleWhen(v) => model.rule_form.when = v,
        Message::RuleValue(v) => model.rule_form.value = v,
        Message::RuleCategory(v) => model.rule_form.category = v,
        Message::RuleAction(v) => model.rule_form.action = v,
        Message::RuleFolder(v) => model.rule_form.folder = v,
        Message::RuleKeep(v) => model.rule_form.keep_original = v,
        Message::RuleAdd => match model.rule_form.build(crate::rules_form::next_id(&model.draft.rules)) {
            Ok(rule) => {
                model.draft.rules.push(rule);
                model.rule_form = Default::default();
                model.rule_error = None;
            }
            Err(why) => model.rule_error = Some(why),
        },
        Message::RuleToggle(id) => {
            if let Some(rule) = model.draft.rules.iter_mut().find(|r| r.id == id) {
                rule.enabled = !rule.enabled;
            }
        }
        Message::RuleDelete(id) => model.draft.rules.retain(|r| r.id != id),
        Message::RefreshTyped(id, text) => model.refresh_typed(id, text),
        Message::RefreshUrl(id) => {
            if let Some(url) = model.take_refresh(id) {
                model.notice = None;
                return fire(&app.manager, move |m| async move { m.refresh_url(id, url).await });
            }
        }
        // A failure is tried again later, a few times (see `view::thumb_backoff`).
        Message::ThumbReady(url, path) => model.thumb_ready(url, path, Instant::now()),
        Message::RetryThumbs => {}
        Message::PauseAll => {
            let ids = model.pause_all_ids();
            return fire(&app.manager, move |m| async move {
                for id in ids {
                    m.pause(id).await;
                }
            });
        }
        Message::ResumeAll => {
            let ids = model.resume_all_ids();
            return fire(&app.manager, move |m| async move {
                for id in ids {
                    m.resume(id).await;
                }
            });
        }
        Message::WinResize(edge) => return window::latest().and_then(move |id| window::drag_resize(id, edge)),
        Message::SetFilter(f) => {
            model.filter = f;
            model.close_stats();
        }
        Message::SetLibrary(l) => {
            model.library = l;
            model.close_stats();
        }
        Message::OpenStats => {
            model.open_stats();
            return update(app, Message::StatsTick);
        }
        Message::StatsRange(range) => model.stats_range = range,
        Message::StatsTick => {
            let m = app.manager.clone();
            return Task::perform(async move { m.snapshot().await.daily }, Message::DailyLoaded);
        }
        Message::DailyLoaded(daily) => model.daily = daily,
        Message::SetGrid(library, on) => {
            if on {
                model.grid.insert(library);
            } else {
                model.grid.remove(&library);
            }
        }
        Message::Search(s) => model.search = s,
        Message::FocusSearch => {
            model.open_search();
            return operation::focus(search_input());
        }
        Message::CheckSearchFocus => return operation::is_focused(search_input()).map(Message::SearchFocused),
        Message::SearchFocused(focused) => model.search_focus(focused),
        Message::ToggleSidebar => model.sidebar_open = !model.sidebar_open,
        Message::ToggleMore => model.more_open = !model.more_open,
        Message::TabMeasured(i, width) => {
            if model.tab_widths.len() <= i {
                model.tab_widths.resize(i + 1, 0.0);
            }
            model.tab_widths[i] = width;
        }
        Message::Frame => {}
        Message::Escape => {
            if model.pair_request.is_some() {
                return update(app, Message::AnswerPair(false));
            } else if model.confirm_quit {
                model.keep_downloading();
            } else if model.info.is_some() {
                model.info = None;
            } else if model.tour.is_some() {
                return update(app, Message::SkipTour);
            } else if model.help_open {
                model.help_open = false;
            } else if model.speed_open {
                if let Some(bps) = model.close_speed() {
                    return set_limit(app, bps);
                }
            } else if model.screen == Screen::Settings {
                return update(app, Message::CloseSettings);
            } else if model.screen == Screen::Picker {
                return update(app, Message::CancelPick);
            } else if model.search_open && model.search.trim().is_empty() {
                model.search_open = false;
            } else if model.stats_open {
                model.close_stats();
            } else {
                model.escape_downloads();
            }
        }
        Message::ToggleSpeed => {
            if model.speed_open {
                if let Some(bps) = model.close_speed() {
                    return set_limit(app, bps);
                }
            } else {
                model.speed_open = true;
                model.speed_preview = None;
            }
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
        Message::SettingsTab(tab) => model.switch_settings_tab(tab),
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
        Message::DraftAccent(v) => model.type_accent(v),
        Message::DraftTheme(mode) => model.draft.theme = mode,
        Message::DraftTranslucent(on) => model.draft.translucent = on,
        Message::SystemTheme(light) => model.system_light = light,
        Message::Backdrop(on) => model.backdrop = on,
        Message::ShowTour => {
            // Closing the sheet saves it first; it stays open if something in it is invalid.
            let close = update(app, Message::CloseSettings);
            if app.model.screen != Screen::Settings {
                app.model.start_tour();
            }
            return close;
        }
        Message::TourNext => {
            if let Some(settings) = model.tour_next() {
                return save_settings(app, settings);
            }
        }
        Message::TourBack => model.tour_back(),
        Message::SkipTour => {
            let settings = model.skip_tour();
            return save_settings(app, settings);
        }
        Message::TryLink => {
            if let Some(settings) = model.try_link() {
                return save_settings(app, settings);
            }
        }
        Message::DropHover(on) => model.drop_hover = on,
        Message::FileDropped(path) => {
            model.drop_hover = false;
            match crate::dropped::read(&path) {
                Ok(links) => {
                    let n = links.len();
                    let tasks: Vec<Task<Message>> = links.into_iter().map(|link| update(app, Message::AddLink(link))).collect();
                    if n > 1 {
                        app.model.notice = Some(format!("Adding {n} links from {}", path.file_name().unwrap_or_default().to_string_lossy()));
                    }
                    return Task::batch(tasks);
                }
                Err(e) => model.notice = Some(e),
            }
        }
        Message::AccentSv(s, v) => model.drag_accent_sv(s, v),
        Message::AccentHue(h) => model.drag_accent_hue(h),
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
                // A photo post: save its images instead.
                Err(e) if rdm_media::gallery::is_photo_post(&url, &e) => {
                    model.notice = None;
                    let m = app.manager.clone();
                    return Task::perform(async move { m.add_gallery(url).await }, Message::Added);
                }
                // Not a site yt-dlp knows and no video on the page: it may be a plain download link.
                Err(e) if !rdm_media::is_media_url(&url) && e.contains("Unsupported URL") => {
                    model.notice = None;
                    let m = app.manager.clone();
                    return Task::perform(async move { m.add(url).await }, Message::Added);
                }
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
            let (requests, queue) = (picker.requests_with_meta(), picker.queue);
            model.screen = Screen::Downloads;
            return fire(&app.manager, move |m| async move {
                for ((url, title, format), (thumbnail, duration)) in requests {
                    m.add_media_meta(url, title, format, queue, thumbnail, duration).await;
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
            // Straight from the toast: whatever is typed in the URL bar stays.
            if let Some(link) = model.toast.take() {
                return update(app, Message::AddLink(link));
            }
        }
        Message::ToastClose => model.toast = None,
        Message::ToggleHelp => model.toggle_help(),
        Message::OpenInfo(info) => model.open_info(info),
        Message::CloseInfo => model.info = None,
        Message::BugEdit(action) => app.bug_text.perform(action),
        Message::BugDiagnostics(on) => model.bug_diagnostics = on,
        Message::SaveBugReport => {
            if model.bug_saving {
                return Task::none();
            }
            let Some(desktop) = crate::report::desktop() else {
                model.notice = Some("Couldn't find your Desktop folder".into());
                return Task::none();
            };
            model.bug_saving = true;
            let (what, include, settings, items) = (app.bug_text.text(), model.bug_diagnostics, model.settings.clone(), model.items.clone());
            return Task::perform(crate::report::save(what, include, settings, items, app.data_dir.clone(), desktop), Message::BugReportSaved);
        }
        Message::BugReportSaved(result) => {
            model.bug_saving = false;
            match result {
                Ok((file, text)) => {
                    model.info = None;
                    model.notice = Some("Bug report saved on your Desktop and copied: paste it wherever you report the bug".into());
                    app.bug_text = text_editor::Content::new();
                    reveal(&file);
                    return iced::clipboard::write(text);
                }
                Err(e) => model.notice = Some(e),
            }
        }
        Message::DismissNotice => model.notice = None,
        #[cfg(debug_assertions)]
        Message::Snap => return crate::snap::take(app),
        #[cfg(debug_assertions)]
        Message::Snapped(shot) => crate::snap::save(&shot),
        Message::Done => {}
    }
    Task::none()
}

/// Un-hides, restores and focuses the window.
fn show_window() -> Task<Message> {
    window::latest().and_then(|id| Task::batch([window::set_mode(id, window::Mode::Windowed), window::minimize(id, false), window::gain_focus(id)]))
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
    let mut subs = vec![
        core_events(app),
        keyboard::listen().filter_map(on_key),
        window::resize_events().map(|_| Message::WinResized),
        window::close_requests().map(|_| Message::HideWindow),
        crate::tray::subscription(),
    ];
    // Notifications go out in batches, so a finished playlist is one toast.
    if !app.model.notes.is_empty() {
        subs.push(iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::FlushNotes));
    }
    // A thumbnail that failed waits to be tried again; something has to wake the window up for it.
    if app.model.next_thumb_retry().is_some() {
        subs.push(iced::time::every(std::time::Duration::from_secs(5)).map(|_| Message::RetryThumbs));
    }
    // Frames only while something moves: an idle window draws nothing.
    if app.motion.animating(Instant::now()) {
        subs.push(window::frames().map(|_| Message::Frame));
    }
    // The stats screen follows the day counter while something downloads.
    #[cfg(debug_assertions)]
    let snapping = crate::snap::dir().is_some(); // scenes bring their own counter
    #[cfg(not(debug_assertions))]
    let snapping = false;
    if app.model.stats_open && app.model.totals().0 > 0 && !snapping {
        subs.push(iced::time::every(std::time::Duration::from_secs(5)).map(|_| Message::StatsTick));
    }
    // An empty search folds away once a click moves the focus elsewhere.
    if app.model.search_open && app.model.search.trim().is_empty() {
        subs.push(iced::event::listen_with(|event, _, _| {
            matches!(event, iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_))).then_some(Message::CheckSearchFocus)
        }));
    }
    // Dropped files (lists of links, torrents, shortcuts), and the hint while they're dragged over.
    subs.push(iced::event::listen_with(|event, _, _| match event {
        iced::Event::Window(window::Event::FileHovered(_)) => Some(Message::DropHover(true)),
        iced::Event::Window(window::Event::FilesHoveredLeft) => Some(Message::DropHover(false)),
        iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
        _ => None,
    }));
    if app.model.theme_mode() == ThemeMode::System {
        subs.push(crate::appearance::system_theme().map(Message::SystemTheme));
    }
    #[cfg(debug_assertions)]
    subs.push(crate::snap::subscription());
    if app.model.settings.clipboard_watch {
        subs.push(iced::time::every(std::time::Duration::from_millis(700)).map(|_| Message::ClipboardTick));
    }
    Subscription::batch(subs)
}

/// Ctrl+K focuses search; F1 opens Help; Escape closes whatever is on top.
fn on_key(event: keyboard::Event) -> Option<Message> {
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else { return None };
    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Escape),
        keyboard::Key::Character("k") if modifiers.command() => Some(Message::FocusSearch),
        keyboard::Key::Named(keyboard::key::Named::F1) => Some(Message::OpenInfo(Info::Help)),
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
