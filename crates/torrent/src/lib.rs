//! Torrents and magnet links (librqbit). Each torrent is checked before anything is written:
//! every file must stay inside the download folder.

use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, ListenerMode, ListenerOptions, Session, SessionOptions};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: u64,
    /// Known once the torrent's metadata arrived.
    pub name: Option<String>,
    /// The folder this torrent writes into (see `placement`); kept so a resume uses it again.
    pub folder: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The downloaded file (single-file torrent) or folder.
    Completed(PathBuf),
    Paused,
}

/// Where a torrent comes from.
pub enum Source {
    Magnet(String),
    /// A `.torrent` file's bytes.
    File(Vec<u8>),
}

/// A magnet link or a link to a `.torrent` file.
pub fn is_torrent_link(url: &str) -> bool {
    let url = url.trim();
    url.starts_with("magnet:?") && url.contains("xt=urn:bt") || (url.starts_with("http") && url.split(['?', '#']).next().is_some_and(|p| p.to_ascii_lowercase().ends_with(".torrent")))
}

/// A display name before the torrent's metadata arrives: the magnet's `dn`, or the `.torrent`
/// file's name.
pub fn link_name(url: &str) -> String {
    let url = url.trim();
    if let Some(query) = url.strip_prefix("magnet:?") {
        for pair in query.split('&') {
            if let Some(v) = pair.strip_prefix("dn=") {
                let v = v.replace('+', " ");
                let bytes = v.as_bytes();
                let mut out = Vec::with_capacity(bytes.len());
                let mut i = 0;
                while i < bytes.len() {
                    let hex = |c: u8| (c as char).to_digit(16);
                    if bytes[i] == b'%'
                        && i + 2 < bytes.len()
                        && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
                    {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                        continue;
                    }
                    out.push(bytes[i]);
                    i += 1;
                }
                let name = String::from_utf8_lossy(&out).trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }
        return "Torrent".into();
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or(path);
    let name = last.strip_suffix(".torrent").or_else(|| last.strip_suffix(".TORRENT")).unwrap_or(last);
    if name.is_empty() { "Torrent".into() } else { name.to_string() }
}

/// One part of a file path inside a torrent is safe on Windows: no traversal, no drive or
/// stream (`:`), no reserved device names, nothing Windows would silently rewrite.
pub fn safe_component(part: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
        "LPT8", "LPT9",
    ];
    let stem = part.split('.').next().unwrap_or(part).trim_end().to_ascii_uppercase();
    !part.is_empty()
        && part != "."
        && part != ".."
        && !part.contains(['/', '\\', ':', '<', '>', '"', '|', '?', '*'])
        && !part.chars().any(char::is_control)
        && !part.ends_with(['.', ' '])
        && !RESERVED.contains(&stem.as_str())
}

/// Where a torrent's files go: never onto files or folders that already exist. A single file
/// goes into `base` when its name is free, else into a free folder `<stem> (n)`; several files
/// get a free folder named after the torrent.
pub fn placement(base: &Path, name: Option<&str>, files: &[PathBuf], exists: impl Fn(&Path) -> bool) -> PathBuf {
    let free_folder = |stem: &str| (1..).map(|n| if n == 1 { base.join(stem) } else { base.join(format!("{stem} ({n})")) }).find(|p| !exists(p)).expect("a free name");
    match files {
        [] => base.to_path_buf(),
        [single] if !exists(&base.join(single)) => base.to_path_buf(),
        [single] => {
            let stem = single.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "torrent".into());
            (2..).map(|n| base.join(format!("{stem} ({n})"))).find(|p| !exists(p)).expect("a free name")
        }
        _ => free_folder(name.filter(|n| safe_component(n)).unwrap_or("torrent")),
    }
}

#[derive(Default)]
pub struct EngineOptions {
    /// Turn off DHT and trackers, and listen on localhost only (tests).
    pub local_only: bool,
    /// Peers to try straight away (tests).
    pub peers: Vec<SocketAddr>,
}

#[derive(Clone)]
pub struct TorrentEngine {
    session: Arc<Session>,
    peers: Vec<SocketAddr>,
    /// Torrents in the session by the caller's key (Snag uses the item's link), so `forget`
    /// finds exactly the right one without adding anything.
    loaded: Arc<std::sync::Mutex<std::collections::HashMap<String, usize>>>,
}

