//! Fake yt-dlp for tests: behaviour depends on the URL (last argument).
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.last().cloned().unwrap_or_default();
    if args.iter().any(|a| a == "-J") {
        println!(
            r#"{{"_type":"video","title":"Fake clip","duration":3.0,"formats":[{{"vcodec":"avc1","acodec":"none","height":480,"filesize":300}},{{"vcodec":"none","acodec":"mp4a","filesize":100}}]}}"#
        );
        return;
    }
    match url.as_str() {
        "fake://ok" => {
            println!("[generic] Extracting URL");
            println!("RDMP 100 1000 NA 50.5");
            println!("RDMP 1000 1000 NA 50");
            println!(r"RDMF C:\out\clip.mp4");
        }
        "fake://fail" => {
            eprintln!("WARNING: something minor");
            eprintln!("ERROR: Unsupported URL: fake://fail");
            std::process::exit(1);
        }
        "fake://slow" => {
            println!("RDMP 1 100 NA 1");
            std::thread::sleep(Duration::from_secs(30));
        }
        _ => std::process::exit(2),
    }
}
