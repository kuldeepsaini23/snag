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
    /// Another process (rdm --quit, the uninstaller) asks RDM to quit; it pauses and saves first.
    Quit,
    /// The watched channels/playlists changed.
    Watches(Vec<crate::watch::Watch>),
    /// A browser extension asks to connect: the UI asks the user, then `answer_pair`. `origin`
    /// is the asking extension's (`chrome-extension://<id>`), shown in the question.
    PairRequest { id: u64, origin: String },
    /// That link was downloaded before (and the file is still there): nothing was added. The UI
    /// offers Show / Download again (`redownload` of this item) / Skip.
    Duplicate(Item),
    /// VirusTotal's verdict on a downloaded program arrived.
    Safety(ItemId, crate::safety::Safety),
    /// A segmented download's connections changed: each one's range and bytes written.
    Segments(ItemId, Vec<rdm_engine::segments::Segment>),
}

/// What became of a browser extension's request to connect (`Manager::request_pair`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairAnswer {
    Allowed,
    /// Not allowed, or no answer within two minutes.
    Refused,
    /// Another request still waits for the user's answer.
    Busy,
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
    /// Media link → the page it was found on (sent as Referer to yt-dlp). Memory only.
    referrers: Referrers,
    torrents: Arc<Torrents>,
    /// Extensions waiting for the user's answer to "connect?".
    pairs: Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>,
    /// Page reads (`probe_media`) running at once; the rest wait their turn.
    probes: Arc<tokio::sync::Semaphore>,
}

/// How many `probe_media` reads (one yt-dlp each) may run at once.
const PROBES_AT_ONCE: usize = 2;

type Referrers = Arc<Mutex<HashMap<String, String>>>;

/// The torrent engine, started on first use.
#[derive(Default)]
struct Torrents {
    engine: tokio::sync::OnceCell<rdm_torrent::TorrentEngine>,
    /// Tests: local peers only (no DHT or trackers).
    peers: Mutex<Option<Vec<std::net::SocketAddr>>>,
}

impl Torrents {
    async fn engine(&self, dir: &Path) -> Result<rdm_torrent::TorrentEngine, String> {
        let peers = self.peers.lock().ok().and_then(|p| p.clone());
        self.engine
            .get_or_try_init(|| async {
                let opts = match peers {
                    Some(peers) => rdm_torrent::EngineOptions { local_only: true, peers },
                    None => rdm_torrent::EngineOptions::default(),
                };
                rdm_torrent::TorrentEngine::start(dir, opts).await
            })
            .await
            .cloned()
    }
}