impl TorrentEngine {
    /// `dir`: where torrents are saved (multi-file torrents get a folder of their own there).
    pub async fn start(dir: &Path, opts: EngineOptions) -> Result<Self, String> {
        let mut session_opts = SessionOptions::default();
        if opts.local_only {
            session_opts.dht = None;
            session_opts.disable_trackers = true;
            session_opts.disable_local_service_discovery = true;
            session_opts.listen = Some(ListenerOptions { mode: ListenerMode::TcpOnly, listen_addr: (std::net::Ipv4Addr::LOCALHOST, 0).into(), ..Default::default() });
        } else {
            session_opts.listen = Some(ListenerOptions::default());
        }
        let session = Session::new_with_opts(dir.to_path_buf(), session_opts).await.map_err(|e| format!("couldn't start torrents: {e:#}"))?;
        Ok(Self { session, peers: opts.peers, loaded: Arc::default() })
    }

    /// The port peers reach us on.
    pub fn port(&self) -> Option<u16> {
        self.session.listen_addr().map(|a| a.port())
    }

    /// Bytes per second for all torrents together; 0 = no limit.
    pub fn set_limit(&self, bps: u64) {
        let limit = NonZeroU32::new(bps.min(u32::MAX as u64) as u32);
        self.session.ratelimits.set_download_bps(limit);
    }

