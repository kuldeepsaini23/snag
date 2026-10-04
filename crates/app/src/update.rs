use crate::state::{Draft, Model, Picker, Screen};
use iced::{Subscription, Task};
use rdm_core::{AppState, Event, ItemId, Manager, MediaInfo};
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
    Done,
}

pub fn boot(manager: Manager) -> (App, Task<Message>) {
    let m = manager.clone();
    (App { model: Model::default(), manager }, Task::perform(async move { m.snapshot().await }, Message::Loaded))
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
        Message::Remove(id) => return fire(&app.manager, move |m| async move { m.remove(id, false).await }),
        Message::Delete(id) => return fire(&app.manager, move |m| async move { m.remove(id, true).await }),
        Message::ShowInFolder(id) => {
            if let Some(path) = model.dest_of(id) {
                let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
            }
        }
        Message::Core(event) => model.apply(event),
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
                    model.picker = Some(Picker { url, info, choice: 0 });
                    model.screen = Screen::Picker;
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
        Message::Done => {}
    }
    Task::none()
}

/// Identity for the core event subscription (there is only ever one feed).
struct Feed(Manager);

impl Hash for Feed {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "rdm-core-events".hash(state);
    }
}

pub fn subscription(app: &App) -> Subscription<Message> {
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
