use crate::category::Category;
use crate::model::{AppState, Item, ItemId, Kind, Settings, Status};
use crate::planner::{pick_next, queue_active};
use crate::schedule::Now;
use crate::store;
use rdm_engine::filename::{filename_from, resume_target};
use rdm_engine::state::{part_path, state_path};
use rdm_engine::{CancellationToken, Client, DownloadOptions, EngineError, Outcome, Progress, RateLimiter, default_client, download, probe};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use rdm_media::{MediaFormat, MediaInfo, MediaOutcome, MediaProgress, YTDLP_URL};
use tokio::time::Instant;

const TICK: Duration = Duration::from_millis(250);
const SAVE_EVERY: Duration = Duration::from_secs(1);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub enum Event {
    Added(Item),
    Updated(Item),
    Removed(ItemId),
    Settings(Settings),
    /// A video link arrived from outside the window (the browser extension):
    /// the UI should let the user choose a quality.
    PickMedia { url: String, info: MediaInfo },
}

/// Handle to the download manager. Cheap to clone; all clones talk to one actor.
#[derive(Clone)]
pub struct Manager {
    tx: mpsc::UnboundedSender<Cmd>,
    events: broadcast::Sender<Event>,
    tools: Tools,
}

/// Locates yt-dlp, downloading it next to `state.json` (in `bin/`) the first time.
#[derive(Clone)]
struct Tools {
    bin_dir: PathBuf,
    client: Client,
    /// Only one first-time download at a time.
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl Tools {
    async fn ytdlp(&self) -> Result<PathBuf, String> {
        let _only_one = self.lock.lock().await;
        let path = self.bin_dir.join("yt-dlp.exe");
        if path.exists() {
            return Ok(path);
        }
        let (progress, _) = watch::channel(Progress::default());
        match download(&self.client, YTDLP_URL, &path, &DownloadOptions::default(), CancellationToken::new(), &progress).await {
            Ok(Outcome::Completed(_)) => Ok(path),
            Ok(Outcome::Paused) => Err("yt-dlp download was interrupted".into()),
            Err(e) => Err(format!("couldn't download yt-dlp: {e}")),
        }
    }
}

enum Cmd {
    Snapshot(oneshot::Sender<AppState>),
    Add(String, oneshot::Sender<ItemId>),
    AddMedia(String, String, MediaFormat, oneshot::Sender<ItemId>),
    Pause(ItemId),
    Resume(ItemId),
    Redownload(ItemId),
    Remove(ItemId, bool),
    Settings(Settings),
    Shutdown(oneshot::Sender<()>),
}

impl Manager {
    /// Loads `state_path` and spawns the manager on the current tokio runtime.
    pub fn start(state_path: PathBuf) -> Manager {
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(1024);
        let tools = Tools {
            bin_dir: state_path.parent().unwrap_or(Path::new(".")).join("bin"),
            client: default_client(),
            lock: Arc::default(),
        };
        let actor = Actor::new(state_path, events.clone(), tools.clone());
        tokio::spawn(actor.run(rx));
        Manager { tx, events, tools }
    }

    /// Reads a video/audio page (title, qualities, playlist entries). Fetches yt-dlp
    /// on first use.
    pub async fn probe_media(&self, url: String) -> Result<MediaInfo, String> {
        let ytdlp = self.tools.ytdlp().await?;
        rdm_media::probe(&ytdlp, &url).await
    }

    /// Asks the UI to show the quality picker for `url` (nothing is added yet).
    pub async fn offer_media(&self, url: String, info: MediaInfo) {
        let _ = self.events.send(Event::PickMedia { url, info });
    }

    pub async fn add_media(&self, url: String, title: String, format: MediaFormat) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddMedia(url, title, format, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// The full current state. Use it to (re)build a view, e.g. after `Lagged`.
    pub async fn snapshot(&self) -> AppState {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::Snapshot(reply));
        rx.await.unwrap_or_default()
    }