    fn add_request(source: &Source) -> AddTorrent<'static> {
        match source {
            Source::Magnet(m) => AddTorrent::from_url(m.clone()),
            Source::File(bytes) => AddTorrent::from_bytes(bytes.clone()),
        }
    }

    /// Downloads into a place under `base` that holds nothing else (`placement`), or into
    /// `claimed` when this torrent already started there; until done, then stops sharing; or
    /// until `cancel` fires (paused: adding it again with its claimed folder continues it).
    /// `key` names it for `forget` (Snag passes the item's link).
    pub async fn download(&self, key: &str, source: &Source, base: &Path, claimed: Option<&Path>, cancel: CancellationToken, progress: &watch::Sender<Progress>) -> Result<Outcome, String> {
        let initial_peers = (!self.peers.is_empty()).then(|| self.peers.clone());
        // First only read its file list: refuse anything that would write outside the folder.
        let listing = tokio::select! {
            _ = cancel.cancelled() => return Ok(Outcome::Paused),
            r = self.session.add_torrent(Self::add_request(source), Some(AddTorrentOptions { list_only: true, initial_peers: initial_peers.clone(), ..Default::default() })) => r,
        }
        .map_err(|e| format!("couldn't read the torrent: {e:#}"))?;
        let (folder, name, files) = match &listing {
            AddTorrentResponse::ListOnly(list) => {
                let files: Vec<PathBuf> = list.info.iter_file_details().map(|f| f.filename.to_pathbuf()).collect();
                for file in &files {
                    let ok = file.components().all(|c| matches!(c, std::path::Component::Normal(p) if safe_component(&p.to_string_lossy())));
                    if !ok {
                        return Err(format!("unsafe file name in torrent: {}", file.display()));
                    }
                }
                let name = list.info.name().map(|n| n.to_string());
                // Never onto existing files: an earlier download, or the user's own.
                let folder = claimed.map(Path::to_path_buf).unwrap_or_else(|| placement(base, name.as_deref(), &files, |p| p.exists()));
                (folder, name, files)
            }
            _ => (claimed.unwrap_or(base).to_path_buf(), None, Vec::new()),
        };
        let output = match files.as_slice() {
            [single] => folder.join(single),
            _ => folder.clone(),
        };
        progress.send_modify(|p| {
            p.name = name.clone();
            p.folder = Some(folder.clone());
        });

        let handle = self
            .session
            .add_torrent(
                Self::add_request(source),
                Some(AddTorrentOptions { initial_peers, overwrite: true, output_folder: Some(folder.display().to_string()), ..Default::default() }),
            )
            .await
            .map_err(|e| format!("couldn't start the torrent: {e:#}"))?
            .into_handle()
            .ok_or("the torrent couldn't be added")?;
        self.remember(key, handle.id());
        if handle.is_paused() {
            self.session.unpause(&handle).await.map_err(|e| format!("{e:#}"))?;
        }
        loop {
            let stats = handle.stats();
            if let Some(e) = stats.error {
                return Err(e);
            }
            let speed = stats.live.as_ref().map_or(0.0, |l| l.download_speed.mbps) * 1024.0 * 1024.0;
            progress.send_modify(|p| {
                p.downloaded = stats.progress_bytes;
                p.total = stats.total_bytes;
                p.speed_bps = speed as u64;
            });
            if stats.finished {
                // Done: stop sharing (a download manager, not a seedbox).
                let _ = self.session.pause(&handle).await;
                return Ok(Outcome::Completed(output));
            }
            tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = self.session.pause(&handle).await;
                    return Ok(Outcome::Paused);
                }
                _ = tokio::time::sleep(Duration::from_millis(400)) => {}
            }
        }
    }

    /// Shares a torrent that is already downloaded ("Keep sharing after download").
    pub async fn share(&self, key: &str, source: &Source, _progress: &watch::Sender<Progress>) -> Result<(), String> {
        let handle = self
            .session
            .add_torrent(Self::add_request(source), Some(AddTorrentOptions { overwrite: true, ..Default::default() }))
            .await
            .map_err(|e| format!("{e:#}"))?
            .into_handle()
            .ok_or("the torrent couldn't be added")?;
        self.remember(key, handle.id());
        if handle.is_paused() {
            self.session.unpause(&handle).await.map_err(|e| format!("{e:#}"))?;
        }
        Ok(())
    }

    fn remember(&self, key: &str, id: usize) {
        if let Ok(mut loaded) = self.loaded.lock() {
            loaded.insert(key.to_string(), id);
        }
    }

    /// How many torrents the engine holds (downloading, paused or sharing).
    pub fn session_len(&self) -> usize {
        self.session.with_torrents(|t| t.count())
    }

    /// The torrent under `key` is in the session (downloading, paused or sharing).
    pub fn is_loaded(&self, key: &str) -> bool {
        self.loaded.lock().is_ok_and(|l| l.contains_key(key))
    }

    /// Stops and forgets the torrent under `key`, and returns once its files are released (they
    /// stay on disk; Snag deletes them if asked). Nothing to do if it isn't loaded.
    pub async fn forget(&self, key: &str) {
        let id = self.loaded.lock().ok().and_then(|mut l| l.remove(key));
        if let Some(id) = id {
            let _ = self.session.delete(id.into(), false).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_torrent_never_lands_on_existing_files() {
        let base = Path::new("C:/dl/Torrents");
        let taken = |paths: Vec<PathBuf>| move |p: &Path| paths.iter().any(|t| t == p);
        let one = vec![PathBuf::from("video.mkv")];
        // A single file goes straight into the folder when its name is free…
        assert_eq!(placement(base, Some("video.mkv"), &one, taken(vec![])), base);
        // …and into a folder of its own when a file of that name is already there.
        assert_eq!(placement(base, Some("video.mkv"), &one, taken(vec![base.join("video.mkv")])), base.join("video (2)"));
        assert_eq!(placement(base, Some("video.mkv"), &one, taken(vec![base.join("video.mkv"), base.join("video (2)")])), base.join("video (3)"));
        // Several files: a folder named after the torrent, never one that exists.
        let many = vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")];
        assert_eq!(placement(base, Some("Album"), &many, taken(vec![])), base.join("Album"));
        assert_eq!(placement(base, Some("Album"), &many, taken(vec![base.join("Album")])), base.join("Album (2)"));
        assert_eq!(placement(base, Some("../x"), &many, taken(vec![])), base.join("torrent"), "an unsafe name isn't used");
    }

    #[test]
    fn torrent_links() {
        assert!(is_torrent_link("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x"));
        assert!(is_torrent_link("https://site.org/files/ubuntu.iso.torrent?dl=1"));
        assert!(!is_torrent_link("magnet:?xt=urn:sha1:abc"), "not a BitTorrent magnet");
        assert!(!is_torrent_link("https://site.org/a.zip"));
    }

    #[test]
    fn names_before_metadata() {
        assert_eq!(link_name("magnet:?xt=urn:btih:abc&dn=Ubuntu+24.04%20Desktop&tr=x"), "Ubuntu 24.04 Desktop");
        assert_eq!(link_name("magnet:?xt=urn:btih:abc"), "Torrent");
        assert_eq!(link_name("https://site.org/files/debian-12.iso.torrent?x=1"), "debian-12.iso");
    }

    #[test]
    fn unsafe_names_are_refused() {
        for bad in ["..", ".", "", "C:evil", "a:stream", "CON", "nul.txt", "LPT1", "x\\y", "a/b", "trailing.", "trailing ", "q?", "tab\t"] {
            assert!(!safe_component(bad), "{bad:?}");
        }
        for good in ["Ubuntu 24.04", "file.iso", "Season 1", "con-artist.mkv", "naïve ☕.txt"] {
            assert!(safe_component(good), "{good:?}");
        }
    }
}
