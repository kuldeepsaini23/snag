use crate::info::RemoteInfo;
use crate::segments::Segment;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DownloadState {
    pub url: String,
    pub size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub segments: Vec<Segment>,
}

impl DownloadState {
    pub fn downloaded(&self) -> u64 {
        self.segments.iter().map(|s| s.written).sum()
    }

    pub fn matches(&self, info: &RemoteInfo) -> bool {
        fn same(a: &Option<String>, b: &Option<String>) -> bool {
            a.is_none() || b.is_none() || a == b
        }
        info.size == Some(self.size)
            && same(&self.etag, &info.etag)
            && same(&self.last_modified, &info.last_modified)
    }

    /// Atomic: write a temp file, then rename over the old sidecar.
    pub async fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = path.with_extension("rdmstate.tmp");
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        tokio::fs::write(&tmp, json).await?;
        tokio::fs::rename(&tmp, path).await
    }

    /// Missing or unreadable sidecars mean "no resumable state".
    pub async fn load(path: &Path) -> std::io::Result<Option<Self>> {
        match tokio::fs::read(path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub fn part_path(dest: &Path) -> PathBuf {
    append_ext(dest, "rdmpart")
}

pub fn state_path(dest: &Path) -> PathBuf {
    append_ext(dest, "rdmstate")
}

fn append_ext(path: &Path, ext: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(size: u64, etag: Option<&str>) -> RemoteInfo {
        RemoteInfo {
            url: "http://x/f".into(),
            size: Some(size),
            accepts_ranges: true,
            filename: "f".into(),
            etag: etag.map(String::from),
            last_modified: None,
        }
    }

    fn sample() -> DownloadState {
        let mut segs = vec![Segment::new(0, 50), Segment::new(50, 100)];
        segs[0].written = 10;
        segs[1].written = 5;
        DownloadState { url: "http://x/f".into(), size: 100, etag: Some("\"v1\"".into()), last_modified: None, segments: segs }
    }

    #[test]
    fn downloaded_sums_written() {
        assert_eq!(sample().downloaded(), 15);
    }

    #[test]
    fn matches_checks_size_and_validators() {
        let s = sample();
        assert!(s.matches(&info(100, Some("\"v1\""))));
        assert!(s.matches(&info(100, None)));
        assert!(!s.matches(&info(100, Some("\"v2\""))));
        assert!(!s.matches(&info(99, Some("\"v1\""))));
    }

    #[test]
    fn temp_paths_append_extension() {
        let dest = Path::new(r"C:\dl\movie.mp4");
        assert_eq!(part_path(dest), PathBuf::from(r"C:\dl\movie.mp4.rdmpart"));
        assert_eq!(state_path(dest), PathBuf::from(r"C:\dl\movie.mp4.rdmstate"));
    }

    #[tokio::test]
    async fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.bin.rdmstate");
        sample().save(&p).await.unwrap();
        assert_eq!(DownloadState::load(&p).await.unwrap(), Some(sample()));
        assert!(!dir.path().join("a.bin.rdmstate.tmp").exists());
    }

    #[tokio::test]
    async fn load_missing_or_corrupt_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rdmstate");
        assert_eq!(DownloadState::load(&p).await.unwrap(), None);
        std::fs::write(&p, b"{not json").unwrap();
        assert_eq!(DownloadState::load(&p).await.unwrap(), None);
    }
}
