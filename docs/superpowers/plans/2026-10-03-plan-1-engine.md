# Plan 1 — Segmented Download Engine + CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `rdm-engine`, an IDM-style segmented HTTP downloader with pause/resume, dynamic segment splitting, retries and a speed limit, plus a `rdm-cli` binary that proves it works on real URLs.

**Architecture:** A pure library crate. `probe` learns the size, range support and filename with a single `Range: bytes=0-0` GET. `download` splits the file into segments that are fetched by parallel tokio tasks writing at their offsets into `name.rdmpart`. Progress goes out through a `tokio::sync::watch` channel, and a JSON sidecar `name.rdmstate` makes resume possible. Servers without range support fall back to one sequential stream.

**Tech Stack:** Rust 1.99 stable (MSVC), edition 2024, tokio 1, reqwest 0.12 (rustls, no default features), tokio-util (CancellationToken), serde/serde_json, thiserror 2, percent-encoding 2. Dev: axum 0.8 (local test server), tempfile 3. CLI: clap 4.

**Spec:** `docs/superpowers/specs/2026-10-03-rdm-design.md` (sections 2.1, 4, 5, and build step 1)

## Global Constraints

- Platform: Windows 11, `stable-x86_64-pc-windows-msvc`, Rust 1.99, edition 2024, workspace `resolver = "3"`.
- Workspace root: `C:\Users\kulde\rdm`. Crates live under `crates/`.
- `rdm-engine` must not depend on any UI crate, yt-dlp or `core`. Dependency direction is `app → core → {engine, media}`.
- reqwest uses `default-features = false` with `rustls-tls` and `stream`. **No gzip/brotli**, because compressed transfer breaks byte offsets.
- Default connections per download: **8** (1–16). Default dynamic-split minimum: **1 MiB**. Retries: **5** per segment, exponential backoff from 500 ms.
- Sidecar flush interval: **1 s**. Progress tick: **250 ms**.
- Temp file names: `<dest>.rdmpart`, `<dest>.rdmstate` (already in `.gitignore`).
- Tests never touch the internet. Everything runs against the local axum server in `crates/engine/tests/support/mod.rs`.
- No `unsafe`. No `unwrap()` in library code except on mutex locks and on builder calls that cannot fail.
- Commit after every task with a message ending in `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Server stops honoring `Range` after the probe** (CDNs do this). Expected: `EngineError::RangeNotHonored` and no final file; garbage must never be written at the wrong offsets. → Task 5 test `range_not_honored_fails_cleanly`.
2. **Remote file changed between pause and resume** (ETag differs). Expected: restart from zero, final bytes equal the *new* file. → Task 6 test `etag_change_restarts_from_zero`.
3. **Connection drops mid-segment.** Expected: the segment retries from its current position and the final file is byte-exact. → Task 5 test `retries_dropped_connections`.
4. **Zero-byte file.** Expected: completes with an empty file and no panic on division or ranges. → Task 6 test `zero_byte_file_completes`.
5. **Server-supplied filename contains `<>:"/\|?*`** (illegal on Windows). Expected: sanitized to `_`, never a write error or path traversal. → Task 1 test `replaces_windows_illegal_chars`.

---

## File Structure

```
rdm/
├─ Cargo.toml                         # workspace
├─ crates/
│  ├─ engine/
│  │  ├─ Cargo.toml
│  │  ├─ src/
│  │  │  ├─ lib.rs                    # module list, re-exports, default_client()
│  │  │  ├─ error.rs                  # EngineError
│  │  │  ├─ filename.rs               # filename_from, sanitize, unique_path
│  │  │  ├─ info.rs                   # RemoteInfo
│  │  │  ├─ segments.rs               # Segment, plan, split_largest
│  │  │  ├─ state.rs                  # DownloadState sidecar, part_path/state_path
│  │  │  ├─ limiter.rs                # RateLimiter (token bucket)
│  │  │  ├─ probe.rs                  # probe()
│  │  │  └─ download.rs               # download(), segmented + single-stream
│  │  └─ tests/
│  │     ├─ support/mod.rs            # local axum test server
│  │     ├─ probe.rs
│  │     └─ download.rs
│  └─ cli/
│     ├─ Cargo.toml
│     └─ src/main.rs                  # rdm-cli
```

---

### Task 1: Workspace, engine crate, errors and filenames

**Files:**
- Create: `Cargo.toml`, `crates/engine/Cargo.toml`, `crates/engine/src/lib.rs`, `crates/engine/src/error.rs`, `crates/engine/src/filename.rs`

**Interfaces:**
- Produces:
  - `rdm_engine::EngineError` with variants `Network(String)`, `LinkExpired(u16)`, `Http(u16)`, `RangeNotHonored`, `Disk(std::io::Error)`, `Internal(String)`; `EngineError::from_status(u16) -> EngineError`; `EngineError::is_retryable(&self) -> bool`; `From<reqwest::Error>`, `From<std::io::Error>`.
  - `rdm_engine::filename::{filename_from(content_disposition: Option<&str>, url: &str) -> String, sanitize(&str) -> String, unique_path(dir: &Path, name: &str) -> PathBuf}`.
  - `rdm_engine::default_client() -> reqwest::Client`.

- [ ] **Step 1: Create the workspace manifest**

`Cargo.toml`:
```toml
[workspace]
resolver = "3"
members = ["crates/engine"]

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
percent-encoding = "2"
axum = "0.8"
tempfile = "3"
clap = { version = "4", features = ["derive"] }
```

`crates/engine/Cargo.toml`:
```toml
[package]
name = "rdm-engine"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
tokio-util.workspace = true
reqwest.workspace = true
futures-util.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
percent-encoding.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
axum.workspace = true
tempfile.workspace = true
```

- [ ] **Step 2: Write `error.rs` and `lib.rs`**

`crates/engine/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("network error: {0}")]
    Network(String),
    #[error("link expired or access denied (HTTP {0})")]
    LinkExpired(u16),
    #[error("server returned HTTP {0}")]
    Http(u16),
    #[error("server stopped honoring range requests")]
    RangeNotHonored,
    #[error("disk error: {0}")]
    Disk(#[from] std::io::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for EngineError {
    fn from(e: reqwest::Error) -> Self {
        EngineError::Network(e.to_string())
    }
}

impl EngineError {
    pub fn from_status(status: u16) -> Self {
        match status {
            403 | 410 => Self::LinkExpired(status),
            s => Self::Http(s),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::Http(s) => *s == 429 || *s >= 500,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_status_codes() {
        assert!(matches!(EngineError::from_status(403), EngineError::LinkExpired(403)));
        assert!(matches!(EngineError::from_status(410), EngineError::LinkExpired(410)));
        assert!(matches!(EngineError::from_status(404), EngineError::Http(404)));
    }

    #[test]
    fn only_transient_errors_retry() {
        assert!(EngineError::Network("x".into()).is_retryable());
        assert!(EngineError::Http(503).is_retryable());
        assert!(EngineError::Http(429).is_retryable());
        assert!(!EngineError::Http(404).is_retryable());
        assert!(!EngineError::LinkExpired(403).is_retryable());
        assert!(!EngineError::RangeNotHonored.is_retryable());
    }
}
```

