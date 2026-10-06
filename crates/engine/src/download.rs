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
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
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
    let _guard = DestGuard::acquire(dest)?;
    if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(dir).await?;
    }
    let info = match retrying(&cancel, opts.retry_base, 0, || probe(client, url)).await {
        Some(result) => result?,
        None => return Ok(Outcome::Paused),
    };
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
        (Some(size), true) if size > 0 => match segmented(&job, &info, size).await {
            // The server stopped honoring ranges: start over on one plain stream.
            Err(EngineError::RangeNotHonored) => {
                let _ = tokio::fs::remove_file(&job.part).await;
                let _ = tokio::fs::remove_file(&job.state_path).await;
                single_stream(&job, &info).await
            }
            other => other,
        },
        _ => single_stream(&job, &info).await,
    }
}

/// Destinations currently being written by this process.
static IN_USE: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Default::default);

/// Holds a destination for one `download` call so two downloads never share a part file.
struct DestGuard(PathBuf);

impl DestGuard {
    fn acquire(dest: &Path) -> Result<Self, EngineError> {
        let absolute = std::path::absolute(dest)?;
        // Windows paths are case-insensitive.
        let key = PathBuf::from(absolute.to_string_lossy().to_lowercase());
        if !IN_USE.lock().unwrap().insert(key.clone()) {
            return Err(EngineError::DestinationBusy);
        }
        Ok(Self(key))
    }
}

impl Drop for DestGuard {
    fn drop(&mut self) {
        IN_USE.lock().unwrap().remove(&self.0);
    }
}

/// Runs `op`, retrying retryable errors with backoff. `None` means cancelled.
async fn retrying<T, F, Fut>(
    cancel: &CancellationToken,
    base: Duration,
    salt: usize,
    mut op: F,
) -> Option<Result<T, EngineError>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, EngineError>>,
{
    let mut attempt = 0;
    loop {
        let result = tokio::select! {
            _ = cancel.cancelled() => return None,
            r = op() => r,
        };
        match result {
            Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                let delay = backoff(&e, base, attempt, salt);
                attempt += 1;
                tokio::select! {
                    _ = cancel.cancelled() => return None,
                    _ = tokio::time::sleep(delay) => {}
                }
            }
            other => return Some(other),
        }
    }
}

/// Exponential backoff, at least what the server asked for in `Retry-After`, plus a
/// per-segment offset so refused connections don't all retry at the same instant.
fn backoff(e: &EngineError, base: Duration, attempt: u32, salt: usize) -> Duration {
    let exp = base * 2u32.pow(attempt);
    let wait = e.retry_after().map_or(exp, |ra| ra.max(exp));
    wait + base * (salt % 7) as u32 / 7
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
    size: u64,
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
        size,
        part: job.part.clone(),
        segs: segs.clone(),
        limiter: job.opts.limiter.clone(),
        cancel: workers_cancel.clone(),
        retry_base: job.opts.retry_base,
    };

    let mut set = WorkerSet::new();
    let mut active = HashSet::new();
    // Lowered when the server refuses extra connections.
    let mut max_active = job.opts.connections.max(1);
    let pending: Vec<usize> = {
        let s = segs.lock().unwrap();
        s.iter().enumerate().filter(|(_, seg)| seg.remaining() > 0).map(|(i, _)| i).collect()
    };
    for idx in pending.into_iter().take(max_active) {
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
                    if failure.is_none() && e.is_retryable() && !active.is_empty() {
                        // Other connections are still healthy: the server is limiting us.
                        // Run with fewer and leave this segment for a worker that frees up.
                        max_active = active.len();
                    } else if failure.is_none() {
                        failure = Some(e);
                        workers_cancel.cancel();
                    }
                    continue;
                }
                if workers_cancel.is_cancelled() {
                    continue;
                }
                while active.len() < max_active {
                    let next = {
                        let mut s = segs.lock().unwrap();
                        let idle = s
                            .iter()
                            .enumerate()
                            .find(|(i, seg)| seg.remaining() > 0 && !active.contains(i))
                            .map(|(i, _)| i);
                        idle.or_else(|| segments::split_largest(&mut s, job.opts.min_split))
                    };
                    let Some(n) = next else { break };
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
                // Best effort: a locked sidecar (antivirus, sync tools) must not kill the
                // download; the next tick tries again.
                if last_save.elapsed() >= SAVE_EVERY {
                    state.segments = snapshot;
                    if state.save(&job.state_path).await.is_ok() {
                        last_save = Instant::now();
                    }
                }
            }
        }
    }

    state.segments = segs.lock().unwrap().clone();
    let downloaded = state.downloaded();
    if let Some(e) = failure {
        if sync_part(&job.part).await.is_ok() {
            let _ = state.save(&job.state_path).await;
        }
        return Err(e);
    }
    if downloaded < size {
        sync_part(&job.part).await?;
        state.save(&job.state_path).await?;
        job.progress.send_replace(Progress { downloaded, total: Some(size), speed_bps: 0, segments: state.segments.clone() });
        if job.cancel.is_cancelled() {
            return Ok(Outcome::Paused);
        }
        return Err(EngineError::Internal(format!("stopped at {downloaded} of {size} bytes")));
    }
    finish(job, Some(size), downloaded).await
}

/// Forces part-file data to disk so a sidecar saved afterwards never claims bytes
/// that a power loss could still drop.
async fn sync_part(part: &Path) -> std::io::Result<()> {
    OpenOptions::new().write(true).open(part).await?.sync_data().await
}

