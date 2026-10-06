use crate::queues::{QueueDraft, drafts_to_queues};
use crate::state::{Draft, Model, Screen, clipboard_link, explorer_select_arg};
use iced::{Subscription, Task};
use rdm_core::{AppState, Event, ItemId, Manager, MediaFormat, MediaInfo, QueueId};
use std::future::Future;
use std::hash::{Hash, Hasher};
use tokio::sync::broadcast::error::RecvError;

pub struct App {
    pub model: Model,
    pub manager: Manager,
}

#[derive(Debug, Clone)]
pub enum Message {
    UrlChanged(String),
    Add,
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
    OpenSettings,
    CloseSettings,
    SaveSettings,
    DraftDir(String),
    DraftConnections(String),
    DraftMax(String),
    DraftLimit(String),
    DraftSort(bool),
    DraftStart(bool),
    /// yt-dlp finished reading a video link.
    Probed(String, Result<MediaInfo, String>),
    PickOption(usize),
    DownloadPicked,
    CancelPick,
    ClipboardTick,
    ClipboardText(Option<String>),
    DraftClipboard(bool),
    CopyToken,
    NewToken,
    OpenQueues,
    CloseQueues,
    SaveQueues,
    AddQueue,
    DeleteQueue(usize),
    QueueName(usize, String),
    QueueMax(usize, String),
    QueueScheduled(usize, bool),
    QueueStart(usize, String),
    QueueStop(usize, String),
    QueueDay(usize, usize, bool),
    MoveToQueue(ItemId, QueueId),
    /// Footer speed-limit list (bytes/s, 0 = none).
    QuickLimit(u64),
    DraftQuality(Option<MediaFormat>),
    DraftAsk(bool),
    UpdateYtdlp,
    YtdlpUpdated(Result<String, String>),
    Done,
}

pub fn boot(manager: Manager, bridge_status: String) -> (App, Task<Message>) {
    let m = manager.clone();
    let model = Model { bridge_status, ..Model::default() };
    (App { model, manager }, Task::perform(async move { m.snapshot().await }, Message::Loaded))
}

/// Runs a manager call in the background; its result isn't needed (events report it).
fn fire<Fut>(manager: &Manager, call: impl FnOnce(Manager) -> Fut) -> Task<Message>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    Task::perform(call(manager.clone()), |_| Message::Done)
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    let model = &mut app.model;
    match message {
        Message::UrlChanged(url) => model.url = url,
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
                return iced::window::latest().and_then(iced::window::gain_focus);
            }
        }
        Message::Loaded(state) => model.load(state),
        Message::Lagged => {
            let m = app.manager.clone();
            return Task::perform(async move { m.snapshot().await }, Message::Loaded);
        }
        Message::OpenSettings => {
            model.draft = Draft::from_settings(&model.settings);
            model.notice = None;
            model.screen = Screen::Settings;
        }
        Message::CloseSettings => {
            model.draft = Draft::from_settings(&model.settings);
            model.notice = None;
            model.screen = Screen::Downloads;
        }
        Message::SaveSettings => match model.draft.to_settings(&model.settings) {
            Ok(settings) => {
                model.notice = None;
                model.screen = Screen::Downloads;
                return fire(&app.manager, move |m| async move { m.update_settings(settings).await });
            }
            Err(e) => model.notice = Some(e),
        },
        Message::DraftDir(v) => model.draft.download_dir = v,
        Message::DraftConnections(v) => model.draft.connections = v,
        Message::DraftMax(v) => model.draft.max_concurrent = v,
        Message::DraftLimit(v) => model.draft.speed_limit_kbps = v,
        Message::DraftSort(v) => model.draft.sort_into_folders = v,
        Message::DraftStart(v) => model.draft.start_immediately = v,
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
        Message::PickOption(i) => {
            if let Some(p) = &mut model.picker {
                p.choice = i;
            }
        }
        Message::DownloadPicked => {
            let requests = model.picker.take().map(|p| p.requests()).unwrap_or_default();
            model.screen = Screen::Downloads;
            let m = app.manager.clone();
            return Task::perform(
                async move {
                    for (url, title, format) in requests {
                        m.add_media(url, title, format).await;
                    }
                },
                |_| Message::Done,
            );
        }
        Message::CancelPick => {
            model.picker = None;
            model.screen = Screen::Downloads;
        }
        Message::ClipboardTick => return iced::clipboard::read().map(Message::ClipboardText),
        Message::ClipboardText(text) => {
            let link = text.and_then(|t| clipboard_link(&t, model.last_clipboard.as_deref()));
            if let Some(link) = link {
                model.last_clipboard = Some(link.clone());
                let suggest = model.clipboard_primed && model.url.is_empty() && model.screen == Screen::Downloads;
                if suggest {
                    model.url = link;
                    model.notice = Some("Link detected from the clipboard. Press Add to download it.".into());
                }
            }
            model.clipboard_primed = true;
        }
        Message::DraftClipboard(v) => model.draft.clipboard_watch = v,
        Message::CopyToken => return iced::clipboard::write(model.settings.extension_token.clone()),
        Message::NewToken => {
            let settings = rdm_core::Settings { extension_token: rdm_core::new_token(), ..model.settings.clone() };
            return fire(&app.manager, move |m| async move { m.update_settings(settings).await });
        }
        Message::OpenQueues => {
            model.queue_drafts = model.queues.iter().map(QueueDraft::from_queue).collect();
            model.notice = None;
            model.screen = Screen::Queues;
        }
        Message::CloseQueues => {
            model.notice = None;
            model.screen = Screen::Downloads;
        }
        Message::SaveQueues => match drafts_to_queues(&model.queue_drafts) {
            Ok(queues) => {
                model.notice = None;
                model.screen = Screen::Downloads;
                return fire(&app.manager, move |m| async move { m.set_queues(queues).await });
            }
            Err(e) => model.notice = Some(e),
        },
        Message::AddQueue => {
            let fresh = QueueDraft::new(&model.queue_drafts);
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
        Message::MoveToQueue(id, queue) => return fire(&app.manager, move |m| async move { m.move_to_queue(id, queue).await }),
        Message::QuickLimit(bps) => {
            let settings = rdm_core::Settings { speed_limit_bps: bps, ..model.settings.clone() };
            return fire(&app.manager, move |m| async move { m.update_settings(settings).await });
        }
        Message::DraftQuality(q) => model.draft.preferred_quality = q,
        Message::DraftAsk(v) => model.draft.ask_quality = v,
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
        Message::Done => {}
    }
    Task::none()
}

fn edit_queue(model: &mut Model, i: usize, change: impl FnOnce(&mut QueueDraft)) {
    if let Some(d) = model.queue_drafts.get_mut(i) {
        change(d);
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
    let feed = core_events(app);
    if app.model.settings.clipboard_watch {
        let clipboard = iced::time::every(std::time::Duration::from_millis(700)).map(|_| Message::ClipboardTick);
        Subscription::batch([feed, clipboard])
    } else {
        feed
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