`crates/engine/src/lib.rs`:
```rust
pub mod error;
pub mod filename;

pub use error::EngineError;

use std::time::Duration;

pub fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("RDM/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .expect("static client config is valid")
}
```

- [ ] **Step 3: Write the failing filename tests**

`crates/engine/src/filename.rs` (tests first, functions as `todo!()` stubs):
```rust
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};

pub fn filename_from(content_disposition: Option<&str>, url: &str) -> String {
    todo!()
}

pub fn sanitize(name: &str) -> String {
    todo!()
}

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_rfc5987_filename() {
        let cd = r#"attachment; filename="fallback.zip"; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"#;
        assert_eq!(filename_from(Some(cd), "https://x.com/a"), "résumé.pdf");
    }

    #[test]
    fn uses_quoted_filename() {
        let cd = r#"attachment; filename="report 2026.xlsx""#;
        assert_eq!(filename_from(Some(cd), "https://x.com/a"), "report 2026.xlsx");
    }

    #[test]
    fn falls_back_to_url_path() {
        assert_eq!(
            filename_from(None, "https://cdn.x.com/files/ubuntu%2024.iso?token=abc#frag"),
            "ubuntu 24.iso"
        );
    }

    #[test]
    fn falls_back_to_download_when_nothing() {
        assert_eq!(filename_from(None, "https://x.com/"), "download");
        assert_eq!(filename_from(None, "https://x.com"), "download");
    }

    #[test]
    fn replaces_windows_illegal_chars() {
        let cd = r#"attachment; filename="a<b>:c|d?.mp4""#;
        assert_eq!(filename_from(Some(cd), "https://x.com"), "a_b__c_d_.mp4");
        assert_eq!(sanitize("..\\..\\evil.exe"), ".._.._evil.exe");
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        assert_eq!(sanitize("video. . "), "video");
        assert_eq!(sanitize("   "), "download");
    }

    #[test]
    fn unique_path_appends_counter() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a.txt"));
        std::fs::write(dir.path().join("a.txt"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a (1).txt"));
        std::fs::write(dir.path().join("a (1).txt"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt"), dir.path().join("a (2).txt"));
        std::fs::write(dir.path().join("README"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "README"), dir.path().join("README (1)"));
    }
}
```

- [ ] **Step 4: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine`
Expected: the error tests PASS and the filename tests FAIL with `not yet implemented`.

- [ ] **Step 5: Implement `filename.rs`**

Replace the three stubs:
```rust
pub fn filename_from(content_disposition: Option<&str>, url: &str) -> String {
    let raw = content_disposition
        .and_then(from_content_disposition)
        .or_else(|| from_url(url))
        .unwrap_or_default();
    sanitize(&raw)
}

fn from_content_disposition(cd: &str) -> Option<String> {
    let mut plain = None;
    for part in cd.split(';').map(str::trim) {
        let Some((key, value)) = part.split_once('=') else { continue };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "filename*" {
            // RFC 5987: charset'language'percent-encoded-value
            let encoded = value.splitn(3, '\'').nth(2).unwrap_or(value);
            return Some(percent_decode_str(encoded).decode_utf8_lossy().into_owned());
        }
        if key == "filename" {
            plain = Some(value.trim_matches('"').to_string());
        }
    }
    plain
}

fn from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let after_scheme = path.split_once("://").map_or(path, |(_, rest)| rest);
    let (_, path_part) = after_scheme.split_once('/')?;
    let last = path_part.rsplit('/').next()?;
    if last.is_empty() {
        return None;
    }
    Some(percent_decode_str(last).decode_utf8_lossy().into_owned())
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() { "download".to_string() } else { trimmed.to_string() }
}

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .expect("unbounded counter always finds a free name")
}
```

- [ ] **Step 6: Run the tests and confirm they pass**

Run: `cargo test -p rdm-engine`
Expected: PASS for all 9 tests.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/engine
git commit -m "feat(engine): workspace, error types, filename handling

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Segments, RemoteInfo and the resume sidecar

**Files:**
- Create: `crates/engine/src/info.rs`, `crates/engine/src/segments.rs`, `crates/engine/src/state.rs`
- Modify: `crates/engine/src/lib.rs` (add modules and re-exports)

**Interfaces:**
- Produces:
  - `rdm_engine::RemoteInfo { url: String, size: Option<u64>, accepts_ranges: bool, filename: String, etag: Option<String>, last_modified: Option<String> }` (Clone, Debug, PartialEq).
  - `rdm_engine::segments::Segment { start: u64, end: u64 /* exclusive */, written: u64 }` with `new(start, end)`, `pos()`, `remaining()`; plus `plan(size: u64, n: usize) -> Vec<Segment>` and `split_largest(&mut Vec<Segment>, min_split: u64) -> Option<usize>` (returns the index of the new segment).
  - `rdm_engine::state::DownloadState { url, size: u64, etag, last_modified, segments: Vec<Segment> }` with `downloaded()`, `matches(&RemoteInfo)`, `async save(&Path) -> io::Result<()>`, `async load(&Path) -> io::Result<Option<Self>>`; `part_path(&Path) -> PathBuf`, `state_path(&Path) -> PathBuf`.

- [ ] **Step 1: Write `info.rs` and register the modules**

`crates/engine/src/info.rs`:
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteInfo {
    pub url: String,
    pub size: Option<u64>,
    pub accepts_ranges: bool,
    pub filename: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}
```

`crates/engine/src/lib.rs`: replace the module block with:
```rust
pub mod error;
pub mod filename;
pub mod info;
pub mod segments;
pub mod state;

pub use error::EngineError;
pub use info::RemoteInfo;
```

- [ ] **Step 2: Write the failing segment tests**

`crates/engine/src/segments.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub start: u64,
    /// Exclusive.
    pub end: u64,
    pub written: u64,
}

impl Segment {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end, written: 0 }
    }
    pub fn pos(&self) -> u64 {
        self.start + self.written
    }
    pub fn remaining(&self) -> u64 {
        self.end.saturating_sub(self.pos())
    }
}

pub fn plan(size: u64, n: usize) -> Vec<Segment> {
    todo!()
}

pub fn split_largest(segments: &mut Vec<Segment>, min_split: u64) -> Option<usize> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_covers_whole_file_contiguously() {
        let segs = plan(10, 3);
        assert_eq!(segs, vec![Segment::new(0, 3), Segment::new(3, 6), Segment::new(6, 10)]);
    }

    #[test]
    fn plan_never_makes_more_segments_than_bytes() {
        assert_eq!(plan(2, 8).len(), 2);
        assert_eq!(plan(5, 0), vec![Segment::new(0, 5)]);
        assert!(plan(0, 8).is_empty());
    }

    #[test]
    fn split_halves_remaining_part_of_largest() {
        let mut segs = vec![Segment::new(0, 100), Segment::new(100, 120)];
        segs[0].written = 20;
        let idx = split_largest(&mut segs, 10).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(segs[0].end, 60);
        assert_eq!(segs[2], Segment::new(60, 100));
    }

    #[test]
    fn split_refuses_small_segments() {
        let mut segs = vec![Segment::new(0, 30)];
        assert_eq!(split_largest(&mut segs, 16), None);
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn split_on_empty_list_is_none() {
        assert_eq!(split_largest(&mut Vec::new(), 1), None);
    }
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine segments`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 4: Implement `plan` and `split_largest`**

