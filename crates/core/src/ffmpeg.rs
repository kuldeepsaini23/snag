//! ffmpeg on demand: joins HD video with its sound, makes MP3s and runs the conversion rules.
//! Not shipped with Snag (most downloads never need it); fetched once, checked against the
//! published SHA-256, and only `ffmpeg` and `ffprobe` are kept (`.exe` on Windows).

use std::path::Path;

/// gyan.dev's "essentials" build: a 35 MB .7z with the codecs Snag uses.
#[cfg(windows)]
pub const URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z";
/// Its SHA-256, published next to it.
#[cfg(windows)]
pub const SHA256_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z.sha256";
/// Where the archive is kept while it's checked and unpacked (inside `bin`).
#[cfg(windows)]
pub const ARCHIVE: &str = "ffmpeg-download.7z";

/// Linux: BtbN's static build (GitHub), with a SHA-256 for every file in `checksums.sha256`.
#[cfg(not(windows))]
const FILE: &str = "ffmpeg-master-latest-linux64-gpl.tar.xz";
#[cfg(not(windows))]
pub const URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz";
#[cfg(not(windows))]
pub const SHA256_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256";
#[cfg(not(windows))]
pub const ARCHIVE: &str = "ffmpeg-download.tar.xz";

/// The programs taken from the archive (they sit in a versioned `…/bin/` folder inside it).
#[cfg_attr(not(windows), allow(dead_code))]
const WINDOWS_WANTED: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];
#[cfg(not(windows))]
const WANTED: [&str; 2] = ["ffmpeg", "ffprobe"];

/// The hash in a `.sha256` file ("<hex>" or "<hex>  <name>"), lower-cased; `None` if it isn't one.
pub fn parse_sha256(text: &str) -> Option<String> {
    let hex = text.split_whitespace().next()?;
    (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| hex.to_ascii_lowercase())
}

/// The archive's SHA-256 from what `SHA256_URL` answered.
pub fn expected_sha256(text: &str) -> Option<String> {
    #[cfg(windows)]
    return parse_sha256(text);
    #[cfg(not(windows))]
    return crate::selfupdate::checksum_for(text, FILE);
}

/// Unpacks `ffmpeg` and `ffprobe` from the archive straight into `bin` (nothing else).
pub fn unpack(archive: &Path, bin: &Path) -> Result<(), String> {
    #[cfg(windows)]
    return unpack_7z(archive, bin, &WINDOWS_WANTED);
    #[cfg(not(windows))]
    return unpack_tar_xz(archive, bin, &WANTED);
}

#[cfg_attr(not(windows), allow(dead_code))]
fn unpack_7z(archive: &Path, bin: &Path, wanted_names: &[&str]) -> Result<(), String> {
    std::fs::create_dir_all(bin).map_err(|e| format!("can't create {}: {e}", bin.display()))?;
    let mut found = 0;
    sevenz_rust::decompress_file_with_extract_fn(archive, bin, |entry, reader, _| {
        let name = entry.name().rsplit(['/', '\\']).next().unwrap_or("");
        let in_bin = entry.name().replace('\\', "/").contains("/bin/");
        if let Some(wanted) = wanted_names.iter().find(|w| in_bin && name.eq_ignore_ascii_case(w)) {
            let mut file = std::fs::File::create(bin.join(wanted)).map_err(sevenz_rust::Error::io)?;
            std::io::copy(reader, &mut file).map_err(sevenz_rust::Error::io)?;
            found += 1;
        } else {
            std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
        }
        Ok(true)
    })
    .map_err(|e| format!("couldn't unpack ffmpeg: {e}"))?;
    if found < wanted_names.len() {
        return Err(format!("the ffmpeg download didn't contain {}", wanted_names.join(" and ")));
    }
    Ok(())
}

/// Linux: the same from a `.tar.xz`, made executable.
#[cfg(not(windows))]
fn unpack_tar_xz(archive: &Path, bin: &Path, wanted_names: &[&str]) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(bin).map_err(|e| format!("can't create {}: {e}", bin.display()))?;
    let unpack_err = |e: std::io::Error| format!("couldn't unpack ffmpeg: {e}");
    let file = std::fs::File::open(archive).map_err(unpack_err)?;
    let mut tar = tar::Archive::new(xz2::read::XzDecoder::new(std::io::BufReader::new(file)));
    let mut found = 0;
    for entry in tar.entries().map_err(unpack_err)? {
        let mut entry = entry.map_err(unpack_err)?;
        let path = entry.path().map_err(unpack_err)?.to_string_lossy().into_owned();
        let name = path.rsplit('/').next().unwrap_or("");
        let in_bin = path.contains("/bin/");
        if let Some(wanted) = wanted_names.iter().find(|w| in_bin && entry.header().entry_type().is_file() && name == **w) {
            let to = bin.join(wanted);
            let mut out = std::fs::File::create(&to).map_err(unpack_err)?;
            std::io::copy(&mut entry, &mut out).map_err(unpack_err)?;
            std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755)).map_err(unpack_err)?;
            found += 1;
        }
    }
    if found < wanted_names.len() {
        return Err(format!("the ffmpeg download didn't contain {}", wanted_names.join(" and ")));
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
        unpack_7z(&archive, &bin, &WINDOWS_WANTED).unwrap();
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
        assert!(unpack_7z(&archive, &dir.path().join("bin"), &WINDOWS_WANTED).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn linux_takes_the_two_programs_from_the_tar_xz() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ff.tar.xz");
        let mut tar = tar::Builder::new(xz2::write::XzEncoder::new(std::fs::File::create(&archive).unwrap(), 1));
        for (name, data) in [
            ("ffmpeg-master-latest-linux64-gpl/bin/ffmpeg", &b"ELF ffmpeg"[..]),
            ("ffmpeg-master-latest-linux64-gpl/bin/ffprobe", &b"ELF ffprobe"[..]),
            ("ffmpeg-master-latest-linux64-gpl/bin/ffplay", &b"ELF ffplay"[..]),
            ("ffmpeg-master-latest-linux64-gpl/doc/ffmpeg", &b"docs"[..]),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, data).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
        let bin = dir.path().join("bin");
        unpack(&archive, &bin).unwrap();
        assert_eq!(std::fs::read(bin.join("ffmpeg")).unwrap(), b"ELF ffmpeg");
        assert_eq!(std::fs::read(bin.join("ffprobe")).unwrap(), b"ELF ffprobe");
        assert_eq!(std::fs::metadata(bin.join("ffmpeg")).unwrap().permissions().mode() & 0o111, 0o111, "runnable");
        let mut names: Vec<_> = std::fs::read_dir(&bin).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        assert_eq!(names, ["ffmpeg", "ffprobe"], "no player, no docs, no folders");
        let sums = format!("{}  ffmpeg-master-latest-linux64-gpl-shared.tar.xz\n{}  ffmpeg-master-latest-linux64-gpl.tar.xz\n", "1".repeat(64), "2".repeat(64));
        assert_eq!(expected_sha256(&sums), Some("2".repeat(64)), "the line for this very file");
    }
}