    pub async fn add(&self, url: String) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::Add(url, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    pub async fn pause(&self, id: ItemId) {
        let _ = self.tx.send(Cmd::Pause(id));
    }

    pub async fn resume(&self, id: ItemId) {
        let _ = self.tx.send(Cmd::Resume(id));
    }

    pub async fn remove(&self, id: ItemId, delete_file: bool) {
        let _ = self.tx.send(Cmd::Remove(id, delete_file));
    }

    /// Downloads a finished (or failed) item again from scratch, e.g. after its file was deleted.
    pub async fn redownload(&self, id: ItemId) {
        let _ = self.tx.send(Cmd::Redownload(id));
    }

    pub async fn update_settings(&self, s: Settings) {
        let _ = self.tx.send(Cmd::Settings(s));
    }

    /// Pauses everything that is running, saves, and stops the manager.
    pub async fn shutdown(&self) {
        let (reply, rx) = oneshot::channel();
        if self.tx.send(Cmd::Shutdown(reply)).is_ok() {
            let _ = rx.await;
        }
    }
}

/// Why a running item is being stopped; decides its status afterwards.
#[derive(Clone, Copy, PartialEq)]
enum Stop {
    None,
    Pause,
    Schedule,
    Remove { delete_file: bool },
    Shutdown,
}

struct Running {
    cancel: CancellationToken,
    progress: watch::Receiver<Progress>,
    stop: Stop,
}

/// Messages from download tasks back to the actor.
enum Msg {
    /// Probe finished: the actor picks the final destination and replies with it.
    Resolve { id: ItemId, name: String, total: Option<u64>, reply: oneshot::Sender<PathBuf> },
    Finished { id: ItemId, result: Result<Outcome, String> },
}

struct Actor {
    path: PathBuf,
    state: AppState,
    running: HashMap<ItemId, Running>,
    limiter: Arc<RateLimiter>,
    client: Client,
    tools: Tools,
    events: broadcast::Sender<Event>,
    msg_tx: mpsc::UnboundedSender<Msg>,
    msg_rx: mpsc::UnboundedReceiver<Msg>,
    dirty: bool,
    last_save: Instant,
}

impl Actor {
    fn new(path: PathBuf, events: broadcast::Sender<Event>, tools: Tools) -> Self {
        let mut state = store::load(&path);
        let needs_token = state.settings.extension_token.is_empty();
        if needs_token {
            state.settings.extension_token = crate::model::new_token();
        }
        let limiter = Arc::new(RateLimiter::new(state.settings.speed_limit_bps));
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Self {
            path,
            state,
            running: HashMap::new(),
            limiter,
            client: tools.client.clone(),
            tools,
            events,
            msg_tx,
            msg_rx,
            dirty: needs_token,
            last_save: Instant::now(),
        }
    }

    async fn run(mut self, mut cmds: mpsc::UnboundedReceiver<Cmd>) {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        self.schedule();
        loop {
            tokio::select! {
                cmd = cmds.recv() => match cmd {
                    Some(Cmd::Shutdown(reply)) => {
                        self.shutdown().await;
                        let _ = reply.send(());
                        return;
                    }
                    Some(cmd) => self.handle(cmd),
                    // Every handle was dropped.
                    None => {
                        self.shutdown().await;
                        return;
                    }
                },
                Some(msg) = self.msg_rx.recv() => self.on_msg(msg),
                _ = tick.tick() => self.on_tick(),
            }
        }
    }

    fn emit(&self, e: Event) {
        let _ = self.events.send(e);
    }