```rust
pub fn plan(size: u64, n: usize) -> Vec<Segment> {
    if size == 0 {
        return Vec::new();
    }
    let n = (n.max(1) as u64).min(size);
    let base = size / n;
    (0..n)
        .map(|i| {
            let start = i * base;
            let end = if i == n - 1 { size } else { start + base };
            Segment::new(start, end)
        })
        .collect()
}

pub fn split_largest(segments: &mut Vec<Segment>, min_split: u64) -> Option<usize> {
    let (idx, remaining, pos, end) = segments
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.remaining(), s.pos(), s.end))
        .max_by_key(|t| t.1)?;
    if remaining < min_split.max(1) * 2 {
        return None;
    }
    let mid = pos + remaining / 2;
    segments[idx].end = mid;
    segments.push(Segment::new(mid, end));
    Some(segments.len() - 1)
}
```

- [ ] **Step 5: Write the failing state tests**

`crates/engine/src/state.rs`:
```rust
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
        todo!()
    }
    pub fn matches(&self, info: &RemoteInfo) -> bool {
        todo!()
    }
    pub async fn save(&self, path: &Path) -> std::io::Result<()> {
        todo!()
    }
    pub async fn load(path: &Path) -> std::io::Result<Option<Self>> {
        todo!()
    }
}

pub fn part_path(dest: &Path) -> PathBuf {
    todo!()
}

pub fn state_path(dest: &Path) -> PathBuf {
    todo!()
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
```

- [ ] **Step 6: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine state`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 7: Implement `state.rs`**

```rust
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
```

- [ ] **Step 8: Run all engine tests and confirm they pass**

Run: `cargo test -p rdm-engine`
Expected: PASS for all tests (9 from Task 1, 5 segments, 5 state).

- [ ] **Step 9: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): segment planning, dynamic split, resume sidecar

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Token-bucket speed limiter

**Files:**
- Create: `crates/engine/src/limiter.rs`
- Modify: `crates/engine/src/lib.rs`

**Interfaces:**
- Produces: `rdm_engine::RateLimiter` with `new(bytes_per_sec: u64) -> Self` (0 means unlimited), `set_limit(&self, u64)`, `limit(&self) -> u64` and `async acquire(&self, bytes: u64)`. It is shared as `Arc<RateLimiter>`, and later the core crate holds one global instance and calls `set_limit` live.

- [ ] **Step 1: Register the module**

In `lib.rs`, add `pub mod limiter;` after `pub mod info;` and `pub use limiter::RateLimiter;` after the other `pub use` lines.

- [ ] **Step 2: Write the failing tests**

`crates/engine/src/limiter.rs`:
```rust
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{Duration, Instant};