/// Total size of the files directly in `dir`.
fn dir_size(dir: &Path) -> Option<u64> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(entries.filter_map(|e| e.ok()?.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum())
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
    /// ffmpeg for conversions: Snag's own copy in `bin`, else one on the PATH.
    fn ffmpeg(&self) -> Option<PathBuf> {
        let own = self.bin_dir.join("ffmpeg.exe");
        if own.exists() {
            return Some(own);
        }
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).map(|d| d.join("ffmpeg.exe")).find(|p| p.exists())
    }

    /// Runs detached: a caller that gives up (pause) never interrupts a download or update of the exe.
    async fn ytdlp(&self) -> Result<PathBuf, String> {
        let this = self.clone();
        tokio::spawn(async move { this.ytdlp_now().await }).await.map_err(|e| e.to_string())?
    }

    fn marker(&self) -> PathBuf {
        self.bin_dir.join("yt-dlp.checked")
    }

    /// gallery-dl, downloaded into `bin/` the first time (detached, like `ytdlp`).
    async fn gallery_dl(&self) -> Result<PathBuf, String> {
        let this = self.clone();
        tokio::spawn(async move {
            let _only_one = this.lock.lock().await;
            let path = this.bin_dir.join("gallery-dl.exe");
            if path.exists() {
                return Ok(path);
            }
            let (progress, _) = watch::channel(Progress::default());
            match download(&this.client, rdm_media::gallery::GALLERY_DL_URL, &path, &DownloadOptions::default(), CancellationToken::new(), &progress).await {
                Ok(Outcome::Completed(_)) => Ok(path),
                Ok(Outcome::Paused) => Err("gallery-dl download was interrupted".into()),
                Err(e) => Err(format!("couldn't download gallery-dl: {e}")),
            }
        })
        .await
        .map_err(|e| e.to_string())?
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
    AddMedia(String, String, MediaFormat, QueueId, Option<String>, Option<f64>, oneshot::Sender<ItemId>),
    AddGallery(String, oneshot::Sender<ItemId>),
    AddPage(String, oneshot::Sender<ItemId>),
    AddTorrent(String, oneshot::Sender<ItemId>),
    Refresh(ItemId, String),
    AddWatch(crate::watch::Watch),
    RemoveWatch(u32),
    CheckWatches,
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
        Self::start_with_retry(state_path, RETRY_BASE)
    }

    /// `start` with a shorter wait before automatic retries (tests).
    #[doc(hidden)]
    pub fn start_with_retry(state_path: PathBuf, retry_base: Duration) -> Manager {
        let (tx, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(1024);
        let tools = Tools {
            bin_dir: state_path.parent().unwrap_or(Path::new(".")).join("bin"),
            client: default_client(),
            lock: Arc::default(),
        };
        let jar = Arc::new(Mutex::new(Jar::default()));
        let cookie_dir = state_path.parent().unwrap_or(Path::new(".")).join("cookies");
        // Cookie files left by a run that was killed or crashed: never leave sessions on disk.
        let _ = std::fs::remove_dir_all(&cookie_dir);
        let referrers = Referrers::default();
        let torrents = Arc::new(Torrents::default());
        let mut actor = Actor::new(state_path, events.clone(), tools.clone(), jar.clone(), cookie_dir.clone());
        actor.retry_base = retry_base;
        actor.referrers = referrers.clone();
        actor.torrents = torrents.clone();
        tokio::spawn(actor.run(rx));
        let probes = Arc::new(tokio::sync::Semaphore::new(PROBES_AT_ONCE));
        Manager { tx, events, tools, jar, cookie_dir, referrers, torrents, pairs: Arc::default(), probes }
    }

    /// Reads a video/audio page (title, qualities, playlist entries). Fetches yt-dlp
    /// on first use. At most two reads run at once (each is a yt-dlp process); others queue.
    pub async fn probe_media(&self, url: String) -> Result<MediaInfo, String> {
        let ytdlp = self.tools.ytdlp().await?;
        let _turn = self.probes.acquire().await.map_err(|e| e.to_string())?;
        static PROBES: AtomicU64 = AtomicU64::new(0);
        let key = format!("probe-{}", PROBES.fetch_add(1, Ordering::Relaxed));
        let cookies = cookie_file(&self.jar, &self.cookie_dir, &key, &url);
        let referer = self.referrers.lock().ok().and_then(|r| r.get(&url).cloned());
        let result = rdm_media::probe(&ytdlp, &url, cookies.as_deref(), referer.as_deref()).await;
        let Some(file) = cookies else { return result };
        let _ = std::fs::remove_file(file);
        if result.is_ok() {
            return result;
        }
        // Browser cookies can be stale (YouTube rotates them while the tab is open: "The page
        // needs to be reloaded"). Try without; if that works, stop sending them for this site.
        let retry = rdm_media::probe(&ytdlp, &url, None, referer.as_deref()).await;
        if retry.is_ok() {
            if let Ok(mut jar) = self.jar.lock() {
                jar.forget(&url);
            }
            return retry;
        }
        result
    }

    pub async fn notify(&self, text: String) {
        let _ = self.events.send(Event::Notice(text));
    }

    pub async fn focus(&self) {
        let _ = self.events.send(Event::Focus);
    }

    /// Asks the UI to quit (see `Event::Quit`).
    pub async fn request_quit(&self) {
        let _ = self.events.send(Event::Quit);
    }

    /// Asks the UI to show the quality picker for `url` (nothing is added yet).
    /// With "ask every time" off, the preferred quality is added straight away instead.
    pub async fn offer_media(&self, url: String, info: MediaInfo) {
        let _ = self.tx.send(Cmd::Offer(url, info));
    }

    pub async fn add_media(&self, url: String, title: String, format: MediaFormat) -> ItemId {
        self.add_media_to(url, title, format, 0).await
    }

    /// A fresh link for a stopped download (the old one expired): it continues from the bytes
    /// already on disk when the file is the same, else starts over. Running items and
    /// non-http(s) links are ignored.
    pub async fn refresh_url(&self, id: ItemId, url: String) {
        let _ = self.tx.send(Cmd::Refresh(id, url));
    }

    /// A page of images (Pinterest, Imgur, an Instagram photo post…): saved by gallery-dl.
    /// Saves a web page as one offline .html file (in `<download>/Pages`), with the browser's
    /// cookies, so logged-in pages save as you see them.
    pub async fn save_page(&self, url: String) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddPage(url, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    pub async fn add_gallery(&self, url: String) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddGallery(url, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    /// `add_media` straight into `queue` (an unknown queue means Main), so it never starts in Main first.
    pub async fn add_media_to(&self, url: String, title: String, format: MediaFormat, queue: QueueId) -> ItemId {
        self.add_media_meta(url, title, format, queue, None, None).await
    }

    /// `add_media_to`, remembering the video's preview image and length.
    pub async fn add_media_meta(&self, url: String, title: String, format: MediaFormat, queue: QueueId, thumbnail: Option<String>, duration: Option<f64>) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddMedia(url, title, format, queue, thumbnail, duration, reply));
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
    /// Watches a channel or playlist: new uploads are downloaded in `format`, every
    /// `every_hours`. The first check only records what's already there.
    pub async fn add_watch(&self, url: String, name: String, format: MediaFormat, max_minutes: Option<u32>, every_hours: u32) {
        let watch = crate::watch::Watch { id: 0, url, name, format, max_minutes, every_hours, queue: 0, last_check: 0, seen: Vec::new(), primed: false, last_len: 0 };
        let _ = self.tx.send(Cmd::AddWatch(watch));
    }

    pub async fn remove_watch(&self, id: u32) {
        let _ = self.tx.send(Cmd::RemoveWatch(id));
    }

    /// Checks every watch now (whether due or not).
    pub async fn check_watches_now(&self) {
        let _ = self.tx.send(Cmd::CheckWatches);
    }

    /// A browser extension (`origin`) asks to connect. The UI shows the question; this waits (up
    /// to two minutes) for the user's answer. One question at a time: while one waits, others
    /// are turned away rather than put in its place.
    pub async fn request_pair(&self, origin: String) -> PairAnswer {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let Ok(mut pairs) = self.pairs.lock() else { return PairAnswer::Refused };
            if !pairs.is_empty() {
                return PairAnswer::Busy;
            }
            pairs.insert(id, tx);
        }
        // Forgets the question however this ends (answered, timed out, or the caller hung up).
        struct Asked<'a>(&'a Mutex<HashMap<u64, oneshot::Sender<bool>>>, u64);
        impl Drop for Asked<'_> {
            fn drop(&mut self) {
                if let Ok(mut pairs) = self.0.lock() {
                    pairs.remove(&self.1);
                }
            }
        }
        let _asked = Asked(&self.pairs, id);
        let _ = self.events.send(Event::PairRequest { id, origin });
        match tokio::time::timeout(Duration::from_secs(120), rx).await {
            Ok(Ok(true)) => PairAnswer::Allowed,
            _ => PairAnswer::Refused,
        }
    }

    /// The user's answer to `Event::PairRequest { id, .. }`.
    pub async fn answer_pair(&self, id: u64, allow: bool) {
        let waiting = self.pairs.lock().ok().and_then(|mut p| p.remove(&id));
        if let Some(tx) = waiting {
            let _ = tx.send(allow);
        }
    }

    /// A magnet link or a link to a `.torrent` file.
    pub async fn add_torrent(&self, url: String) -> ItemId {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::AddTorrent(url, reply));
        rx.await.unwrap_or(ItemId(0))
    }

    /// Tests: torrents use only these peers (no DHT, no trackers). Before the first torrent.
    #[doc(hidden)]
    pub fn use_torrent_peers(&self, peers: Vec<std::net::SocketAddr>) {
        if let Ok(mut p) = self.torrents.peers.lock() {
            *p = Some(peers);
        }
    }

    /// The page a media link was found on (the browser's media sniffer): yt-dlp sends it as
    /// Referer, which many stream hosts require.
    pub fn remember_referrer(&self, url: String, page: String) {
        if let Ok(mut map) = self.referrers.lock() {
            map.insert(url, page);
        }
    }

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

