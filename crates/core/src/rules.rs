//! After-download rules: when a finished download matches (by extension, site or category),
//! Snag converts it to MP3, shrinks a video, unpacks an archive or moves the file.

use crate::Category;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: u32,
    pub enabled: bool,
    pub when: Match,
    pub action: Action,
    /// Keep the downloaded file next to the result (else it is replaced or removed).
    pub keep_original: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Match {
    /// File extensions, comma separated: "zip, 7z".
    Ext(String),
    /// A site and its subdomains: "youtube.com".
    Site(String),
    Category(Category),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    ToMp3,
    /// Re-encode with x264 at this quality (18 = near original … 32 = small).
    SmallerMp4 { crf: u8 },
    /// Unpack a .zip or .7z into a folder named after it.
    Extract,
    MoveTo(PathBuf),
}

/// The first enabled rule that fits a finished download.
pub fn matching<'a>(rules: &'a [Rule], url: &str, file: &Path, category: Category) -> Option<&'a Rule> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_ascii_lowercase)).unwrap_or_default();
    rules.iter().filter(|r| r.enabled).find(|r| match &r.when {
        Match::Ext(list) => !ext.is_empty() && list.split(',').any(|e| e.trim().trim_start_matches('.').eq_ignore_ascii_case(&ext)),
        Match::Site(site) => {
            let site = site.trim().to_ascii_lowercase();
            let site = site.trim_start_matches("www.");
            !site.is_empty() && (host == site || host.ends_with(&format!(".{site}")))
        }
        Match::Category(c) => *c == category,
    })
}

/// What a rule did: the file the download now points at.
#[derive(Debug, PartialEq)]
pub struct Applied {
    pub result: PathBuf,
    pub note: String,
}

/// Runs `rule` on `file` (blocking; call from a worker thread). On failure the original file is
/// always left as it was.
pub fn apply(rule: &Rule, file: &Path, ffmpeg: Option<&Path>) -> Result<Applied, String> {
    let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "download".into());
    let dir = file.parent().unwrap_or(Path::new("."));
    let result = match &rule.action {
        Action::ToMp3 => {
            let out = free_path(dir.join(format!("{stem}.mp3")));
            convert(ffmpeg, file, &out, &["-vn", "-c:a", "libmp3lame", "-b:a", "192k"])?;
            out
        }
        Action::SmallerMp4 { crf } => {
            let out = free_path(dir.join(format!("{stem} (smaller).mp4")));
            let crf = crf.clamp(&18, &40).to_string();
            convert(ffmpeg, file, &out, &["-c:v", "libx264", "-preset", "veryfast", "-crf", &crf, "-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart"])?;
            out
        }
        Action::Extract => {
            let out = free_path(dir.join(&stem));
            if let Err(e) = extract(file, &out) {
                let _ = std::fs::remove_dir_all(&out);
                return Err(e);
            }
            out
        }
        Action::MoveTo(target) => {
            std::fs::create_dir_all(target).map_err(|e| format!("can't create {}: {e}", target.display()))?;
            let name = file.file_name().ok_or("the file has no name")?;
            let out = free_path(target.join(name));
            let mut note = format!("Moved to {}", target.display());
            if rule.keep_original {
                std::fs::copy(file, &out).map_err(|e| format!("can't copy to {}: {e}", target.display()))?;
                note = format!("Copied to {}", target.display());
            } else if std::fs::rename(file, &out).is_err() {
                // Another drive, or the file is in use: copy, then remove the original if possible.
                std::fs::copy(file, &out).map_err(|e| format!("can't move to {}: {e}", target.display()))?;
                if std::fs::remove_file(file).is_err() {
                    note = format!("Copied to {} (the original is in use, so it stays too)", target.display());
                }
            }
            return Ok(Applied { note, result: out });
        }
    };
    // The result is safe on disk: now the original may go.
    if !rule.keep_original {
        let _ = std::fs::remove_file(file);
    }
    let note = match &rule.action {
        Action::ToMp3 => "Converted to MP3".into(),
        Action::SmallerMp4 { .. } => "Made a smaller MP4".into(),
        Action::Extract => format!("Unpacked into {}", result.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
        Action::MoveTo(_) => unreachable!(),
    };
    Ok(Applied { result, note })
}

/// `path`, or `name (2).ext`, `name (3).ext`… if it exists.
fn free_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).expect("a free name")
}