pub struct RateLimiter {
    rate: AtomicU64,
    bucket: Mutex<Bucket>,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    pub fn new(bytes_per_sec: u64) -> Self {
        todo!()
    }
    pub fn set_limit(&self, bytes_per_sec: u64) {
        todo!()
    }
    pub fn limit(&self) -> u64 {
        todo!()
    }
    pub async fn acquire(&self, bytes: u64) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn unlimited_never_waits() {
        let l = RateLimiter::new(0);
        let t = Instant::now();
        l.acquire(100 * 1024 * 1024).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn allows_one_second_burst_then_throttles() {
        let l = RateLimiter::new(1000);
        let t = Instant::now();
        l.acquire(1000).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
        l.acquire(2000).await;
        let e = t.elapsed().as_secs_f64();
        assert!((1.99..=2.01).contains(&e), "elapsed {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_share_the_budget() {
        let l = std::sync::Arc::new(RateLimiter::new(1000));
        l.acquire(1000).await; // drain burst
        let t = Instant::now();
        let a = { let l = l.clone(); tokio::spawn(async move { l.acquire(1000).await }) };
        let b = { let l = l.clone(); tokio::spawn(async move { l.acquire(1000).await }) };
        a.await.unwrap();
        b.await.unwrap();
        let e = t.elapsed().as_secs_f64();
        assert!((1.99..=2.01).contains(&e), "elapsed {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn set_limit_zero_disables() {
        let l = RateLimiter::new(10);
        l.set_limit(0);
        assert_eq!(l.limit(), 0);
        let t = Instant::now();
        l.acquire(1_000_000).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
    }
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine limiter`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 4: Implement the limiter**

```rust
impl RateLimiter {
    /// `bytes_per_sec == 0` means unlimited. Burst capacity is one second of traffic.
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            rate: AtomicU64::new(bytes_per_sec),
            bucket: Mutex::new(Bucket { tokens: bytes_per_sec as f64, last: Instant::now() }),
        }
    }

    pub fn set_limit(&self, bytes_per_sec: u64) {
        self.rate.store(bytes_per_sec, Ordering::Relaxed);
    }

    pub fn limit(&self) -> u64 {
        self.rate.load(Ordering::Relaxed)
    }

    /// Takes `bytes` from the bucket, sleeping off any deficit. Tokens may go
    /// negative so concurrent callers queue up fairly behind each other.
    pub async fn acquire(&self, bytes: u64) {
        let rate = self.limit();
        if rate == 0 {
            return;
        }
        let wait = {
            let rate = rate as f64;
            let mut b = self.bucket.lock().unwrap();
            let now = Instant::now();
            b.tokens = (b.tokens + now.duration_since(b.last).as_secs_f64() * rate).min(rate);
            b.last = now;
            b.tokens -= bytes as f64;
            if b.tokens >= 0.0 {
                return;
            }
            Duration::from_secs_f64(-b.tokens / rate)
        };
        tokio::time::sleep(wait).await;
    }
}
```

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test -p rdm-engine limiter`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): token-bucket rate limiter

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Local test server + `probe`

**Files:**
- Create: `crates/engine/tests/support/mod.rs`, `crates/engine/src/probe.rs`, `crates/engine/tests/probe.rs`
- Modify: `crates/engine/src/lib.rs`

**Interfaces:**
- Consumes: `RemoteInfo`, `EngineError::from_status`, `filename::filename_from`.
- Produces:
  - `rdm_engine::probe(client: &reqwest::Client, url: &str) -> Result<RemoteInfo, EngineError>`.
  - Test support: `support::TestServer::start().await` and `.url(path) -> String`; `support::data(size) -> Vec<u8>`, `support::data_with_seed(size, seed)`, `support::seed_for(tag) -> usize`.
  - Server routes: `/file/{size}` (ranges, ETag `"v1"`, filename `test.bin`), `/etag/{tag}/{size}` (ranges, ETag `"<tag>"`, content seeded by tag, 10 ms per 16 KiB chunk), `/slow/{size}` (ranges, 10 ms per 16 KiB chunk), `/norange/{size}` (always 200, no Content-Disposition), `/flaky/{size}` (the first 2 non-probe range requests drop halfway), `/ranges-once/{size}` (only the first request gets a 206), `/expired` (403).

- [ ] **Step 1: Write the test server**

`crates/engine/tests/support/mod.rs`:
```rust
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use futures_util::StreamExt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

const CHUNK: usize = 16 * 1024;

pub fn data(size: usize) -> Vec<u8> {
    data_with_seed(size, 0)
}

pub fn data_with_seed(size: usize, seed: usize) -> Vec<u8> {
    (0..size).map(|i| ((i + seed) % 251) as u8).collect()
}

pub fn seed_for(tag: &str) -> usize {
    tag.bytes().map(usize::from).sum()
}

#[derive(Clone, Default)]
struct Counters {
    flaky: Arc<AtomicUsize>,
    ranges_once: Arc<AtomicUsize>,
}

pub struct TestServer {
    pub base: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/file/{size}", get(file))
            .route("/etag/{tag}/{size}", get(etag_file))
            .route("/slow/{size}", get(slow))
            .route("/norange/{size}", get(norange))
            .route("/flaky/{size}", get(flaky))
            .route("/ranges-once/{size}", get(ranges_once))
            .route("/expired", get(|| async { StatusCode::FORBIDDEN }))
            .with_state(Counters::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { base: format!("http://{addr}") }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

/// Returns (start, end_exclusive) for `Range: bytes=a-b` / `bytes=a-`.
fn parse_range(headers: &HeaderMap, len: usize) -> Option<(usize, usize)> {
    let v = headers.get(header::RANGE)?.to_str().ok()?.strip_prefix("bytes=")?;
    let (a, b) = v.split_once('-')?;
    let start: usize = a.parse().ok()?;
    let end = if b.is_empty() { len } else { b.parse::<usize>().ok()? + 1 };
    Some((start, end.min(len)))
}

fn stream_body(bytes: Vec<u8>, delay: Option<Duration>, fail_after: Option<usize>) -> Body {
    let chunks: Vec<Vec<u8>> = bytes.chunks(CHUNK).map(<[u8]>::to_vec).collect();
    let stream = futures_util::stream::iter(chunks.into_iter().enumerate()).then(move |(i, c)| async move {
        if let Some(d) = delay {
            tokio::time::sleep(d).await;
        }
        if fail_after.is_some_and(|n| i * CHUNK >= n) {
            return Err(std::io::Error::other("simulated connection drop"));
        }
        Ok::<_, std::io::Error>(c)
    });
    Body::from_stream(stream)
}

fn ranged(headers: &HeaderMap, body: Vec<u8>, etag: &str, delay: Option<Duration>) -> Response {
    let len = body.len();
    let builder = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, etag)
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"test.bin\"");
    match parse_range(headers, len) {
        Some((start, _)) if start >= len => builder
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{len}"))
            .body(Body::empty())
            .unwrap(),
        Some((start, end)) => builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{len}", end - 1))
            .header(header::CONTENT_LENGTH, end - start)
            .body(stream_body(body[start..end].to_vec(), delay, None))
            .unwrap(),
        None => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, len)
            .body(stream_body(body, delay, None))
            .unwrap(),
    }
}

fn full_200(size: usize) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, size)
        .body(stream_body(data(size), None, None))
        .unwrap()
}

async fn file(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    ranged(&headers, data(size), "\"v1\"", None)
}

async fn etag_file(Path((tag, size)): Path<(String, usize)>, headers: HeaderMap) -> Response {
    ranged(&headers, data_with_seed(size, seed_for(&tag)), &format!("\"{tag}\""), Some(Duration::from_millis(10)))
}

async fn slow(Path(size): Path<usize>, headers: HeaderMap) -> Response {
    ranged(&headers, data(size), "\"v1\"", Some(Duration::from_millis(10)))
}

async fn norange(Path(size): Path<usize>) -> Response {
    full_200(size)
}

async fn flaky(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    let full = data(size);
    let Some((start, end)) = parse_range(&headers, size) else {
        return ranged(&headers, full, "\"v1\"", None);
    };
    if end - start > 1 && c.flaky.fetch_add(1, Ordering::SeqCst) < 2 {
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::ETAG, "\"v1\"")
            .header(header::CONTENT_RANGE, format!("bytes {start}-{}/{size}", end - 1))
            .header(header::CONTENT_LENGTH, end - start)
            .body(stream_body(full[start..end].to_vec(), None, Some((end - start) / 2)))
            .unwrap();
    }
    ranged(&headers, full, "\"v1\"", None)
}

async fn ranges_once(State(c): State<Counters>, Path(size): Path<usize>, headers: HeaderMap) -> Response {
    if c.ranges_once.fetch_add(1, Ordering::SeqCst) == 0 {
        ranged(&headers, data(size), "\"v1\"", None)
    } else {
        full_200(size)
    }
}
```

- [ ] **Step 2: Add `probe` as a stub and write the failing tests**

`crates/engine/src/probe.rs`:
```rust
use crate::{error::EngineError, filename::filename_from, info::RemoteInfo};
use reqwest::{Client, StatusCode, header};

pub async fn probe(client: &Client, url: &str) -> Result<RemoteInfo, EngineError> {
    todo!()
}
```

In `lib.rs`, add `pub mod probe;` and `pub use probe::probe;`.

`crates/engine/tests/probe.rs`:
```rust
mod support;

use rdm_engine::{EngineError, default_client, probe};
use support::TestServer;

#[tokio::test]
async fn probes_range_capable_server() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/file/1000000")).await.unwrap();
    assert_eq!(info.size, Some(1_000_000));
    assert!(info.accepts_ranges);
    assert_eq!(info.filename, "test.bin");
    assert_eq!(info.etag.as_deref(), Some("\"v1\""));
    assert_eq!(info.url, s.url("/file/1000000"));
}

#[tokio::test]
async fn probes_server_without_ranges() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/norange/1000")).await.unwrap();
    assert_eq!(info.size, Some(1000));
    assert!(!info.accepts_ranges);
    assert_eq!(info.filename, "1000");
}

#[tokio::test]
async fn probes_zero_byte_file() {
    let s = TestServer::start().await;
    let info = probe(&default_client(), &s.url("/file/0")).await.unwrap();
    assert_eq!(info.size, Some(0));
    assert!(!info.accepts_ranges);
}

#[tokio::test]
async fn forbidden_is_link_expired() {
    let s = TestServer::start().await;
    let err = probe(&default_client(), &s.url("/expired")).await.unwrap_err();
    assert!(matches!(err, EngineError::LinkExpired(403)), "{err:?}");
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine --test probe`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 4: Implement `probe`**

```rust
/// One `Range: bytes=0-0` GET tells us size, range support, filename and validators.
pub async fn probe(client: &Client, url: &str) -> Result<RemoteInfo, EngineError> {
    let resp = client.get(url).header(header::RANGE, "bytes=0-0").send().await?;
    let headers = resp.headers();
    let get = |name: header::HeaderName| {
        headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string)
    };
    let (size, accepts_ranges) = match resp.status() {
        StatusCode::PARTIAL_CONTENT => (get(header::CONTENT_RANGE).and_then(|v| parse_total(&v)), true),
        StatusCode::RANGE_NOT_SATISFIABLE => (Some(0), false),
        s if s.is_success() => (resp.content_length(), false),
        s => return Err(EngineError::from_status(s.as_u16())),
    };
    Ok(RemoteInfo {
        url: url.to_string(),
        size,
        accepts_ranges: accepts_ranges && size.is_some(),
        filename: filename_from(get(header::CONTENT_DISPOSITION).as_deref(), resp.url().as_str()),
        etag: get(header::ETAG),
        last_modified: get(header::LAST_MODIFIED),
    })
}

/// `bytes 0-0/12345` → 12345; `bytes 0-0/*` → None.
fn parse_total(content_range: &str) -> Option<u64> {
    content_range.rsplit('/').next()?.trim().parse().ok()
}
```

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test -p rdm-engine`
Expected: PASS for all tests, including 4 in `tests/probe.rs`.

- [ ] **Step 6: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): probe + local range-aware test server

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Segmented download with dynamic split, retries and pause/resume

**Files:**
- Create: `crates/engine/src/download.rs`, `crates/engine/tests/download.rs`
- Modify: `crates/engine/src/lib.rs`

**Interfaces:**
- Consumes: `probe`, `RemoteInfo`, `segments::{Segment, plan, split_largest}`, `state::{DownloadState, part_path, state_path}`, `RateLimiter`, `EngineError`.
- Produces (the main API later used by `core`):
  - `rdm_engine::DownloadOptions { connections: usize, min_split: u64, limiter: Option<Arc<RateLimiter>>, retry_base: Duration }`, `Default` = 8 / 1 MiB / None / 500 ms.
  - `rdm_engine::Progress { downloaded: u64, total: Option<u64>, speed_bps: u64, segments: Vec<Segment> }` (Clone, Debug, Default, PartialEq).
  - `rdm_engine::Outcome { Completed(PathBuf), Paused }`.
  - `rdm_engine::download(client: &Client, url: &str, dest: &Path, opts: &DownloadOptions, cancel: CancellationToken, progress: &watch::Sender<Progress>) -> Result<Outcome, EngineError>`.
  - Re-export `rdm_engine::CancellationToken`.

- [ ] **Step 1: Register the module**

In `lib.rs`, add `pub mod download;`, `pub use download::{DownloadOptions, Outcome, Progress, download};` and `pub use tokio_util::sync::CancellationToken;`.

- [ ] **Step 2: Write the failing integration tests**

`crates/engine/tests/download.rs`:
```rust
mod support;

use rdm_engine::state::{DownloadState, part_path, state_path};
use rdm_engine::{CancellationToken, DownloadOptions, EngineError, Outcome, Progress, default_client, download};
use std::path::Path;
use std::time::Duration;
use support::{TestServer, data};
use tokio::sync::watch;

fn opts(connections: usize) -> DownloadOptions {
    DownloadOptions {
        connections,
        min_split: 64 * 1024,
        retry_base: Duration::from_millis(10),
        ..Default::default()
    }
}

async fn run(url: &str, dest: &Path, o: &DownloadOptions, cancel: CancellationToken) -> (Result<Outcome, EngineError>, Progress) {
    let (tx, rx) = watch::channel(Progress::default());
    let r = download(&default_client(), url, dest, o, cancel, &tx).await;
    let last = rx.borrow().clone();
    (r, last)
}

fn cancel_after(ms: u64) -> CancellationToken {
    let t = CancellationToken::new();
    let t2 = t.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        t2.cancel();
    });
    t
}

#[tokio::test]
async fn downloads_byte_exact_with_many_connections_into_new_dir() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("nested").join("out.bin");
    let size = 5 * 1024 * 1024 + 123;
    let (r, last) = run(&s.url(&format!("/file/{size}")), &dest, &opts(8), CancellationToken::new()).await;
    assert_eq!(r.unwrap(), Outcome::Completed(dest.clone()));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
    assert!(!part_path(&dest).exists());
    assert!(!state_path(&dest).exists());
    assert_eq!(last.downloaded, size as u64);
}

#[tokio::test]
async fn single_connection_works() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("one.bin");
    let (r, _) = run(&s.url("/file/700000"), &dest, &opts(1), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(700_000));
}

#[tokio::test]
async fn pause_then_resume_keeps_progress() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("big.bin");
    let size = 4 * 1024 * 1024;
    let url = s.url(&format!("/slow/{size}"));

    let (r, _) = run(&url, &dest, &opts(4), cancel_after(150)).await;
    assert_eq!(r.unwrap(), Outcome::Paused);
    assert!(!dest.exists());
    assert!(part_path(&dest).exists());
    let saved = DownloadState::load(&state_path(&dest)).await.unwrap().expect("sidecar written");
    assert!(saved.downloaded() > 0 && saved.downloaded() < size as u64, "saved {}", saved.downloaded());

    let (r, _) = run(&url, &dest, &opts(4), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
    assert!(!state_path(&dest).exists());
}

#[tokio::test]
async fn retries_dropped_connections() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("flaky.bin");
    let size = 2 * 1024 * 1024;
    let (r, _) = run(&s.url(&format!("/flaky/{size}")), &dest, &opts(2), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(size));
}

#[tokio::test]
async fn range_not_honored_fails_cleanly() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("cdn.bin");
    let (r, _) = run(&s.url("/ranges-once/2000000"), &dest, &opts(4), CancellationToken::new()).await;
    assert!(matches!(r, Err(EngineError::RangeNotHonored)), "{r:?}");
    assert!(!dest.exists());
}

#[tokio::test]
async fn expired_link_is_reported() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (r, _) = run(&s.url("/expired"), &dir.path().join("x"), &opts(4), CancellationToken::new()).await;
    assert!(matches!(r, Err(EngineError::LinkExpired(403))), "{r:?}");
}
```

- [ ] **Step 3: Write `download.rs` with a stub and confirm the tests fail**

`crates/engine/src/download.rs`, with only the public types plus a `todo!()` body for `download`:
```rust
use crate::{
    error::EngineError,
    info::RemoteInfo,
    limiter::RateLimiter,
    probe::probe,
    segments::{self, Segment},
    state::{DownloadState, part_path, state_path},
};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode, header};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

const MAX_RETRIES: u32 = 5;
const TICK: Duration = Duration::from_millis(250);
const SAVE_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct DownloadOptions {
    pub connections: usize,
    pub min_split: u64,
    pub limiter: Option<Arc<RateLimiter>>,
    pub retry_base: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self { connections: 8, min_split: 1024 * 1024, limiter: None, retry_base: Duration::from_millis(500) }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: u64,
    pub segments: Vec<Segment>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Completed(PathBuf),
    Paused,
}

pub async fn download(
    client: &Client,
    url: &str,
    dest: &Path,
    opts: &DownloadOptions,
    cancel: CancellationToken,
    progress: &watch::Sender<Progress>,
) -> Result<Outcome, EngineError> {
    todo!()
}
```

Run: `cargo test -p rdm-engine --test download`
Expected: FAIL with `not yet implemented` in all 6 tests.

- [ ] **Step 4: Implement the segmented path**

Replace the `download` stub and add the private machinery below it:
```rust
pub async fn download(
    client: &Client,
    url: &str,
    dest: &Path,
    opts: &DownloadOptions,
    cancel: CancellationToken,
    progress: &watch::Sender<Progress>,
) -> Result<Outcome, EngineError> {
    if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(dir).await?;
    }
    let info = probe(client, url).await?;
    let job = Job {
        client,
        dest,
        part: part_path(dest),
        state_path: state_path(dest),
        opts,
        cancel,
        progress,
    };
    match (info.size, info.accepts_ranges) {
        (Some(size), true) if size > 0 => segmented(&job, &info, size).await,
        _ => single_stream(&job, &info).await,
    }
}

struct Job<'a> {
    client: &'a Client,
    dest: &'a Path,
    part: PathBuf,
    state_path: PathBuf,
    opts: &'a DownloadOptions,
    cancel: CancellationToken,
    progress: &'a watch::Sender<Progress>,
}

type WorkerSet = JoinSet<(usize, Result<(), EngineError>)>;

#[derive(Clone)]
struct WorkerCtx {
    client: Client,
    url: String,
    part: PathBuf,
    segs: Arc<Mutex<Vec<Segment>>>,
    limiter: Option<Arc<RateLimiter>>,
    cancel: CancellationToken,
    retry_base: Duration,
}

async fn segmented(job: &Job<'_>, info: &RemoteInfo, size: u64) -> Result<Outcome, EngineError> {
    let resumed = match DownloadState::load(&job.state_path).await? {
        Some(s) if s.matches(info) && job.part.exists() => Some(s),
        _ => None,
    };
    let mut state = match resumed {
        Some(s) => s,
        None => {
            let file = tokio::fs::File::create(&job.part).await?;
            file.set_len(size).await?;
            DownloadState {
                url: info.url.clone(),
                size,
                etag: info.etag.clone(),
                last_modified: info.last_modified.clone(),
                segments: segments::plan(size, job.opts.connections),
            }
        }
    };
    // A refreshed link keeps the bytes already on disk.
    state.url = info.url.clone();

    let segs = Arc::new(Mutex::new(state.segments.clone()));
    let workers_cancel = job.cancel.child_token();
    let ctx = WorkerCtx {
        client: job.client.clone(),
        url: info.url.clone(),
        part: job.part.clone(),
        segs: segs.clone(),
        limiter: job.opts.limiter.clone(),
        cancel: workers_cancel.clone(),
        retry_base: job.opts.retry_base,
    };

    let mut set = WorkerSet::new();
    let mut active = HashSet::new();
    let pending: Vec<usize> = {
        let s = segs.lock().unwrap();
        s.iter().enumerate().filter(|(_, seg)| seg.remaining() > 0).map(|(i, _)| i).collect()
    };
    for idx in pending.into_iter().take(job.opts.connections.max(1)) {
        active.insert(idx);
        spawn_worker(&mut set, &ctx, idx);
    }

    let mut tick = tokio::time::interval(TICK);
    let mut last_save = Instant::now();
    let mut last_bytes = state.downloaded();
    let mut speed = 0.0_f64;
    let mut failure: Option<EngineError> = None;

    loop {
        tokio::select! {
            joined = set.join_next() => {
                let Some(joined) = joined else { break };
                let (idx, result) = joined.map_err(|e| EngineError::Internal(e.to_string()))?;
                active.remove(&idx);
                if let Err(e) = result {
                    if failure.is_none() {
                        failure = Some(e);
                        workers_cancel.cancel();
                    }
                    continue;
                }
                if workers_cancel.is_cancelled() {
                    continue;
                }
                let next = {
                    let mut s = segs.lock().unwrap();
                    let idle = s
                        .iter()
                        .enumerate()
                        .find(|(i, seg)| seg.remaining() > 0 && !active.contains(i))
                        .map(|(i, _)| i);
                    idle.or_else(|| segments::split_largest(&mut s, job.opts.min_split))
                };
                if let Some(n) = next {
                    active.insert(n);
                    spawn_worker(&mut set, &ctx, n);
                }
            }
            _ = tick.tick() => {
                let snapshot = segs.lock().unwrap().clone();
                let downloaded: u64 = snapshot.iter().map(|s| s.written).sum();
                let inst = downloaded.saturating_sub(last_bytes) as f64 / TICK.as_secs_f64();
                speed = if speed == 0.0 { inst } else { speed * 0.7 + inst * 0.3 };
                last_bytes = downloaded;
                job.progress.send_replace(Progress {
                    downloaded,
                    total: Some(size),
                    speed_bps: speed as u64,
                    segments: snapshot.clone(),
                });
                if last_save.elapsed() >= SAVE_EVERY {
                    state.segments = snapshot;
                    state.save(&job.state_path).await?;
                    last_save = Instant::now();
                }
            }
        }
    }

    state.segments = segs.lock().unwrap().clone();
    let downloaded = state.downloaded();
    if let Some(e) = failure {
        state.save(&job.state_path).await?;
        return Err(e);
    }
    if downloaded < size {
        state.save(&job.state_path).await?;
        job.progress.send_replace(Progress { downloaded, total: Some(size), speed_bps: 0, segments: state.segments.clone() });
        if job.cancel.is_cancelled() {
            return Ok(Outcome::Paused);
        }
        return Err(EngineError::Internal(format!("stopped at {downloaded} of {size} bytes")));
    }
    finish(job, Some(size), downloaded).await
}

async fn finish(job: &Job<'_>, total: Option<u64>, downloaded: u64) -> Result<Outcome, EngineError> {
    tokio::fs::rename(&job.part, job.dest).await?;
    let _ = tokio::fs::remove_file(&job.state_path).await;
    job.progress.send_replace(Progress { downloaded, total, speed_bps: 0, segments: Vec::new() });
    Ok(Outcome::Completed(job.dest.to_path_buf()))
}

fn spawn_worker(set: &mut WorkerSet, ctx: &WorkerCtx, idx: usize) {
    let ctx = ctx.clone();
    set.spawn(async move {
        let r = run_segment(&ctx, idx).await;
        (idx, r)
    });
}

async fn run_segment(ctx: &WorkerCtx, idx: usize) -> Result<(), EngineError> {
    let mut attempt = 0;
    loop {
        match fetch_segment(ctx, idx).await {
            Ok(()) => return Ok(()),
            Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                let delay = ctx.retry_base * 2u32.pow(attempt);
                attempt += 1;
                tokio::select! {
                    _ = ctx.cancel.cancelled() => return Ok(()),
                    _ = tokio::time::sleep(delay) => {}
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Streams this segment's remaining range into the part file. The segment's
/// `end` can shrink while we run (dynamic split), so it's re-read per chunk,
/// and bytes are reserved under the lock before writing so a concurrent split
/// never hands out a range we're about to write.
async fn fetch_segment(ctx: &WorkerCtx, idx: usize) -> Result<(), EngineError> {
    let (pos, end) = {
        let s = ctx.segs.lock().unwrap();
        (s[idx].pos(), s[idx].end)
    };
    if pos >= end || ctx.cancel.is_cancelled() {
        return Ok(());
    }
    let request = ctx.client.get(&ctx.url).header(header::RANGE, format!("bytes={pos}-{}", end - 1));
    let resp = tokio::select! {
        _ = ctx.cancel.cancelled() => return Ok(()),
        r = request.send() => r?,
    };
    match resp.status() {
        StatusCode::PARTIAL_CONTENT => {}
        StatusCode::OK => return Err(EngineError::RangeNotHonored),
        s => return Err(EngineError::from_status(s.as_u16())),
    }

    let mut file = OpenOptions::new().write(true).open(&ctx.part).await?;
    file.seek(std::io::SeekFrom::Start(pos)).await?;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::select! {
            _ = ctx.cancel.cancelled() => None,
            c = stream.next() => c,
        };
        let Some(chunk) = next else { break };
        let chunk = chunk?;
        if let Some(l) = &ctx.limiter {
            l.acquire(chunk.len() as u64).await;
        }
        let (take, segment_done) = {
            let mut s = ctx.segs.lock().unwrap();
            let take = (s[idx].remaining() as usize).min(chunk.len());
            s[idx].written += take as u64;
            (take, s[idx].remaining() == 0)
        };
        if let Err(e) = file.write_all(&chunk[..take]).await {
            ctx.segs.lock().unwrap()[idx].written -= take as u64;
            return Err(e.into());
        }
        if segment_done || take < chunk.len() {
            break;
        }
    }
    file.flush().await?;
    let done = ctx.segs.lock().unwrap()[idx].remaining() == 0;
    if done || ctx.cancel.is_cancelled() {
        Ok(())
    } else {
        Err(EngineError::Network("connection closed early".into()))
    }
}

async fn single_stream(job: &Job<'_>, info: &RemoteInfo) -> Result<Outcome, EngineError> {
    todo!("Task 6")
}
```

- [ ] **Step 5: Run the tests and confirm all of Task 5's tests pass**

Run: `cargo test -p rdm-engine --test download`
Expected: PASS for all 6 tests. (`expired_link_is_reported` fails in `probe` before `single_stream` is reached, so the Task 6 stub isn't hit.)

If `pause_then_resume_keeps_progress` finishes before 150 ms on a fast machine, raise `size` to `8 * 1024 * 1024`. Don't shorten the cancel delay.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p rdm-engine --all-targets -- -D warnings`
Expected: no warnings. (`too_many_arguments` is avoided by `Job` and `WorkerCtx`.)

- [ ] **Step 7: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): segmented download with dynamic split, retries, pause/resume

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Single-stream fallback, zero-byte files, ETag change, speed limit

**Files:**
- Modify: `crates/engine/src/download.rs` (replace the `single_stream` stub), `crates/engine/tests/download.rs` (append tests)

**Interfaces:**
- Consumes: everything from Task 5.
- Produces: no new public API. `download` now handles servers without range support and empty files.

- [ ] **Step 1: Append the failing tests**

Add to `crates/engine/tests/download.rs`:
```rust
use rdm_engine::RateLimiter;
use std::sync::Arc;
use support::{data_with_seed, seed_for};

#[tokio::test]
async fn server_without_ranges_uses_single_stream() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("plain.bin");
    let (r, last) = run(&s.url("/norange/1048576"), &dest, &opts(8), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data(1_048_576));
    assert!(!state_path(&dest).exists());
    assert_eq!(last.downloaded, 1_048_576);
}

#[tokio::test]
async fn zero_byte_file_completes() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("empty.bin");
    let (r, _) = run(&s.url("/file/0"), &dest, &opts(8), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), 0);
}

#[tokio::test]
async fn etag_change_restarts_from_zero() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("changing.bin");
    let size = 4 * 1024 * 1024;

    let (r, _) = run(&s.url(&format!("/etag/v1/{size}")), &dest, &opts(4), cancel_after(150)).await;
    assert_eq!(r.unwrap(), Outcome::Paused);

    let (r, _) = run(&s.url(&format!("/etag/v2/{size}")), &dest, &opts(4), CancellationToken::new()).await;
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    assert_eq!(std::fs::read(&dest).unwrap(), data_with_seed(size, seed_for("v2")));
}

#[tokio::test]
async fn speed_limit_is_respected() {
    let s = TestServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("limited.bin");
    let size = 3 * 1024 * 1024;
    let o = DownloadOptions {
        limiter: Some(Arc::new(RateLimiter::new(1024 * 1024))),
        ..opts(4)
    };
    let started = std::time::Instant::now();
    let (r, _) = run(&s.url(&format!("/file/{size}")), &dest, &o, CancellationToken::new()).await;
    let secs = started.elapsed().as_secs_f64();
    assert!(matches!(r.unwrap(), Outcome::Completed(_)));
    // 1 s burst allowance + 2 MiB at 1 MiB/s ≈ 2 s.
    assert!((1.7..=3.5).contains(&secs), "took {secs:.2}s");
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p rdm-engine --test download`
Expected: `server_without_ranges_uses_single_stream` and `zero_byte_file_completes` panic with `not yet implemented: Task 6`. The other two may already pass, because they exercise the segmented path.

- [ ] **Step 3: Implement `single_stream`**

Replace the stub in `download.rs`:
```rust
/// Servers without range support (or unknown size): one sequential stream.
/// Pausing discards the partial file, because it can't be resumed.
async fn single_stream(job: &Job<'_>, info: &RemoteInfo) -> Result<Outcome, EngineError> {
    let resp = job.client.get(&info.url).send().await?;
    if !resp.status().is_success() {
        return Err(EngineError::from_status(resp.status().as_u16()));
    }
    let total = info.size.or(resp.content_length());
    let mut file = tokio::fs::File::create(&job.part).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0_u64;
    let mut last_tick = Instant::now();
    let mut last_bytes = 0_u64;
    loop {
        let next = tokio::select! {
            _ = job.cancel.cancelled() => {
                drop(file);
                let _ = tokio::fs::remove_file(&job.part).await;
                return Ok(Outcome::Paused);
            }
            c = stream.next() => c,
        };
        let Some(chunk) = next else { break };
        let chunk = chunk?;
        if let Some(l) = &job.opts.limiter {
            l.acquire(chunk.len() as u64).await;
        }
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if last_tick.elapsed() >= TICK {
            let speed = (downloaded - last_bytes) as f64 / last_tick.elapsed().as_secs_f64();
            job.progress.send_replace(Progress { downloaded, total, speed_bps: speed as u64, segments: Vec::new() });
            last_tick = Instant::now();
            last_bytes = downloaded;
        }
    }
    file.flush().await?;
    drop(file);
    if total.is_some_and(|t| downloaded < t) {
        return Err(EngineError::Network("connection closed early".into()));
    }
    finish(job, total, downloaded).await
}
```

- [ ] **Step 4: Run all engine tests and confirm they pass**

Run: `cargo test -p rdm-engine`
Expected: PASS for every unit test, 4 probe tests and 10 download tests.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p rdm-engine --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): single-stream fallback, empty files, etag-change restart, limiter test

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: `rdm-cli` — real-world proof

**Files:**
- Create: `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`
- Modify: `Cargo.toml` (add `"crates/cli"` to `members`)

**Interfaces:**
- Consumes: `default_client`, `probe`, `download`, `DownloadOptions`, `Outcome`, `Progress`, `RateLimiter`, `CancellationToken`, `filename::unique_path`, `state::part_path`.
- Produces: the `rdm-cli` binary: `rdm-cli <URL> [-d DIR] [-c CONNECTIONS] [-l LIMIT_KBPS]`.

- [ ] **Step 1: Create the crate**

In the root `Cargo.toml`, set `members = ["crates/engine", "crates/cli"]`.

`crates/cli/Cargo.toml`:
```toml
[package]
name = "rdm-cli"
version.workspace = true
edition.workspace = true

[dependencies]
rdm-engine = { path = "../engine" }
tokio.workspace = true
clap.workspace = true
```

- [ ] **Step 2: Write `main.rs` with a failing unit test for `human`**

`crates/cli/src/main.rs`:
```rust
use clap::Parser;
use rdm_engine::filename::unique_path;
use rdm_engine::state::part_path;
use rdm_engine::{CancellationToken, DownloadOptions, Outcome, Progress, RateLimiter, default_client, download, probe};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Parser)]
#[command(name = "rdm-cli", about = "RDM engine test harness: segmented, resumable downloads")]
struct Args {
    url: String,
    /// Destination folder
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
    /// Parallel connections (1–16)
    #[arg(short, long, default_value_t = 8, value_parser = clap::value_parser!(u8).range(1..=16))]
    connections: u8,
    /// Speed limit in KB/s (0 = unlimited)
    #[arg(short, long, default_value_t = 0)]
    limit: u64,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let client = default_client();
    let info = match probe(&client, &args.url).await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let dest = pick_dest(&args.dir, &info.filename);
    println!("{} → {}", info.filename, dest.display());
    println!(
        "size: {}  resumable: {}",
        info.size.map(human).unwrap_or_else(|| "unknown".into()),
        info.accepts_ranges
    );

    let opts = DownloadOptions {
        connections: args.connections as usize,
        limiter: (args.limit > 0).then(|| Arc::new(RateLimiter::new(args.limit * 1024))),
        ..Default::default()
    };
    let cancel = CancellationToken::new();
    let on_ctrl_c = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        on_ctrl_c.cancel();
    });

    let (tx, mut rx) = watch::channel(Progress::default());
    let printer = tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let p = rx.borrow_and_update().clone();
            print!("\r{}   ", progress_line(&p));
            let _ = std::io::stdout().flush();
        }
    });

    let result = download(&client, &args.url, &dest, &opts, cancel, &tx).await;
    drop(tx);
    let _ = printer.await;
    println!();
    match result {
        Ok(Outcome::Completed(p)) => {
            println!("done: {}", p.display());
            ExitCode::SUCCESS
        }
        Ok(Outcome::Paused) => {
            println!("paused: run the same command again to resume");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Reuse the destination of an unfinished download so it resumes.
fn pick_dest(dir: &Path, name: &str) -> PathBuf {
    let direct = dir.join(name);
    if part_path(&direct).exists() { direct } else { unique_path(dir, name) }
}

fn progress_line(p: &Progress) -> String {
    let pct = p
        .total
        .filter(|t| *t > 0)
        .map(|t| format!("{:5.1}%", p.downloaded as f64 * 100.0 / t as f64))
        .unwrap_or_else(|| "  ?  ".into());
    format!(
        "{pct}  {} / {}  {}/s  [{} segments]",
        human(p.downloaded),
        p.total.map(human).unwrap_or_else(|| "?".into()),
        human(p.speed_bps),
        p.segments.len()
    )
}

fn human(bytes: u64) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_sizes() {
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(human(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
```

- [ ] **Step 3: Run the test and confirm it fails**

Run: `cargo test -p rdm-cli`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 4: Implement `human`**

```rust
fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}
```

- [ ] **Step 5: Run the workspace tests and clippy**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: everything passes with no warnings.

- [ ] **Step 6: Real-world check (manual)**

```bash
cargo build -p rdm-cli --release
./target/release/rdm-cli.exe "https://proof.ovh.net/files/100Mb.dat" -d "$TEMP/rdm-test"
```
Expected: the percentage, speed and `[N segments]` update live. Once the first segments finish, the segment count rises above 8 (dynamic split).

Then:
1. Run it again and press **Ctrl+C** around 40%. Expected: `paused: run the same command again to resume`, with `100Mb.dat.rdmpart` and `100Mb.dat.rdmstate` in the folder.
2. Run the same command again. Expected: it starts at about 40%, not 0%, and finishes with `done: …\100Mb.dat`. The file is exactly 104,857,600 bytes.
3. Run with `-l 500`. Expected: the speed settles near 500 KB/s.

If `proof.ovh.net` is unreachable, use any large direct-download URL (for example an Ubuntu ISO mirror) and stop it early with Ctrl+C.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/cli
git commit -m "feat(cli): rdm-cli harness with live progress and Ctrl+C pause

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Later plans (written after this one ships)

- **Plan 2:** `core` (items, queues, `state.json` persistence, `Manager` + events) and the iced shell (theme tokens, toolbar, sidebar, list, inspector) wired to real HTTP downloads.
- **Plan 3:** `media`, covering the yt-dlp bootstrap, probe, the quality sheet, MP4/MP3 and playlists.
- **Plan 4:** speed-limit UI, scheduler, settings sheet, accent picker.
- **Plan 5:** clipboard watch, the localhost bridge and the Chrome MV3 extension.