/// Some sites send a bare page to unknown programs; a saved page should look like the browser's.
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0 Safari/537.36";

/// "Rust: Saved Page" → "Rust - Saved Page": the page title as a Windows file name (else the
/// site's name), at most 120 characters.
fn page_file_name(title: Option<&str>, url: &str) -> String {
    let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| "page".into());
    let title = title.map(str::trim).filter(|t| !t.is_empty()).unwrap_or(&host).replace(':', " -");
    let words: Vec<&str> = title.split_whitespace().collect();
    let name: String = rdm_engine::filename::sanitize(&words.join(" ")).chars().take(120).collect();
    name.trim_end_matches(['.', ' ']).to_string()
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
    /// The `--limit-rate` its yt-dlp runs with (videos only; 0 = none).
    limit_bps: u64,
    /// Progress already added to the day's total (it starts at what was on disk before).
    counted: u64,
    /// The segments last sent to the UI.
    segments: Vec<rdm_engine::segments::Segment>,
}

impl Running {
    /// Bytes downloaded since the last call. A report of 0 is the watch's initial value, not
    /// progress; a smaller one is a new stream of a video (its count starts again).
    fn newly_downloaded(&mut self, downloaded: u64) -> u64 {
        if downloaded == 0 {
            return 0;
        }
        let new = downloaded.saturating_sub(self.counted);
        self.counted = downloaded;
        new
    }
}

/// Messages from download tasks back to the actor.
enum Msg {
    /// Probe finished: the actor picks the final destination and replies with it.
    Resolve { id: ItemId, name: String, total: Option<u64>, reply: oneshot::Sender<PathBuf> },
    Finished { id: ItemId, result: Result<Outcome, String> },
    /// A watched channel/playlist was read.
    WatchListed(u32, Result<Vec<rdm_media::Entry>, String>),
    /// An automatic retry's wait is over (only the latest timer of an item counts).
    Retry(ItemId, u64),
    /// VirusTotal answered about a finished program.
    Safety(ItemId, crate::safety::Safety),
    /// A torrent picked its folder: remembered so a resume writes there again.
    TorrentFolder(ItemId, PathBuf),
    /// An after-download rule finished (the item now points at its result).
    RuleDone(ItemId, Result<crate::rules::Applied, String>),
}

/// Automatic retries after a temporary failure: waits of base, 3×base, 9×base.
const AUTO_RETRIES: u32 = 3;
const RETRY_BASE: Duration = Duration::from_secs(10);