async fn finish(job: &Job<'_>, total: Option<u64>, downloaded: u64) -> Result<Outcome, EngineError> {
    // A file may have appeared at this name while we were paused: never replace it.
    let target = match (job.dest.exists(), job.dest.parent(), job.dest.file_name()) {
        (true, Some(dir), Some(name)) => crate::filename::unique_path(dir, &name.to_string_lossy()),
        _ => job.dest.to_path_buf(),
    };
    tokio::fs::rename(&job.part, &target).await?;
    let _ = tokio::fs::remove_file(&job.state_path).await;
    job.progress.send_replace(Progress { downloaded, total, speed_bps: 0, segments: Vec::new() });
    Ok(Outcome::Completed(target))
}

fn spawn_worker(set: &mut WorkerSet, ctx: &WorkerCtx, idx: usize) {
    let ctx = ctx.clone();
    set.spawn(async move {
        let r = run_segment(&ctx, idx).await;
        (idx, r)
    });
}

/// Retries a segment; the retry budget resets whenever an attempt made progress, so
/// long downloads on flaky networks only fail on repeated failures without progress.
async fn run_segment(ctx: &WorkerCtx, idx: usize) -> Result<(), EngineError> {
    let mut attempt = 0;
    loop {
        let before = ctx.segs.lock().unwrap()[idx].written;
        match fetch_segment(ctx, idx).await {
            Ok(()) => return Ok(()),
            Err(e) if e.is_retryable() => {
                if ctx.segs.lock().unwrap()[idx].written > before {
                    attempt = 0;
                }
                if attempt >= MAX_RETRIES {
                    return Err(e);
                }
                let delay = backoff(&e, ctx.retry_base, attempt, idx);
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

/// Streams this segment's remaining range into the part file. The segment's `end`
/// can shrink while we run (dynamic split), so it's re-read per chunk.
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
        _ => return Err(EngineError::from_response(&resp)),
    }
    // Trust the bytes only if the server says they start where we asked.
    let served = resp
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range);
    if served != Some((pos, ctx.size)) {
        return Err(EngineError::RangeNotHonored);
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
            tokio::select! {
                _ = ctx.cancel.cancelled() => break,
                _ = l.acquire(chunk.len() as u64) => {}
            }
        }
        let (take, segment_done) = commit_chunk(&mut file, &ctx.segs, idx, &chunk).await?;
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

/// `bytes 100-199/1000` → (100, 1000).
fn parse_content_range(value: &str) -> Option<(u64, u64)> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let start = range.split_once('-')?.0.trim().parse().ok()?;
    Some((start, total.trim().parse().ok()?))
}

/// Writes as much of `chunk` as segment `idx` still needs. The bytes are reserved as
/// in-flight (so a concurrent split can't hand them out) and only count as `written`
/// once the write has really completed; a failed write leaves nothing counted.
/// Returns (bytes taken, segment finished).
async fn commit_chunk(
    file: &mut tokio::fs::File,
    segs: &Mutex<Vec<Segment>>,
    idx: usize,
    chunk: &[u8],
) -> Result<(usize, bool), EngineError> {
    let take = {
        let mut s = segs.lock().unwrap();
        let take = (s[idx].remaining() as usize).min(chunk.len());
        s[idx].inflight += take as u64;
        take
    };
    // tokio reports write errors only on the next operation, so flush to surface them.
    let written = async {
        file.write_all(&chunk[..take]).await?;
        file.flush().await
    }
    .await;
    let mut s = segs.lock().unwrap();
    s[idx].inflight -= take as u64;
    written?;
    s[idx].written += take as u64;
    Ok((take, s[idx].remaining() == 0))
}

/// Servers without range support (or unknown size): one sequential stream.
/// Pausing discards the partial file, because it can't be resumed.
async fn single_stream(job: &Job<'_>, info: &RemoteInfo) -> Result<Outcome, EngineError> {
    let resp = job.client.get(&info.url).send().await?;
    if !resp.status().is_success() {
        return Err(EngineError::from_response(&resp));
    }
    let total = info.size.or(resp.content_length());
    let mut file = tokio::fs::File::create(&job.part).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0_u64;
    let mut last_tick = Instant::now();
    let mut last_bytes = 0_u64;
    loop {
        let next = tokio::select! {
            _ = job.cancel.cancelled() => None,
            c = stream.next() => c,
        };
        if job.cancel.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(&job.part).await;
            return Ok(Outcome::Paused);
        }
        let Some(chunk) = next else { break };
        let chunk = chunk?;
        if let Some(l) = &job.opts.limiter {
            tokio::select! {
                _ = job.cancel.cancelled() => continue,
                _ = l.acquire(chunk.len() as u64) => {}
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn commit_chunk_does_not_count_bytes_that_failed_to_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ro.bin");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        // A read-only handle: the write itself fails, but tokio only reports it on flush.
        let mut file = tokio::fs::File::from_std(std::fs::File::open(&path).unwrap());
        let segs = Mutex::new(vec![Segment::new(0, 1024)]);
        let result = commit_chunk(&mut file, &segs, 0, &[7u8; 512]).await;
        let seg = segs.lock().unwrap()[0].clone();
        assert!(result.is_err(), "write to read-only file must fail");
        assert_eq!(seg.written, 0, "failed bytes must not count as downloaded");
        assert_eq!(seg.pos(), 0, "nothing may stay reserved after a failed write");
    }

    #[test]
    fn parses_content_range() {
        assert_eq!(parse_content_range("bytes 100-199/1000"), Some((100, 1000)));
        assert_eq!(parse_content_range("bytes 0-0/*"), None);
        assert_eq!(parse_content_range("garbage"), None);
    }
}
