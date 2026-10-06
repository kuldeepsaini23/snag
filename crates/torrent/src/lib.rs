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
        Ok(Self { session, peers: opts.peers })
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

    /// Downloads into `base` (a multi-file torrent gets a folder of its own there) until done,
    /// then stops sharing; or until `cancel` fires (paused: adding it again continues it).
    pub async fn download(&self, source: &Source, base: &Path, cancel: CancellationToken, progress: &watch::Sender<Progress>) -> Result<Outcome, String> {
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
                let folder = match (&name, files.len()) {
                    (Some(n), 2..) if safe_component(n) => base.join(n),
                    (_, 2..) => base.join("torrent"),
                    _ => base.to_path_buf(),
                };
                (folder, name, files)
            }
            _ => (base.to_path_buf(), None, Vec::new()),
        };
        let output = match files.as_slice() {
            [single] => folder.join(single),
            _ => folder.clone(),
        };
        progress.send_modify(|p| p.name = name.clone());

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
    pub async fn share(&self, source: &Source, _progress: &watch::Sender<Progress>) -> Result<(), String> {
        let handle = self
            .session
            .add_torrent(Self::add_request(source), Some(AddTorrentOptions { overwrite: true, ..Default::default() }))
            .await
            .map_err(|e| format!("{e:#}"))?
            .into_handle()
            .ok_or("the torrent couldn't be added")?;
        if handle.is_paused() {
            self.session.unpause(&handle).await.map_err(|e| format!("{e:#}"))?;
        }
        Ok(())
    }

    /// Forgets a torrent (its files stay; Snag handles deleting them).
    pub async fn forget(&self, source: &Source) {
        let Ok(AddTorrentResponse::AlreadyManaged(id, _)) = self.session.add_torrent(Self::add_request(source), Some(AddTorrentOptions { paused: true, ..Default::default() })).await else {
            return;
        };
        let _ = self.session.delete(id.into(), false).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