    fn updated(&mut self, id: ItemId) {
        if let Some(item) = self.state.item(id) {
            self.emit(Event::Updated(item.clone()));
        }
        self.dirty = true;
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Snapshot(reply) => {
                let _ = reply.send(self.state.clone());
            }
            Cmd::Add(url, reply) => {
                let name = filename_from(None, &url);
                let category = Category::from_name(&name);
                let _ = reply.send(self.push_item(url, name, category, Kind::Http));
            }
            Cmd::AddMedia(url, title, format, reply) => {
                let category = if format == MediaFormat::AudioMp3 { Category::Music } else { Category::Video };
                let _ = reply.send(self.push_item(url, title, category, Kind::Media(format)));
            }
            Cmd::Pause(id) => {
                if let Some(r) = self.running.get_mut(&id) {
                    if matches!(r.stop, Stop::Remove { .. }) {
                        return; // being removed: a pause must not bring it back
                    }
                    r.stop = Stop::Pause;
                    r.cancel.cancel();
                } else if let Some(item) = self.state.item_mut(id).filter(|i| i.status == Status::Queued) {
                    item.status = Status::Paused;
                    self.updated(id);
                }
            }
            Cmd::Resume(id) => {
                if self.running.contains_key(&id) {
                    return;
                }
                if let Some(item) = self.state.item_mut(id).filter(|i| matches!(i.status, Status::Paused | Status::Failed(_))) {
                    item.status = Status::Queued;
                    self.updated(id);
                    self.schedule();
                }
            }
            Cmd::Redownload(id) => {
                if self.running.contains_key(&id) {
                    return;
                }
                if let Some(item) = self.state.item_mut(id) {
                    item.status = Status::Queued;
                    item.dest = None;
                    item.downloaded = 0;
                    item.total = None;
                    item.speed_bps = 0;
                    self.updated(id);
                    self.schedule();
                }
            }
            Cmd::Remove(id, delete_file) => {
                if let Some(r) = self.running.get_mut(&id) {
                    r.stop = Stop::Remove { delete_file };
                    r.cancel.cancel();
                } else {
                    self.remove_now(id, delete_file);
                }
            }
            Cmd::Settings(s) => {
                self.limiter.set_limit(s.speed_limit_bps);
                self.state.settings = s.clone();
                self.emit(Event::Settings(s));
                self.dirty = true;
                self.schedule();
            }
            Cmd::Shutdown(_) => unreachable!("handled in run"),
        }
    }

    fn push_item(&mut self, url: String, name: String, category: Category, kind: Kind) -> ItemId {
        let id = ItemId(self.state.next_id);
        self.state.next_id += 1;
        let item = Item {
            id,
            url,
            name,
            category,
            status: if self.state.settings.start_immediately { Status::Queued } else { Status::Paused },
            dest: None,
            downloaded: 0,
            total: None,
            speed_bps: 0,
            queue: 0,
            added: chrono::Utc::now().timestamp(),
            kind,
        };
        self.state.items.push(item.clone());
        self.emit(Event::Added(item));
        self.dirty = true;
        self.schedule();
        id
    }

    /// Stops items whose queue went inactive and starts whatever the planner picks.
    fn schedule(&mut self) {
        let now = Now::local();
        for (id, r) in self.running.iter_mut() {
            let active = self.state.item(*id).is_some_and(|i| queue_active(&self.state, i.queue, now));
            if !active && r.stop == Stop::None {
                r.stop = Stop::Schedule;
                r.cancel.cancel();
            }
        }
        let running: HashSet<ItemId> = self.running.keys().copied().collect();
        for id in pick_next(&self.state, &running, now) {
            self.start(id);
        }
    }

