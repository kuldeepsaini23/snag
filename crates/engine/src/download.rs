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
