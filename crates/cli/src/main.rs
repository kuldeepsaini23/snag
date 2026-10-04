use clap::Parser;
use rdm_engine::filename::resume_target;
use rdm_engine::{CancellationToken, DownloadOptions, Outcome, Progress, RateLimiter, default_client, download, probe};
use std::io::Write;
use std::path::PathBuf;
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
    let dest = resume_target(&args.dir, &info.filename, &args.url);
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
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
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