    fn start(&mut self, id: ItemId) {
        let Some(item) = self.state.item_mut(id) else { return };
        item.status = Status::Running;
        item.speed_bps = 0;
        let (url, existing_dest, kind) = (item.url.clone(), item.dest.clone(), item.kind.clone());
        self.updated(id);

        let cancel = CancellationToken::new();
        let (progress_tx, progress_rx) = watch::channel(Progress::default());
        self.running.insert(id, Running { cancel: cancel.clone(), progress: progress_rx, stop: Stop::None });
        if let Kind::Media(format) = kind {
            self.start_media(id, url, format, cancel, progress_tx);
            return;
        }

        let opts = DownloadOptions {
            connections: self.state.settings.connections.max(1),
            limiter: Some(self.limiter.clone()),
            ..Default::default()
        };
        let (client, msg_tx) = (self.client.clone(), self.msg_tx.clone());
        tokio::spawn(async move {
            let result = async {
                let dest = match existing_dest {
                    Some(dest) => dest,
                    None => {
                        let info = tokio::select! {
                            _ = cancel.cancelled() => return Ok(Outcome::Paused),
                            info = probe(&client, &url) => info?,
                        };
                        let (reply, dest_rx) = oneshot::channel();
                        let _ = msg_tx.send(Msg::Resolve { id, name: info.filename, total: info.size, reply });
                        // No reply: the item was removed or the manager is stopping.
                        match dest_rx.await {
                            Ok(dest) => dest,
                            Err(_) => return Ok(Outcome::Paused),
                        }
                    }
                };
                download(&client, &url, &dest, &opts, cancel, &progress_tx).await
            }
            .await
            .map_err(|e: EngineError| e.to_string());
            let _ = msg_tx.send(Msg::Finished { id, result });
        });
    }

