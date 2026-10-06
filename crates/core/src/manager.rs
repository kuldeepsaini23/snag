use crate::category::Category;
use crate::cookies::{Cookie, Jar};
use crate::model::{AppState, Item, ItemId, Kind, Queue, QueueId, Settings, Status};
use crate::planner::{pick_next, queue_active};
use crate::schedule::Now;
use crate::store;
use rdm_engine::filename::{filename_from, resume_target};
use rdm_engine::state::{part_path, state_path};
use rdm_engine::{CancellationToken, Client, DownloadOptions, EngineError, Outcome, Progress, RateLimiter, client_with, default_client, download, probe};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use rdm_media::{MediaFormat, MediaInfo, MediaOptions, MediaOutcome, MediaProgress, YTDLP_URL};
use tokio::time::Instant;

const TICK: Duration = Duration::from_millis(250);
const SAVE_EVERY: Duration = Duration::from_secs(1);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);
/// How often yt-dlp updates itself (sites change, old versions stop working).
const UPDATE_EVERY: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Folder (inside the download folder) holding each video item's partial files.
const PARTS_DIR: &str = ".rdm-parts";

#[derive(Clone, Debug)]
pub enum Event {
    Added(Item),
    Updated(Item),
    Removed(ItemId),
    Settings(Settings),
    /// A video link arrived from outside the window (the browser extension):
    /// the UI should let the user choose a quality.
    PickMedia { url: String, info: MediaInfo, choice: usize },
    /// The queue list changed.
    Queues(Vec<Queue>),
    /// Something the user should see (e.g. a link from the browser that couldn't be read).
    Notice(String),
    /// Bring the window to the front (a second RDM was started).
    Focus,
}

/// Handle to the download manager. Cheap to clone; all clones talk to one actor.
#[derive(Clone)]
pub struct Manager {
    tx: mpsc::UnboundedSender<Cmd>,
    events: broadcast::Sender<Event>,
    tools: Tools,
    /// Browser cookies, shared with the actor. Memory only.
    jar: Arc<Mutex<Jar>>,
    cookie_dir: PathBuf,
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
    /// Runs detached: a caller that gives up (pause) never interrupts a download or update of the exe.
    async fn ytdlp(&self) -> Result<PathBuf, String> {
        let this = self.clone();
        tokio::spawn(async move { this.ytdlp_now().await }).await.map_err(|e| e.to_string())?
    }

    fn marker(&self) -> PathBuf {
        self.bin_dir.join("yt-dlp.checked")
    }

    async fn ytdlp_now(&self) -> Result<PathBuf, String> {
        let _only_one = self.lock.lock().await;
        let path = self.bin_dir.join("yt-dlp.exe");
        if path.exists() {
            let checked = std::fs::metadata(self.marker()).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
            if checked.is_none_or(|age| age >= UPDATE_EVERY) {
                // Marked first: a failed update is retried next week, not before every download.
                let _ = std::fs::write(self.marker(), b"");
                let _ = tokio::time::timeout(Duration::from_secs(60), rdm_media::self_update(&path)).await;
            }
            return Ok(path);
        }
        let (progress, _) = watch::channel(Progress::default());
        match download(&self.client, YTDLP_URL, &path, &DownloadOptions::default(), CancellationToken::new(), &progress).await {
            Ok(Outcome::Completed(_)) => {
                let _ = std::fs::write(self.marker(), b"");
                Ok(path)
            }
            Ok(Outcome::Paused) => Err("yt-dlp download was interrupted".into()),
            Err(e) => Err(format!("couldn't download yt-dlp: {e}")),
        }
    }
}