/// A failure worth retrying by itself: network trouble or a busy server, not a refusal.
fn is_transient(error: &str) -> bool {
    let e = error.to_ascii_lowercase();
    let busy = ["429", "500", "502", "503", "504"].iter().any(|code| e.contains(code));
    let network = ["timed out", "timeout", "connection", "reset", "temporarily", "network", "dns error", "unreachable", "eof"]
        .iter()
        .any(|w| e.contains(w));
    busy || network
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
    /// Automatic retries used per item since it last started by hand or finished.
    retries: HashMap<ItemId, u32>,
    referrers: Referrers,
    torrents: Arc<Torrents>,
    /// Watches being read right now.
    watching: HashSet<u32>,
    /// The current retry timer of each item; anything the user does cancels it.
    retry_gen: HashMap<ItemId, u64>,
    next_gen: u64,
    retry_base: Duration,
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
            retries: HashMap::new(),
            referrers: Referrers::default(),
            torrents: Arc::default(),
            watching: HashSet::new(),
            retry_gen: HashMap::new(),
            next_gen: 0,
            retry_base: RETRY_BASE,
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
                if let Some(id) = self.already_downloaded(&url) {
                    let _ = reply.send(id);
                    return;
                }
                let name = filename_from(None, &url);
                let category = Category::from_name(&name);
                let _ = reply.send(self.push_item(url, name, category, Kind::Http, referrer, 0));
            }
            Cmd::AddMedia(url, title, format, queue, thumbnail, duration, reply) => {
                if let Some(id) = self.already_downloaded(&url) {
                    let _ = reply.send(id);
                    return;
                }
                let id = self.push_media(url, title, format, queue);
                self.set_meta(id, thumbnail, duration);
                let _ = reply.send(id);
            }
            Cmd::Refresh(id, url) => {
                let web = url.starts_with("http://") || url.starts_with("https://");
                if !web || self.running.contains_key(&id) {
                    return;
                }
                if let Some(item) = self.state.item_mut(id).filter(|i| i.status != Status::Done) {
                    item.url = url;
                    item.status = Status::Queued;
                    self.cancel_retry(id);
                    self.dirty = true;
                    self.updated(id);
                    self.schedule();
                }
            }
            Cmd::AddWatch(mut watch) => {
                if self.state.watches.iter().any(|w| w.url == watch.url) {
                    return;
                }
                watch.id = self.state.watches.iter().map(|w| w.id).max().unwrap_or(0) + 1;
                let id = watch.id;
                self.state.watches.push(watch);
                self.watches_changed();
                self.check_watch(id);
            }
            Cmd::RemoveWatch(id) => {
                self.state.watches.retain(|w| w.id != id);
                self.watches_changed();
            }
            Cmd::CheckWatches => {
                let ids: Vec<u32> = self.state.watches.iter().map(|w| w.id).collect();
                ids.into_iter().for_each(|id| self.check_watch(id));
            }
            Cmd::AddTorrent(url, reply) => {
                if let Some(id) = self.already_downloaded(&url) {
                    let _ = reply.send(id);
                    return;
                }
                let name = rdm_torrent::link_name(&url);
                let category = Category::from_name(&name);
                let _ = reply.send(self.push_item(url, name, category, Kind::Torrent, None, 0));
            }
            Cmd::AddGallery(url, reply) => {
                if let Some(id) = self.already_downloaded(&url) {
                    let _ = reply.send(id);
                    return;
                }
                let name = crate::model::gallery_name(&url);
                let _ = reply.send(self.push_item(url, name, Category::Image, Kind::Gallery, None, 0));
            }
            Cmd::AddPage(url, reply) => {
                // Pages change: saving one again is never a duplicate.
                let host = url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| "Web page".into());
                let _ = reply.send(self.push_item(url, host, Category::Document, Kind::Page, None, 0));
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
                } else if let Some(item) = self.state.item_mut(id).filter(|i| i.status == Status::Queued || i.retry_at.is_some()) {
                    // Also a failed item waiting for an automatic retry: the retry is called off.
                    item.status = Status::Paused;
                    self.cancel_retry(id);
                    self.updated(id);
                }
            }
            Cmd::Resume(id) => {
                if self.running.contains_key(&id) {
                    return;
                }
                if let Some(item) = self.state.item_mut(id).filter(|i| matches!(i.status, Status::Paused | Status::Failed(_))) {
                    item.status = Status::Queued;
                    self.cancel_retry(id);
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
                    self.cancel_retry(id);
                    self.updated(id);
                    self.schedule();
                }
            }
            Cmd::Remove(id, delete_file) => {
                self.cancel_retry(id);
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

    /// A finished download of the same link whose file is still on disk.
    fn done_copy(&self, url: &str) -> Option<&Item> {
        let wanted = crate::dupe::normalize(url);
        self.state
            .items
            .iter()
            .filter(|i| i.status == Status::Done && i.dest.as_ref().is_some_and(|d| d.exists()))
            .find(|i| crate::dupe::normalize(&i.url) == wanted)
    }

    /// `done_copy`, and if there is one, tells the user (nothing gets added).
    fn already_downloaded(&mut self, url: &str) -> Option<ItemId> {
        let item = self.done_copy(url)?.clone();
        let id = item.id;
        self.emit(Event::Duplicate(item));
        Some(id)
    }

    fn push_media(&mut self, url: String, title: String, format: MediaFormat, queue: QueueId) -> ItemId {
        let category = if format == MediaFormat::AudioMp3 { Category::Music } else { Category::Video };
        let queue = if self.state.queue(queue).is_some() { queue } else { 0 };
        let referrer = self.referrers.lock().ok().and_then(|r| r.get(&url).cloned());
        self.push_item(url, title, category, Kind::Media(format), referrer, queue)
    }

    /// Records a video's preview and length. Every recorded thumbnail comes through here, so only
    /// web ones are kept (see `is_web_link`).
    fn set_meta(&mut self, id: ItemId, thumbnail: Option<String>, duration: Option<f64>) {
        let thumbnail = thumbnail.filter(|t| crate::model::is_web_link(t));
        if thumbnail.is_none() && duration.is_none() {
            return;
        }
        if let Some(item) = self.state.item_mut(id) {
            item.thumbnail = thumbnail;
            item.duration = duration;
            self.dirty = true;
        }
        self.updated(id);
    }

    fn watches_changed(&mut self) {
        self.dirty = true;
        self.emit(Event::Watches(self.state.watches.clone()));
    }

    /// Reads a watched channel/playlist in the background (once at a time per watch).
    fn check_watch(&mut self, id: u32) {
        let Some(url) = self.state.watches.iter().find(|w| w.id == id).map(|w| w.url.clone()) else { return };
        if !self.watching.insert(id) {
            return;
        }
        let (tools, msg_tx) = (self.tools.clone(), self.msg_tx.clone());
        tokio::spawn(async move {
            let listed = async {
                let ytdlp = tools.ytdlp().await?;
                Ok(rdm_media::probe(&ytdlp, &url, None, None).await?.entries)
            }
            .await;
            let _ = msg_tx.send(Msg::WatchListed(id, listed));
        });
    }

    /// Calls off a pending automatic retry and starts the count afresh (the user acted).
    fn cancel_retry(&mut self, id: ItemId) {
        self.retry_gen.remove(&id);
        self.retries.remove(&id);
        if let Some(item) = self.state.item_mut(id) {
            item.retry_at = None;
        }
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
        if let [(url, ..)] = requests.as_slice() {
            // A single video already on disk: the duplicate warning instead of "Added".
            if self.already_downloaded(url).is_some() {
                return;
            }
        }
        let (mut count, mut skipped) = (0, 0);
        for ((url, title, format), (thumbnail, duration)) in requests.into_iter().zip(info.request_meta()) {
            // In a playlist, videos downloaded before are skipped quietly (counted below).
            if self.done_copy(&url).is_some() {
                skipped += 1;
                continue;
            }
            let id = self.push_media(url, title, format, 0);
            self.set_meta(id, thumbnail, duration);
            count += 1;
        }
        let what = if count == 1 { format!("\u{201c}{}\u{201d}", info.title) } else { format!("{count} videos") };
        let skipped = if skipped > 0 { format!(", {skipped} already downloaded") } else { String::new() };
        self.emit(Event::Notice(format!("Added {what} ({label}){skipped}")));
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
            let media = self.state.item(*id).is_some_and(|i| matches!(i.kind, Kind::Media(_) | Kind::Gallery | Kind::Torrent));
            if media && r.stop == Stop::None {
                r.stop = Stop::Schedule; // comes back as Queued and starts again
                r.cancel.cancel();
            }
        }
    }

    fn push_item(&mut self, url: String, name: String, category: Category, kind: Kind, referrer: Option<String>, queue: QueueId) -> ItemId {
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
            queue,
            added: chrono::Utc::now().timestamp(),
            kind,
            referrer,
            work_dir: None,
            thumbnail: None,
            duration: None,
            retry_at: None,
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
        let picked = pick_next(&self.state, &running, now);
        // Every download gets an equal share of the speed limit. Videos with a bigger share
        // than that are restarted with the smaller one (yt-dlp continues its partial files).
        let limit = self.state.settings.speed_limit_bps;
        let share = if limit == 0 { 0 } else { (limit / (running.len() + picked.len()).max(1) as u64).max(1) };
        if limit > 0 {
            for (id, r) in self.running.iter_mut() {
                let video = self.state.item(*id).is_some_and(|i| matches!(i.kind, Kind::Media(_) | Kind::Gallery | Kind::Torrent));
                if video && r.stop == Stop::None && (r.limit_bps == 0 || r.limit_bps > share) {
                    r.stop = Stop::Schedule;
                    r.cancel.cancel();
                }
            }
        }
        for id in picked {
            self.start(id, share);
        }
        self.balance_http_limit();
    }

    /// File downloads share what the videos leave of the speed limit.
    fn balance_http_limit(&self) {
        let limit = self.state.settings.speed_limit_bps;
        if limit == 0 {
            self.limiter.set_limit(0);
            return;
        }
        let videos: u64 = self.running.values().map(|r| r.limit_bps).sum();
        // While videos restart with smaller shares the sum can briefly exceed the limit; never set 0 (= unlimited).
        let floor = limit / self.running.len().max(1) as u64;
        self.limiter.set_limit(limit.saturating_sub(videos).max(floor).max(1));
    }

    /// `share`: this download's part of the speed limit (0 = no limit).
    fn start(&mut self, id: ItemId, share: u64) {
        let settings = self.state.settings.clone();
        let Some(item) = self.state.item_mut(id) else { return };
        item.status = Status::Running;
        item.speed_bps = 0;
        if let Kind::Media(format) = &item.kind
            && item.work_dir.is_none()
        {
            item.work_dir = Some(media_dir(&settings, format).join(PARTS_DIR).join(id.0.to_string()));
        }
        let (url, existing_dest, kind, counted) = (item.url.clone(), item.dest.clone(), item.kind.clone(), item.downloaded);
        let (referrer, work_dir) = (item.referrer.clone(), item.work_dir.clone());
        self.updated(id);

        let cancel = CancellationToken::new();
        let (progress_tx, progress_rx) = watch::channel(Progress::default());
        let video = matches!(kind, Kind::Media(_) | Kind::Gallery | Kind::Torrent);
        let limit_bps = if video { share } else { 0 };
        self.running.insert(id, Running { cancel: cancel.clone(), progress: progress_rx, stop: Stop::None, limit_bps, counted, segments: Vec::new() });
        if let Kind::Media(format) = kind {
            self.start_media(id, url, format, work_dir, share, cancel, progress_tx);
            return;
        }
        if kind == Kind::Gallery {
            self.start_gallery(id, url, share, cancel, progress_tx);
            return;
        }
        if kind == Kind::Torrent {
            self.start_torrent(id, url, work_dir, share, cancel, progress_tx);
            return;
        }
        if kind == Kind::Page {
            self.start_page(id, url, cancel, progress_tx);
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

    #[allow(clippy::too_many_arguments)]
    fn start_media(&mut self, id: ItemId, url: String, format: MediaFormat, work_dir: Option<PathBuf>, limit_bps: u64, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let dir = media_dir(settings, &format);
        let cookies = cookie_file(&self.jar, &self.cookie_dir, &id.0.to_string(), &url);
        let subtitles = (settings.subtitles && !settings.subtitle_langs.trim().is_empty()).then(|| settings.subtitle_langs.trim().to_string());
        let referer = self.state.item(id).and_then(|i| i.referrer.clone());
        let opts = MediaOptions { cookies: cookies.clone(), limit_bps, temp_dir: work_dir, subtitles, referer };
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

    /// A page is fetched (with the browser's cookies) and packed into one .html file named after
    /// its title, in `<download>/Pages`.
    fn start_page(&mut self, id: ItemId, url: String, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let dir = if settings.sort_into_folders { settings.download_dir.join("Pages") } else { settings.download_dir.clone() };
        let cookies = self.jar.lock().ok().and_then(|j| j.netscape(&url)).and_then(|text| monolith::cookies::parse_cookie_file_contents(&text).ok());
        let msg_tx = self.msg_tx.clone();
        tokio::spawn(async move {
            let save = tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
                let options = monolith::core::MonolithOptions { silent: true, ignore_errors: true, timeout: 30, user_agent: Some(BROWSER_UA.into()), ..Default::default() };
                let session = monolith::session::Session::new(None, cookies, options);
                let (html, title) = monolith::core::create_monolithic_document(session, url.clone()).map_err(|e| format!("couldn't save the page: {e}"))?;
                std::fs::create_dir_all(&dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
                let name = format!("{}.html", page_file_name(title.as_deref(), &url));
                let path = rdm_engine::filename::unique_path(&dir, &name);
                std::fs::write(&path, &html).map_err(|e| format!("can't write {}: {e}", path.display()))?;
                Ok(path)
            });
            let result = tokio::select! {
                // The page can't be interrupted half-way; a pause just stops waiting for it.
                _ = cancel.cancelled() => Ok(Outcome::Paused),
                saved = save => match saved {
                    Ok(Ok(path)) => {
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        progress_tx.send_replace(Progress { downloaded: size, total: Some(size), speed_bps: 0, segments: Vec::new() });
                        Ok(Outcome::Completed(path))
                    }
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(format!("couldn't save the page: {e}")),
                },
            };
            let _ = msg_tx.send(Msg::Finished { id, result });
        });
    }

    /// Galleries go through gallery-dl into `<Images>/<name>`; existing images are skipped, so
    /// a paused or restarted gallery continues.
    fn start_gallery(&mut self, id: ItemId, url: String, limit_bps: u64, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let base = if settings.sort_into_folders { settings.download_dir.join(Category::Image.folder()) } else { settings.download_dir.clone() };
        let name = self.state.item(id).map(|i| i.name.clone()).unwrap_or_else(|| crate::model::gallery_name(&url));
        let dir = base.join(name);
        let cookies = cookie_file(&self.jar, &self.cookie_dir, &id.0.to_string(), &url);
        let opts = MediaOptions { cookies: cookies.clone(), limit_bps, ..Default::default() };
        let (tools, msg_tx) = (self.tools.clone(), self.msg_tx.clone());
        tokio::spawn(async move {
            let result = async {
                let exe = tokio::select! {
                    _ = cancel.cancelled() => return Ok(Outcome::Paused),
                    path = tools.gallery_dl() => path?,
                };
                let (media_tx, mut media_rx) = watch::channel(MediaProgress::default());
                let forward = tokio::spawn(async move {
                    while media_rx.changed().await.is_ok() {
                        let p = media_rx.borrow_and_update().clone();
                        progress_tx.send_replace(Progress { downloaded: p.downloaded, total: p.total, speed_bps: p.speed_bps, segments: Vec::new() });
                    }
                });
                let outcome = rdm_media::gallery::download(&exe, &url, &dir, &opts, cancel, &media_tx).await;
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

    /// Torrents go to `<download>/Torrents` (a folder per multi-file torrent). Stopping pauses
    /// them; starting again continues from the pieces already on disk.
    /// `claimed`: the folder this torrent started in before (kept in `work_dir`), if any.
    fn start_torrent(&mut self, id: ItemId, url: String, claimed: Option<PathBuf>, limit_bps: u64, cancel: CancellationToken, progress_tx: watch::Sender<Progress>) {
        let settings = &self.state.settings;
        let base = if settings.sort_into_folders { settings.download_dir.join("Torrents") } else { settings.download_dir.clone() };
        let data_dir = self.path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let keep_sharing = settings.keep_sharing;
        let (torrents, client, msg_tx) = (self.torrents.clone(), self.client.clone(), self.msg_tx.clone());
        tokio::spawn(async move {
            let result = async {
                let source = if url.starts_with("magnet:") {
                    rdm_torrent::Source::Magnet(url.clone())
                } else {
                    let get = client.get(&url).send();
                    let bytes = tokio::select! {
                        _ = cancel.cancelled() => return Ok(Outcome::Paused),
                        r = get => r.and_then(|r| r.error_for_status()).map_err(|e| format!("couldn't fetch the .torrent: {e}"))?.bytes().await.map_err(|e| e.to_string())?,
                    };
                    rdm_torrent::Source::File(bytes.to_vec())
                };
                let session_dir = data_dir.join("torrents");
                let engine = tokio::select! {
                    _ = cancel.cancelled() => return Ok(Outcome::Paused),
                    e = torrents.engine(&session_dir) => e?,
                };
                engine.set_limit(limit_bps);
                let (tx, mut rx) = watch::channel(rdm_torrent::Progress::default());
                let folder_tx = msg_tx.clone();
                let forward = tokio::spawn(async move {
                    let mut told = None;
                    while rx.changed().await.is_ok() {
                        let p = rx.borrow_and_update().clone();
                        if p.folder.is_some() && p.folder != told {
                            told = p.folder.clone();
                            let _ = folder_tx.send(Msg::TorrentFolder(id, p.folder.clone().unwrap_or_default()));
                        }
                        let total = (p.total > 0).then_some(p.total);
                        progress_tx.send_replace(Progress { downloaded: p.downloaded, total, speed_bps: p.speed_bps, segments: Vec::new() });
                    }
                });
                let outcome = engine.download(&url, &source, &base, claimed.as_deref(), cancel, &tx).await;
                if keep_sharing && matches!(outcome, Ok(rdm_torrent::Outcome::Completed(_))) {
                    let _ = engine.share(&url, &source, &tx).await;
                }
                drop(tx);
                let _ = forward.await;
                Ok(match outcome? {
                    rdm_torrent::Outcome::Completed(path) => Outcome::Completed(path),
                    rdm_torrent::Outcome::Paused => Outcome::Paused,
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
            Msg::WatchListed(id, listed) => {
                self.watching.remove(&id);
                let now = chrono::Utc::now().timestamp();
                let Some(watch) = self.state.watches.iter_mut().find(|w| w.id == id) else { return };
                let new = match listed {
                    Ok(entries) => watch.take_new(&entries, now),
                    Err(_) => {
                        // Try again at the next due time.
                        watch.last_check = now;
                        Vec::new()
                    }
                };
                let (format, queue) = (watch.format.clone(), watch.queue);
                for e in new {
                    let id = self.push_media(e.url, e.title, format.clone(), queue);
                    self.set_meta(id, e.thumbnail, e.duration);
                }
                self.watches_changed();
            }
            Msg::TorrentFolder(id, folder) => {
                if let Some(item) = self.state.item_mut(id).filter(|i| i.kind == Kind::Torrent) {
                    item.work_dir = Some(folder);
                    self.dirty = true;
                }
            }
            Msg::RuleDone(id, outcome) => {
                let Some(item) = self.state.item_mut(id) else { return };
                let name = item.name.clone();
                let text = match outcome {
                    Ok(applied) => {
                        if let Some(file) = applied.result.file_name() {
                            item.name = file.to_string_lossy().to_string();
                        }
                        if let Some(size) = std::fs::metadata(&applied.result).ok().filter(|m| m.is_file()).map(|m| m.len()) {
                            item.downloaded = size;
                            item.total = Some(size);
                            // An MP4 turned MP3 is music now.
                            item.category = Category::from_name(&item.name);
                        }
                        item.dest = Some(applied.result);
                        self.dirty = true;
                        self.updated(id);
                        format!("{}: {name}", applied.note)
                    }
                    Err(e) => format!("A rule couldn't finish for {name} (the download is kept): {e}"),
                };
                self.emit(Event::Notice(text));
            }
            Msg::Safety(id, verdict) => {
                // Only while the download is still in the list.
                if self.state.item(id).is_some() {
                    self.state.safety.insert(id, verdict.clone());
                    self.dirty = true;
                    self.emit(Event::Safety(id, verdict));
                }
            }
            Msg::Retry(id, generation) => {
                // Only this item's latest timer, and only if nothing was done to it meanwhile.
                if self.retry_gen.get(&id) != Some(&generation) || self.running.contains_key(&id) {
                    return;
                }
                self.retry_gen.remove(&id);
                if let Some(item) = self.state.item_mut(id).filter(|i| i.retry_at.is_some()) {
                    item.retry_at = None;
                    item.status = Status::Queued;
                    self.updated(id);
                    self.schedule();
                }
            }
            Msg::Finished { id, result } => {
                let Some(mut r) = self.running.remove(&id) else { return };
                if let Stop::Remove { delete_file } = r.stop {
                    self.remove_now(id, delete_file);
                    self.schedule();
                    return;
                }
                let last = r.progress.borrow().clone();
                // The bytes since the last tick.
                let new = r.newly_downloaded(last.downloaded);
                self.count_today(new, 0);
                let auto_retry = self.state.settings.auto_retry;
                let vt_key = self.state.settings.virustotal_key.clone();
                let safety_tx = self.msg_tx.clone();
                let rules = self.state.settings.rules.clone();
                let ffmpeg = self.tools.ffmpeg();
                let used = self.retries.get(&id).copied().unwrap_or(0);
                if let Some(item) = self.state.item_mut(id) {
                    item.speed_bps = 0;
                    item.downloaded = last.downloaded.max(item.downloaded);
                    if last.total.is_some() {
                        item.total = last.total;
                    }
                    item.status = match result {
                        Ok(Outcome::Completed(path)) => {
                            // The real size: video downloads report progress per stream; a gallery is a folder.
                            let size = if path.is_dir() { dir_size(&path) } else { std::fs::metadata(&path).map(|m| m.len()).ok() };
                            if let Some(size) = size {
                                item.downloaded = size;
                                item.total = Some(size);
                            }
                            if item.kind == Kind::Page
                                && let Some(name) = path.file_name()
                            {
                                item.name = name.to_string_lossy().to_string();
                            }
                            if item.kind == Kind::Torrent
                                && let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string())
                            {
                                item.category = if path.is_dir() { Category::Other } else { Category::from_name(&name) };
                                item.name = name;
                            }
                            item.dest = Some(path);
                            Status::Done
                        }
                        Ok(Outcome::Paused) if r.stop == Stop::Schedule => Status::Queued,
                        Ok(Outcome::Paused) => Status::Paused,
                        // Only a download that stopped by itself (not paused, removed or shutting down).
                        Err(e) if r.stop == Stop::None && auto_retry && used < AUTO_RETRIES && is_transient(&e) => {
                            let wait = self.retry_base * 3u32.pow(used);
                            self.retries.insert(id, used + 1);
                            self.next_gen += 1;
                            let generation = self.next_gen;
                            self.retry_gen.insert(id, generation);
                            item.retry_at = Some(chrono::Utc::now().timestamp() + wait.as_secs().max(1) as i64);
                            let msg_tx = self.msg_tx.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(wait).await;
                                let _ = msg_tx.send(Msg::Retry(id, generation));
                            });
                            Status::Failed(e)
                        }
                        Err(e) => Status::Failed(e),
                    };
                    if item.status == Status::Done {
                        self.retries.remove(&id);
                        // Programs get a safety check (hash only), if the user set a VirusTotal key.
                        // A matching after-download rule (files only, not galleries or torrent folders).
                        if let Some(path) = item.dest.clone().filter(|p| p.is_file())
                            && let Some(rule) = crate::rules::matching(&rules, &item.url, &path, item.category).cloned()
                        {
                            let rule_tx = safety_tx.clone();
                            tokio::task::spawn_blocking(move || {
                                let _ = rule_tx.send(Msg::RuleDone(id, crate::rules::apply(&rule, &path, ffmpeg.as_deref())));
                            });
                        }
                        if let Some(path) = item.dest.clone().filter(|p| !vt_key.trim().is_empty() && crate::safety::worth_checking(p)) {
                            tokio::spawn(async move {
                                if let Some(verdict) = crate::safety::check(&path, &vt_key).await {
                                    let _ = safety_tx.send(Msg::Safety(id, verdict));
                                }
                            });
                        }
                    }
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
        self.state.safety.remove(&id);
        if let Some(dir) = item.work_dir.clone() {
            remove_parts(dir);
        }
        let shared = item.dest.as_ref().is_some_and(|d| self.state.items.iter().any(|other| other.dest.as_ref() == Some(d)));
        let trash_it = item.dest.clone().filter(|d| delete_file && !shared && d.exists());
        if let Some(dest) = &item.dest {
            let _ = std::fs::remove_file(part_path(dest));
            let _ = std::fs::remove_file(state_path(dest));
        }
        // A torrent is stopped and forgotten first (it holds its files open, and must not keep
        // sharing); then its files can go.
        let engine = (item.kind == Kind::Torrent).then(|| self.torrents.engine.get().cloned()).flatten();
        if engine.is_some() || trash_it.is_some() {
            let (events, url, name) = (self.events.clone(), item.url.clone(), item.name.clone());
            tokio::spawn(async move {
                if let Some(engine) = engine {
                    engine.forget(&url).await;
                }
                if let Some(dest) = trash_it {
                    // Recycle Bin, never a permanent delete: a misclick must be undoable.
                    let moved = tokio::task::spawn_blocking(move || trash::delete(&dest)).await;
                    if !matches!(moved, Ok(Ok(()))) {
                        let why = match moved {
                            Ok(Err(e)) => e.to_string(),
                            _ => "it is in use".into(),
                        };
                        let _ = events.send(Event::Notice(format!("Couldn't move {name} to the Recycle Bin: {why}")));
                    }
                }
            });
        }
        self.emit(Event::Removed(id));
        self.dirty = true;
    }

    fn on_tick(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let due: Vec<u32> = self.state.watches.iter().filter(|w| w.due(now)).map(|w| w.id).collect();
        due.into_iter().for_each(|id| self.check_watch(id));
        let mut changed = Vec::new();
        let (mut new_bytes, mut segments) = (0, Vec::new());
        for (id, r) in &mut self.running {
            let p = r.progress.borrow().clone();
            new_bytes += r.newly_downloaded(p.downloaded);
            if p.segments != r.segments {
                r.segments = p.segments.clone();
                segments.push((*id, p.segments.clone()));
            }
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
        for (id, segs) in segments {
            self.emit(Event::Segments(id, segs));
        }
        // Time counts only while bytes arrive (a stalled download isn't "downloading").
        self.count_today(new_bytes, if new_bytes > 0 { TICK.as_millis() as u64 } else { 0 });
        self.schedule();
        if self.dirty && self.last_save.elapsed() >= SAVE_EVERY {
            self.save();
        }
    }

    /// Adds to today's downloaded total. It is saved with the next save (a finish, a pause, quitting);
    /// progress alone doesn't make the list dirty, so a long download doesn't rewrite it every second.
    fn count_today(&mut self, bytes: u64, ms: u64) {
        let day = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.state.count_download(&day, bytes, ms);
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

#[cfg(test)]
mod retry_tests {
    use super::is_transient;

    #[test]
    fn transient_errors_are_recognised() {
        for e in ["HTTP Error 503: Service Unavailable", "operation timed out", "connection reset by peer", "error sending request: connection closed", "HTTP Error 429: Too Many Requests", "server returned 502", "dns error: no such host is known"] {
            assert!(is_transient(e), "{e}");
        }
        for e in ["HTTP Error 403: Forbidden", "HTTP Error 404: Not Found", "Unsupported URL: x", "There is no video in this post", "not enough space on the disk"] {
            assert!(!is_transient(e), "{e}");
        }
    }
}
