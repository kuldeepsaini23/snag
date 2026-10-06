//! Fake yt-dlp for tests: behaviour depends on the URL (last argument).
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.last().cloned().unwrap_or_default();
    record(&args);
    let what = url.rsplit('/').next().unwrap_or_default().to_string();
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
    let _ = std::fs::write(std::path::Path::new(&home).join("fake-args.txt"), text);
}