enum Cmd {
    Snapshot(oneshot::Sender<AppState>),
    AddMedia(String, String, MediaFormat, oneshot::Sender<ItemId>),
    AddWith(String, Option<String>, oneshot::Sender<ItemId>),
    Offer(String, MediaInfo),
    SetQueues(Vec<Queue>),
    MoveToQueue(ItemId, QueueId),
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
        let jar = Arc::new(Mutex::new(Jar::default()));
        let cookie_dir = state_path.parent().unwrap_or(Path::new(".")).join("cookies");
        let actor = Actor::new(state_path, events.clone(), tools.clone(), jar.clone(), cookie_dir.clone());
        tokio::spawn(actor.run(rx));
        Manager { tx, events, tools, jar, cookie_dir }
    }

    /// Reads a video/audio page (title, qualities, playlist entries). Fetches yt-dlp
    /// on first use.
    pub async fn probe_media(&self, url: String) -> Result<MediaInfo, String> {
        let ytdlp = self.tools.ytdlp().await?;
        static PROBES: AtomicU64 = AtomicU64::new(0);
        let key = format!("probe-{}", PROBES.fetch_add(1, Ordering::Relaxed));
        let cookies = cookie_file(&self.jar, &self.cookie_dir, &key, &url);
        let result = rdm_media::probe(&ytdlp, &url, cookies.as_deref()).await;
        if let Some(file) = cookies {
            let _ = std::fs::remove_file(file);
        }
        result
    }

    pub async fn notify(&self, text: String) {
        let _ = self.events.send(Event::Notice(text));
    }

    pub async fn focus(&self) {
        let _ = self.events.send(Event::Focus);
    }

    /// Asks the UI to show the quality picker for `url` (nothing is added yet).
    /// With "ask every time" off, the preferred quality is added straight away instead.
    pub async fn offer_media(&self, url: String, info: MediaInfo) {
        let _ = self.tx.send(Cmd::Offer(url, info));
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
        self.add_with(url, None).await
    }

    /// `add`, remembering the page it came from (sent as Referer).
    pub async fn add_with(&self, url: String, referrer: Option<String>) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddWith(url, referrer, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    /// Browser cookies for later downloads (memory only).
    pub fn remember_cookies(&self, cookies: Vec<Cookie>) {
        if let Ok(mut jar) = self.jar.lock() {
            jar.add(cookies);
        }
    }

    /// Replaces the queue list. Main (id 0) always stays; items of removed queues move to Main.
    pub async fn set_queues(&self, queues: Vec<Queue>) {
        let _ = self.tx.send(Cmd::SetQueues(queues));
    }

    /// Unknown queues are ignored.
    pub async fn move_to_queue(&self, id: ItemId, queue: QueueId) {
        let _ = self.tx.send(Cmd::MoveToQueue(id, queue));
    }

    /// Runs `yt-dlp -U` and returns what it said.
    pub async fn update_ytdlp(&self) -> Result<String, String> {
        let path = self.tools.ytdlp().await?;
        let tools = self.tools.clone();
        tokio::spawn(async move {
            let _only_one = tools.lock.lock().await;
            let result = rdm_media::self_update(&path).await;
            let _ = std::fs::write(tools.marker(), b"");
            result
        })
        .await
        .map_err(|e| e.to_string())?
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

/// Where a video/audio item's final file goes.
fn media_dir(settings: &Settings, format: &MediaFormat) -> PathBuf {
    let category = if *format == MediaFormat::AudioMp3 { Category::Music } else { Category::Video };
    if settings.sort_into_folders { settings.download_dir.join(category.folder()) } else { settings.download_dir.clone() }
}

/// Writes the browser cookies for `url` as a cookies.txt for yt-dlp; `None` when there are none.
fn cookie_file(jar: &Mutex<Jar>, dir: &Path, key: &str, url: &str) -> Option<PathBuf> {
    let text = jar.lock().ok()?.netscape(url)?;
    std::fs::create_dir_all(dir).ok()?;
    let path = dir.join(format!("{key}.txt"));
    std::fs::write(&path, text).ok()?;
    Some(path)
}

/// Deletes a video item's partial-file folder in the background, retrying while a just-killed
/// yt-dlp still holds files open. Only ever touches folders inside a `.rdm-parts` folder.
fn remove_parts(dir: PathBuf) {
    let Some(parent) = dir.parent().filter(|p| p.file_name().is_some_and(|n| n == PARTS_DIR)).map(Path::to_path_buf) else { return };
    std::thread::spawn(move || {
        for _ in 0..20 {
            match std::fs::remove_dir_all(&dir) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => std::thread::sleep(Duration::from_millis(250)),
                _ => break,
            }
        }
        let _ = std::fs::remove_dir(parent); // only succeeds once no other item uses it
    });
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
    jar: Arc<Mutex<Jar>>,
    cookie_dir: PathBuf,
    events: broadcast::Sender<Event>,
    msg_tx: mpsc::UnboundedSender<Msg>,
    msg_rx: mpsc::UnboundedReceiver<Msg>,
    dirty: bool,
    last_save: Instant,
}

impl Actor {
    fn new(path: PathBuf, events: broadcast::Sender<Event>, tools: Tools, jar: Arc<Mutex<Jar>>, cookie_dir: PathBuf) -> Self {
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
            jar,
            cookie_dir,
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
            Cmd::AddWith(url, referrer, reply) => {
                let name = filename_from(None, &url);
                let category = Category::from_name(&name);
                let _ = reply.send(self.push_item(url, name, category, Kind::Http, referrer));
            }
            Cmd::AddMedia(url, title, format, reply) => {
                let _ = reply.send(self.push_media(url, title, format));
            }
            Cmd::Offer(url, info) => self.offer(url, info),
            Cmd::SetQueues(queues) => self.set_queues(queues),
            Cmd::MoveToQueue(id, queue) => {
                if self.state.queue(queue).is_none() {
                    return;
                }
                if let Some(item) = self.state.item_mut(id).filter(|i| i.queue != queue) {
                    item.queue = queue;
                    self.updated(id);
                    // Stops it if its new queue isn't allowed to run now.
                    self.schedule();
                }
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
                    if let Some(dir) = item.work_dir.take() {
                        remove_parts(dir);
                    }
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
                if s.speed_limit_bps != self.state.settings.speed_limit_bps {
                    self.restart_media();
                }
                self.limiter.set_limit(s.speed_limit_bps);
                self.state.settings = s.clone();
                self.emit(Event::Settings(s));
                self.dirty = true;
                self.schedule();
            }
            Cmd::Shutdown(_) => unreachable!("handled in run"),
        }
    }

    fn push_media(&mut self, url: String, title: String, format: MediaFormat) -> ItemId {
        let category = if format == MediaFormat::AudioMp3 { Category::Music } else { Category::Video };
        self.push_item(url, title, category, Kind::Media(format), None)
    }

    /// Shows the picker with the preferred quality selected, or adds that quality right away.
    fn offer(&mut self, url: String, info: MediaInfo) {
        let choice = info.preferred(self.state.settings.preferred_quality.as_ref());
        if self.state.settings.ask_quality {
            self.emit(Event::PickMedia { url, info, choice });
            return;
        }
        let requests = info.requests(&url, choice);
        if requests.is_empty() {
            self.emit(Event::Notice("No downloadable video or audio found on that page.".into()));
            return;
        }
        let label = info.options.get(choice).map(|o| o.label.clone()).unwrap_or_default();
        let count = requests.len();
        for (url, title, format) in requests {
            self.push_media(url, title, format);
        }
        let what = if count == 1 { format!("\u{201c}{}\u{201d}", info.title) } else { format!("{count} videos") };
        self.emit(Event::Notice(format!("Added {what} ({label})")));
    }

    fn set_queues(&mut self, mut queues: Vec<Queue>) {
        if !queues.iter().any(|q| q.id == 0) {
            let main = self.state.queue(0).cloned().unwrap_or_else(|| AppState::default().queues.remove(0));
            queues.insert(0, main);
        }
        self.state.queues = queues;
        let orphans: Vec<ItemId> = self.state.items.iter().filter(|i| self.state.queue(i.queue).is_none()).map(|i| i.id).collect();
        for id in orphans {
            if let Some(item) = self.state.item_mut(id) {
                item.queue = 0;
            }
            self.updated(id);
        }
        self.emit(Event::Queues(self.state.queues.clone()));
        self.dirty = true;
        self.schedule();
    }

    /// Restarts running video downloads so a new speed limit reaches yt-dlp (it continues its partial files).
    fn restart_media(&mut self) {
        for (id, r) in self.running.iter_mut() {
            let media = self.state.item(*id).is_some_and(|i| matches!(i.kind, Kind::Media(_)));
            if media && r.stop == Stop::None {
                r.stop = Stop::Schedule; // comes back as Queued and starts again
                r.cancel.cancel();
            }
        }
    }

    fn push_item(&mut self, url: String, name: String, category: Category, kind: Kind, referrer: Option<String>) -> ItemId {
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
            referrer,
            work_dir: None,
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
        let settings = self.state.settings.clone();
        let Some(item) = self.state.item_mut(id) else { return };
        item.status = Status::Running;
        item.speed_bps = 0;
        if let Kind::Media(format) = &item.kind
            && item.work_dir.is_none()
        {
            item.work_dir = Some(media_dir(&settings, format).join(PARTS_DIR).join(id.0.to_string()));
        }
        let (url, existing_dest, kind) = (item.url.clone(), item.dest.clone(), item.kind.clone());
        let (referrer, work_dir) = (item.referrer.clone(), item.work_dir.clone());
        self.updated(id);

        let cancel = CancellationToken::new();
        let (progress_tx, progress_rx) = watch::channel(Progress::default());
        self.running.insert(id, Running { cancel: cancel.clone(), progress: progress_rx, stop: Stop::None });
        if let Kind::Media(format) = kind {
            self.start_media(id, url, format, work_dir, cancel, progress_tx);
            return;
        }

        let opts = DownloadOptions {
            connections: self.state.settings.connections.max(1),
            limiter: Some(self.limiter.clone()),
            ..Default::default()
        };
        let (client, msg_tx) = (self.client_for(&url, referrer.as_deref()), self.msg_tx.clone());
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
    /// The browser's cookies and the page as Referer, when there are any.
    fn client_for(&self, url: &str, referrer: Option<&str>) -> Client {
        use rdm_engine::header::{COOKIE, HeaderMap, HeaderValue, REFERER};
        let mut headers = HeaderMap::new();
        let cookie = self.jar.lock().ok().and_then(|jar| jar.header(url));
        if let Some(value) = cookie.and_then(|c| HeaderValue::from_str(&c).ok()) {
            headers.insert(COOKIE, value);
        }
        if let Some(value) = referrer.and_then(|r| HeaderValue::from_str(r).ok()) {
            headers.insert(REFERER, value);
        }
        if headers.is_empty() { self.client.clone() } else { client_with(headers) }
    }

    fn start_media(&mut self, id: ItemId, url: String, format: MediaFormat, work_dir: Option<PathBuf>, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let dir = media_dir(settings, &format);
        // yt-dlp can't share the engine's limiter: each running download gets an equal share.
        let limit = settings.speed_limit_bps;
        let limit_bps = if limit == 0 { 0 } else { (limit / self.running.len().max(1) as u64).max(1) };
        let cookies = cookie_file(&self.jar, &self.cookie_dir, &id.0.to_string(), &url);
        let opts = MediaOptions { cookies: cookies.clone(), limit_bps, temp_dir: work_dir };
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
                let outcome = rdm_media::download(&ytdlp, &url, &format, &dir, &opts, cancel, &media_tx).await;
                drop(media_tx);
                let _ = forward.await;
                Ok(match outcome? {
                    MediaOutcome::Completed(path) => Outcome::Completed(path),
                    MediaOutcome::Paused => Outcome::Paused,
                })
            }
            .await;
            if let Some(file) = cookies {
                let _ = std::fs::remove_file(file);
            }
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
                    if item.status == Status::Done
                        && let Some(dir) = item.work_dir.take()
                    {
                        remove_parts(dir);
                    }
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
        if let Some(dir) = item.work_dir.clone() {
            remove_parts(dir);
        }
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