    /// Video/audio items go through yt-dlp; its progress feeds the same `Progress` watch.
    fn start_media(&mut self, id: ItemId, url: String, format: MediaFormat, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let category = if format == MediaFormat::AudioMp3 { Category::Music } else { Category::Video };
        let dir = if settings.sort_into_folders { settings.download_dir.join(category.folder()) } else { settings.download_dir.clone() };
        let (tools, msg_tx) = (self.tools.clone(), self.msg_tx.clone());
        tokio::spawn(async move {
            let result = async {
                let ytdlp = tokio::select! {
                    _ = cancel.cancelled() => return Ok(Outcome::Paused),
                    path = tools.ytdlp() => path?,
                };
                let (media_tx, mut media_rx) = watch::channel(MediaProgress::default());
                let forward = tokio::spawn(async move {
                    while media_rx.changed().await.is_ok() {
                        let p = media_rx.borrow_and_update().clone();
                        progress_tx.send_replace(Progress { downloaded: p.downloaded, total: p.total, speed_bps: p.speed_bps, segments: Vec::new() });
                    }
                });
                let outcome = rdm_media::download(&ytdlp, &url, &format, &dir, cancel, &media_tx).await;
                drop(media_tx);
                let _ = forward.await;
                Ok(match outcome? {
                    MediaOutcome::Completed(path) => Outcome::Completed(path),
                    MediaOutcome::Paused => Outcome::Paused,
                })
            }
            .await;
            let _ = msg_tx.send(Msg::Finished { id, result });
        });
    }

    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Resolve { id, name, total, reply } => {
                let stopping = self.running.get(&id).is_none_or(|r| r.stop != Stop::None);
                if stopping {
                    return;
                }
                let category = Category::from_name(&name);
                let settings = &self.state.settings;
                let dir = if settings.sort_into_folders { settings.download_dir.join(category.folder()) } else { settings.download_dir.clone() };
                let url = self.state.item(id).map(|i| i.url.clone()).unwrap_or_default();
                let dest = self.free_dest(id, &dir, &name, &url);
                if let Some(item) = self.state.item_mut(id) {
                    item.name = name;
                    item.category = category;
                    item.total = total;
                    item.dest = Some(dest.clone());
                }
                self.updated(id);
                let _ = reply.send(dest);
            }
            Msg::Finished { id, result } => {
                let Some(r) = self.running.remove(&id) else { return };
                if let Stop::Remove { delete_file } = r.stop {
                    self.remove_now(id, delete_file);
                    self.schedule();
                    return;
                }
                let last = r.progress.borrow().clone();
                if let Some(item) = self.state.item_mut(id) {
                    item.speed_bps = 0;
                    item.downloaded = last.downloaded.max(item.downloaded);
                    if last.total.is_some() {
                        item.total = last.total;
                    }
                    item.status = match result {
                        Ok(Outcome::Completed(path)) => {
                            // The real size: video downloads report progress per stream.
                            if let Ok(meta) = std::fs::metadata(&path) {
                                item.downloaded = meta.len();
                                item.total = Some(meta.len());
                            }
                            item.dest = Some(path);
                            Status::Done
                        }
                        Ok(Outcome::Paused) if r.stop == Stop::Schedule => Status::Queued,
                        Ok(Outcome::Paused) => Status::Paused,
                        Err(e) => Status::Failed(e),
                    };
                }
                self.updated(id);
                if r.stop != Stop::Shutdown {
                    self.schedule();
                }
            }
        }
    }

    /// `resume_target`, but never a destination another item already owns
    /// (two items for the same URL would otherwise collide before either creates its part file).
    fn free_dest(&self, id: ItemId, dir: &Path, name: &str, url: &str) -> PathBuf {
        let taken: HashSet<&PathBuf> = self.state.items.iter().filter(|i| i.id != id).filter_map(|i| i.dest.as_ref()).collect();
        let first = resume_target(dir, name, url);
        if !taken.contains(&first) {
            return first;
        }
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
            _ => (name.to_string(), String::new()),
        };
        (1..)
            .map(|i| dir.join(format!("{stem} ({i}){ext}")))
            .find(|p| !taken.contains(p) && !p.exists() && !part_path(p).exists())
            .expect("unbounded counter always finds a free name")
    }

    fn remove_now(&mut self, id: ItemId, delete_file: bool) {
        let Some(pos) = self.state.items.iter().position(|i| i.id == id) else { return };
        let item = self.state.items.remove(pos);
        if let Some(dest) = &item.dest {
            let _ = std::fs::remove_file(part_path(dest));
            let _ = std::fs::remove_file(state_path(dest));
            let shared = self.state.items.iter().any(|other| other.dest.as_ref() == Some(dest));
            if delete_file && !shared && dest.exists() {
                // Recycle Bin, never a permanent delete: a misclick must be undoable.
                let _ = trash::delete(dest);
            }
        }
        self.emit(Event::Removed(id));
        self.dirty = true;
    }

    fn on_tick(&mut self) {
        let mut changed = Vec::new();
        for (id, r) in &self.running {
            let p = r.progress.borrow().clone();
            if let Some(item) = self.state.item_mut(*id) {
                let total = p.total.or(item.total);
                if item.downloaded != p.downloaded || item.speed_bps != p.speed_bps || item.total != total {
                    item.downloaded = p.downloaded;
                    item.speed_bps = p.speed_bps;
                    item.total = total;
                    changed.push(*id);
                }
            }
        }
        for id in changed {
            if let Some(item) = self.state.item(id) {
                self.emit(Event::Updated(item.clone()));
            }
        }
        self.schedule();
        if self.dirty && self.last_save.elapsed() >= SAVE_EVERY {
            self.save();
        }
    }

    fn save(&mut self) {
        if store::save(&self.path, &self.state).is_ok() {
            self.dirty = false;
            self.last_save = Instant::now();
        }
    }

    async fn shutdown(&mut self) {
        for r in self.running.values_mut() {
            if !matches!(r.stop, Stop::Remove { .. }) {
                r.stop = Stop::Shutdown;
            }
            r.cancel.cancel();
        }
        let deadline = Instant::now() + SHUTDOWN_GRACE;
        while !self.running.is_empty() {
            match tokio::time::timeout_at(deadline, self.msg_rx.recv()).await {
                Ok(Some(msg)) => self.on_msg(msg),
                _ => break,
            }
        }
        // Anything that didn't stop in time is saved as paused, never as running.
        for item in self.state.items.iter_mut().filter(|i| i.status == Status::Running) {
            item.status = Status::Paused;
        }
        self.save();
    }
}
