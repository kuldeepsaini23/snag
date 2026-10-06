//! Fake yt-dlp for tests: behaviour depends on the URL (last argument).
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.last().cloned().unwrap_or_default();
    record(&args);
    let what = url.rsplit('/').next().unwrap_or_default().to_string();
    if let Some(dir) = args.windows(2).find(|w| w[0] == "-D").map(|w| w[1].clone()) {
        gallery(&what, std::path::Path::new(&dir));
        return;
    }
    if args.iter().any(|a| a == "-U") {
        println!("Current version: fake");
        println!("yt-dlp is up to date (fake)");
        return;
    }
    if args.iter().any(|a| a == "-J") {
        println!(
            r#"{{"_type":"video","title":"Fake clip","duration":3.0,"formats":[{{"vcodec":"avc1","acodec":"none","height":480,"filesize":300}},{{"vcodec":"none","acodec":"mp4a","filesize":100}}]}}"#
        );
        return;
    }
    match what.as_str() {
        "ok" => {
            // Leftover partial file in the temp folder, as a real run can leave behind.
            if let Some(temp) = args.iter().find_map(|a| a.strip_prefix("temp:")) {
                let _ = std::fs::create_dir_all(temp);
                let _ = std::fs::write(std::path::Path::new(temp).join("clip.f1.mp4.part"), b"x");
            }
            println!("[generic] Extracting URL");
            println!("RDMP 100 1000 NA 50.5");
            println!("RDMP 1000 1000 NA 50");
            println!(r"RDMF C:\out\clip.mp4");
        }
        "subfail" => {
            // The video is saved, but the subtitles were refused: yt-dlp exits 1 anyway.
            println!("RDMP 1000 1000 NA 50");
            println!(r"RDMF C:\out\clip.mp4");
            eprintln!("ERROR: Unable to download video subtitles for 'en': HTTP Error 429: Too Many Requests");
            std::process::exit(1);
        }
        "live" => {
            // A live stream: writes straight to its file and keeps going until stopped.
            let home = args.windows(2).find(|w| w[0] == "-P" && !w[1].starts_with("temp:")).map(|w| w[1].clone()).unwrap_or_default();
            let file = std::path::Path::new(&home).join("Launch [live].mp4");
            let _ = std::fs::write(&file, vec![7u8; 4096]);
            println!("[download] Destination: {}", file.display());
            println!("RDMP 4096 NA NA 1000");
            std::thread::sleep(Duration::from_secs(30));
        }
        "busy" => {
            eprintln!("ERROR: Unable to download webpage: HTTP Error 503: Service Unavailable");
            std::process::exit(1);
        }
        "flaky" => {
            // Fails once (like a server hiccup), then works.
            let marker = args.windows(2).find(|w| w[0] == "-P" && !w[1].starts_with("temp:")).map(|w| std::path::Path::new(&w[1]).join("flaky-once"));
            if let Some(marker) = marker.filter(|m| !m.exists()) {
                let _ = std::fs::write(marker, b"");
                eprintln!("ERROR: Unable to download webpage: HTTP Error 503: Service Unavailable");
                std::process::exit(1);
            }
            println!("RDMP 1000 1000 NA 50");
            println!(r"RDMF C:\out\clip.mp4");
        }
        "fail" => {
            eprintln!("WARNING: something minor");
            eprintln!("ERROR: Unsupported URL: fake://fail");
            std::process::exit(1);
        }
        "slow" => {
            println!("RDMP 1 100 NA 1");
            std::thread::sleep(Duration::from_secs(30));
        }
        _ => std::process::exit(2),
    }
}

/// Writes the arguments (and the cookies file's content) to `fake-args.txt` in the output folder.
fn record(args: &[String]) {
    let home = args.windows(2).find(|w| w[0] == "-P" && !w[1].starts_with("temp:")).map(|w| w[1].clone());
    let Some(home) = home else { return };
    let mut text = args.join("\n");
    if let Some(file) = args.windows(2).find(|w| w[0] == "--cookies").map(|w| w[1].clone()) {
        text.push_str("\nCOOKIES:\n");
        text.push_str(&std::fs::read_to_string(file).unwrap_or_default());
    }
    if let Some(temp) = args.iter().find_map(|a| a.strip_prefix("temp:")) {
        let _ = std::fs::create_dir_all(temp);
        let _ = std::fs::write(std::path::Path::new(temp).join("fake-args.txt"), &text);
    }
    let _ = std::fs::write(std::path::Path::new(&home).join("fake-args.txt"), text);
}

/// Pretends to be gallery-dl: prints one path per image (skipped ones start with "# ").
fn gallery(what: &str, dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    match what {
        "ok" => {
            let old = dir.join("0.jpg");
            let _ = std::fs::write(&old, vec![0u8; 50]);
            println!("# {}", old.display());
            for n in 1..=3 {
                let file = dir.join(format!("{n}.jpg"));
                let _ = std::fs::write(&file, vec![1u8; 100]);
                println!("{}", file.display());
            }
            // Real gallery-dl reports an image again ("# …") when a page lists it twice.
            println!("# {}", dir.join("1.jpg").display());
        }
        "empty" => {}
        "slow" => {
            let file = dir.join("1.jpg");
            let _ = std::fs::write(&file, vec![1u8; 100]);
            println!("{}", file.display());
            std::thread::sleep(Duration::from_secs(30));
        }
        _ => {
            eprintln!("[gallery-dl][error] No suitable extractor found for 'fake://{what}'");
            std::process::exit(64);
        }
    }
}
