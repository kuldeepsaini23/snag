//! ffmpeg on demand: joins HD video with its sound, makes MP3s and runs the conversion rules.
//! Not shipped with Snag (most downloads never need it); fetched once, checked against the
//! published SHA-256, and only `ffmpeg.exe` and `ffprobe.exe` are kept.

use std::path::Path;

/// gyan.dev's "essentials" build: a 35 MB .7z with the codecs Snag uses.
pub const URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z";
/// Its SHA-256, published next to it.
pub const SHA256_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z.sha256";

/// The programs taken from the archive (they sit in a versioned `…/bin/` folder inside it).
const WANTED: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];

/// The hash in a `.sha256` file ("<hex>" or "<hex>  <name>"), lower-cased; `None` if it isn't one.
pub fn parse_sha256(text: &str) -> Option<String> {
    let hex = text.split_whitespace().next()?;
    (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| hex.to_ascii_lowercase())
}

/// Unpacks `ffmpeg.exe` and `ffprobe.exe` from the archive straight into `bin` (nothing else).
pub fn unpack(archive: &Path, bin: &Path) -> Result<(), String> {
    std::fs::create_dir_all(bin).map_err(|e| format!("can't create {}: {e}", bin.display()))?;
    let mut found = 0;
    sevenz_rust::decompress_file_with_extract_fn(archive, bin, |entry, reader, _| {
        let name = entry.name().rsplit(['/', '\\']).next().unwrap_or("");
        let in_bin = entry.name().replace('\\', "/").contains("/bin/");
        if let Some(wanted) = WANTED.iter().find(|w| in_bin && name.eq_ignore_ascii_case(w)) {
            let mut file = std::fs::File::create(bin.join(wanted)).map_err(sevenz_rust::Error::io)?;
            std::io::copy(reader, &mut file).map_err(sevenz_rust::Error::io)?;
            found += 1;
        } else {
            std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
        }
        Ok(true)
    })
    .map_err(|e| format!("couldn't unpack ffmpeg: {e}"))?;
    if found < WANTED.len() {
        return Err("the ffmpeg download didn't contain ffmpeg.exe and ffprobe.exe".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_published_checksum() {
        let hex = "a".repeat(64);
        assert_eq!(parse_sha256(&hex), Some(hex.clone()));
        assert_eq!(parse_sha256(&format!("{}  ffmpeg-release-essentials.7z\n", "AB".repeat(32))), Some("ab".repeat(32)));
        assert_eq!(parse_sha256("<html>not found</html>"), None);
        assert_eq!(parse_sha256(""), None);
    }

    #[test]
    fn only_the_two_programs_are_unpacked() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ff.7z");
        let mut sz = sevenz_rust::SevenZWriter::create(&archive).unwrap();
        for (name, data) in [
            ("ffmpeg-8.0-essentials_build/bin/ffmpeg.exe", &b"MZ ffmpeg"[..]),
            ("ffmpeg-8.0-essentials_build/bin/ffprobe.exe", &b"MZ ffprobe"[..]),
            ("ffmpeg-8.0-essentials_build/bin/ffplay.exe", &b"MZ ffplay"[..]),
            ("ffmpeg-8.0-essentials_build/doc/readme.txt", &b"docs"[..]),
        ] {
            let mut entry = sevenz_rust::SevenZArchiveEntry::new();
            entry.name = name.to_string();
            entry.has_stream = true;
            entry.size = data.len() as u64;
            sz.push_archive_entry(entry, Some(data)).unwrap();
        }
        sz.finish().unwrap();
        let bin = dir.path().join("bin");
        unpack(&archive, &bin).unwrap();
        assert_eq!(std::fs::read(bin.join("ffmpeg.exe")).unwrap(), b"MZ ffmpeg");
        assert_eq!(std::fs::read(bin.join("ffprobe.exe")).unwrap(), b"MZ ffprobe");
        let mut names: Vec<_> = std::fs::read_dir(&bin).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        assert_eq!(names, ["ffmpeg.exe", "ffprobe.exe"], "no player, no docs, no folders");
    }

    #[test]
    fn an_archive_without_ffmpeg_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("other.7z");
        let mut sz = sevenz_rust::SevenZWriter::create(&archive).unwrap();
        let mut entry = sevenz_rust::SevenZArchiveEntry::new();
        entry.name = "readme.txt".into();
        entry.has_stream = true;
        entry.size = 2;
        sz.push_archive_entry(entry, Some(&b"hi"[..])).unwrap();
        sz.finish().unwrap();
        assert!(unpack(&archive, &dir.path().join("bin")).is_err());
    }
}