/// ffmpeg from `in` to `out`; a failed run leaves no `out` behind.
fn convert(ffmpeg: Option<&Path>, input: &Path, out: &Path, args: &[&str]) -> Result<(), String> {
    let ffmpeg = ffmpeg.ok_or("this rule needs ffmpeg, which isn't installed")?;
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-i"]).arg(input).args(args).arg(out);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let failed = match cmd.output() {
        Ok(o) if o.status.success() && out.exists() => return Ok(()),
        Ok(o) => format!("ffmpeg failed: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("").trim()),
        Err(e) => format!("couldn't run ffmpeg: {e}"),
    };
    let _ = std::fs::remove_file(out);
    Err(failed)
}

/// Unpacks a .zip or .7z into `out`. Entries that would land outside it are skipped.
fn extract(archive: &Path, out: &Path) -> Result<(), String> {
    let ext = archive.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    std::fs::create_dir_all(out).map_err(|e| format!("can't create {}: {e}", out.display()))?;
    match ext.as_str() {
        "zip" => {
            let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("not a readable zip: {e}"))?;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
                // `enclosed_name` refuses absolute paths and `..`.
                let Some(rel) = entry.enclosed_name() else { continue };
                let target = out.join(rel);
                if entry.is_dir() {
                    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                    continue;
                }
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut file = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        // Each entry is checked before it is written: no `..`, drive letters or absolute paths
        // (the library would join them onto `out` as they are).
        "7z" => sevenz_rust::decompress_file_with_extract_fn(archive, out, |entry, reader, dest| {
            let inside = entry.name().split(['/', '\\']).filter(|p| !p.is_empty()).all(rdm_torrent::safe_component);
            if !inside || entry.name().starts_with(['/', '\\']) {
                std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                return Ok(true);
            }
            sevenz_rust::default_entry_extract_fn(entry, reader, dest)
        })
        .map_err(|e| format!("not a readable 7z: {e}")),
        _ => Err(format!("Snag can unpack .zip and .7z, not .{ext}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(when: Match, action: Action, keep_original: bool) -> Rule {
        Rule { id: 1, enabled: true, when, action, keep_original }
    }

    #[test]
    fn rules_match_by_extension_site_or_category() {
        let rules = vec![
            Rule { enabled: false, ..rule(Match::Ext("zip".into()), Action::ToMp3, false) },
            rule(Match::Ext(" ZIP, 7z ".into()), Action::Extract, true),
            rule(Match::Site("youtube.com".into()), Action::ToMp3, true),
            rule(Match::Category(Category::Program), Action::MoveTo("C:\\Programs".into()), false),
        ];
        let first = |url: &str, name: &str| matching(&rules, url, Path::new(name), Category::from_name(name)).map(|r| r.action.clone());
        assert_eq!(first("https://x.com/a.zip", "a.zip"), Some(Action::Extract), "disabled rules are skipped");
        assert_eq!(first("https://x.com/b.7Z", "b.7Z"), Some(Action::Extract));
        assert_eq!(first("https://music.youtube.com/watch?v=1", "song.webm"), Some(Action::ToMp3), "subdomains count");
        assert_eq!(first("https://notyoutube.com/v", "v.mp4"), None, "a lookalike site doesn't");
        assert_eq!(first("https://x.com/setup.exe", "setup.exe"), Some(Action::MoveTo("C:\\Programs".into())));
        assert_eq!(first("https://x.com/doc.pdf", "doc.pdf"), None);
    }

    fn zip_with(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut zip, data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn archives_unpack_into_a_folder_of_their_name() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("photos 2026.zip");
        zip_with(&archive, &[("a.txt", b"one"), ("sub/b.txt", b"two"), ("../escape.txt", b"no")]);
        let done = apply(&rule(Match::Ext("zip".into()), Action::Extract, false), &archive, None).unwrap();
        let folder = dir.path().join("photos 2026");
        assert_eq!(done.result, folder);
        assert_eq!(std::fs::read(folder.join("a.txt")).unwrap(), b"one");
        assert_eq!(std::fs::read(folder.join("sub").join("b.txt")).unwrap(), b"two");
        assert!(!dir.path().join("escape.txt").exists(), "nothing lands outside the folder");
        assert!(!archive.exists(), "keep original off: the archive goes");
    }

    #[test]
    fn a_7z_cannot_write_outside_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("pack.7z");
        let mut sz = sevenz_rust::SevenZWriter::create(&archive).unwrap();
        // An absolute name, pointing at a harmless place inside the test's own folder.
        let absolute = dir.path().join("abs-escape.txt").display().to_string();
        for (name, data) in [("a.txt", &b"one"[..]), ("../escape.txt", &b"no"[..]), (r"..\escape2.txt", &b"no"[..]), (absolute.as_str(), &b"no"[..])] {
            let mut entry = sevenz_rust::SevenZArchiveEntry::new();
            entry.name = name.to_string();
            entry.has_stream = true;
            entry.size = data.len() as u64;
            sz.push_archive_entry(entry, Some(data)).unwrap();
        }
        sz.finish().unwrap();
        let done = apply(&rule(Match::Ext("7z".into()), Action::Extract, true), &archive, None).unwrap();
        assert_eq!(std::fs::read(done.result.join("a.txt")).unwrap(), b"one");
        assert!(!dir.path().join("escape.txt").exists(), "../ stays inside");
        assert!(!dir.path().join("escape2.txt").exists(), r"..\ stays inside");
        assert!(!dir.path().join("abs-escape.txt").exists(), "absolute paths are refused");
    }

    #[test]
    fn keep_original_is_honoured() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        zip_with(&archive, &[("x.txt", b"x")]);
        apply(&rule(Match::Ext("zip".into()), Action::Extract, true), &archive, None).unwrap();
        assert!(archive.exists());
        let file = dir.path().join("setup.exe");
        std::fs::write(&file, b"exe").unwrap();
        let to = dir.path().join("Programs");
        let moved = apply(&rule(Match::Ext("exe".into()), Action::MoveTo(to.clone()), true), &file, None).unwrap();
        assert_eq!(moved.result, to.join("setup.exe"));
        assert!(file.exists() && moved.result.exists(), "copied, not moved");
        let again = dir.path().join("tool.exe");
        std::fs::write(&again, b"exe").unwrap();
        let moved = apply(&rule(Match::Ext("exe".into()), Action::MoveTo(to.clone()), false), &again, None).unwrap();
        assert!(!again.exists() && moved.result.exists(), "moved");
    }

    #[test]
    fn a_move_that_cannot_remove_the_original_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("busy.exe");
        std::fs::write(&file, b"exe").unwrap();
        // Another program has it open (e.g. a torrent still sharing it): Windows won't move it.
        #[cfg(windows)]
        let _held = {
            use std::os::windows::fs::OpenOptionsExt;
            // Read and write sharing only (no delete), like most programs open files.
            std::fs::OpenOptions::new().read(true).share_mode(0x1 | 0x2).open(&file).unwrap()
        };
        let to = dir.path().join("Programs");
        let done = apply(&rule(Match::Ext("exe".into()), Action::MoveTo(to.clone()), false), &file, None).unwrap();
        assert!(to.join("busy.exe").exists(), "copied over");
        assert!(file.exists(), "the original is still in use");
        assert!(done.note.contains("in use"), "the notice doesn't claim a move: {}", done.note);
    }

    #[test]
    fn a_failed_conversion_keeps_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"not really a video").unwrap();
        let mp3 = rule(Match::Ext("mp4".into()), Action::ToMp3, false);
        let e = apply(&mp3, &video, None).unwrap_err();
        assert!(e.contains("ffmpeg"), "says what's missing: {e}");
        assert!(video.exists());
        // ffmpeg that fails (here: not a real program).
        let broken = dir.path().join("ffmpeg.exe");
        std::fs::write(&broken, b"nope").unwrap();
        assert!(apply(&mp3, &video, Some(&broken)).is_err());
        assert_eq!(std::fs::read(&video).unwrap(), b"not really a video");
        assert!(!dir.path().join("clip.mp3").exists(), "no half-written result left behind");
    }
}

/// Real ffmpeg from PATH. Run with: cargo test -p rdm-core --lib real_ffmpeg -- --ignored
#[cfg(test)]
mod real {
    use super::*;

    #[test]
    #[ignore]
    fn real_ffmpeg_converts_and_shrinks() {
        let ffmpeg = PathBuf::from("ffmpeg");
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("clip.mp4");
        let made = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=duration=2:size=640x360", "-f", "lavfi", "-i", "sine=duration=2", "-shortest"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(made.success());
        let rule = |action| Rule { id: 1, enabled: true, when: Match::Ext("mp4".into()), action, keep_original: true };
        let mp3 = apply(&rule(Action::ToMp3), &clip, Some(&ffmpeg)).unwrap();
        assert_eq!(mp3.result, dir.path().join("clip.mp3"));
        assert!(std::fs::metadata(&mp3.result).unwrap().len() > 1000);
        let small = apply(&rule(Action::SmallerMp4 { crf: 32 }), &clip, Some(&ffmpeg)).unwrap();
        assert_eq!(small.result, dir.path().join("clip (smaller).mp4"));
        assert!(clip.exists(), "kept");
    }
}
