//! ffmpeg on demand: joins HD video with its sound, makes MP3s and runs the conversion rules.
//! Not shipped with Snag (most downloads never need it); fetched once, checked against the
//! published SHA-256, and only `ffmpeg` and `ffprobe` are kept (`.exe` on Windows).
//! Windows and Linux fetch one archive holding both; macOS one zip for each.

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
#[cfg(all(unix, not(target_os = "macos")))]
const FILE: &str = "ffmpeg-master-latest-linux64-gpl.tar.xz";
#[cfg(all(unix, not(target_os = "macos")))]
pub const URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz";
#[cfg(all(unix, not(target_os = "macos")))]
pub const SHA256_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256";
#[cfg(all(unix, not(target_os = "macos")))]
pub const ARCHIVE: &str = "ffmpeg-download.tar.xz";

/// macOS: Martin Riedl's static builds (signed and notarized), a zip per program for this Mac's
/// processor. The link leads to the newest release; its SHA-256 sits next to the file it leads
/// to (`…/ffmpeg.zip.sha256`), so the checksum always belongs to the very file downloaded.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RIEDL: [&str; 2] = [
    "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip",
    "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffprobe.zip",
];
#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
const RIEDL: [&str; 2] = [
    "https://ffmpeg.martin-riedl.de/redirect/latest/macos/amd64/release/ffmpeg.zip",
    "https://ffmpeg.martin-riedl.de/redirect/latest/macos/amd64/release/ffprobe.zip",
];
#[cfg(target_os = "macos")]
pub const ARCHIVE: &str = "ffmpeg-download.zip";

/// One archive to fetch: its link, where its SHA-256 is published (`None`: next to the file the
/// link leads to, as `<file>.sha256`), and its name in `bin` while it's checked and unpacked.
pub struct Source {
    pub url: &'static str,
    pub sha256_url: Option<&'static str>,
    pub archive: &'static str,
}

/// What `ensure_ffmpeg` fetches, in order.
#[cfg(not(target_os = "macos"))]
pub const SOURCES: &[Source] = &[Source { url: URL, sha256_url: Some(SHA256_URL), archive: ARCHIVE }];
#[cfg(target_os = "macos")]
pub const SOURCES: &[Source] = &[
    Source { url: RIEDL[0], sha256_url: None, archive: ARCHIVE },
    Source { url: RIEDL[1], sha256_url: None, archive: "ffprobe-download.zip" },
];

/// The programs taken from the archive (they sit in a versioned `…/bin/` folder inside it).
#[cfg_attr(not(windows), allow(dead_code))]
const WINDOWS_WANTED: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];
#[cfg(unix)]
const WANTED: [&str; 2] = ["ffmpeg", "ffprobe"];

/// The hash in a `.sha256` file ("<hex>" or "<hex>  <name>"), lower-cased; `None` if it isn't one.
pub fn parse_sha256(text: &str) -> Option<String> {
    let hex = text.split_whitespace().next()?;
    (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| hex.to_ascii_lowercase())
}

/// The archive's SHA-256 from what `SHA256_URL` answered.
pub fn expected_sha256(text: &str) -> Option<String> {
    #[cfg(any(windows, target_os = "macos"))]
    return parse_sha256(text);
    #[cfg(all(unix, not(target_os = "macos")))]
    return crate::selfupdate::checksum_for(text, FILE);
}

/// Unpacks `ffmpeg` and `ffprobe` from the archive straight into `bin` (nothing else; on macOS
/// each zip holds one of them).
pub fn unpack(archive: &Path, bin: &Path) -> Result<(), String> {
    #[cfg(windows)]
    return unpack_7z(archive, bin, &WINDOWS_WANTED);
    #[cfg(all(unix, not(target_os = "macos")))]
    return unpack_tar_xz(archive, bin, &WANTED);
    #[cfg(target_os = "macos")]
    return unpack_zip(archive, bin, &WANTED);
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

/// macOS: whichever of the programs the zip holds, made executable; an error if it holds none.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn unpack_zip(archive: &Path, bin: &Path, wanted_names: &[&str]) -> Result<(), String> {
    std::fs::create_dir_all(bin).map_err(|e| format!("can't create {}: {e}", bin.display()))?;
    let unpack_err = |e: &dyn std::fmt::Display| format!("couldn't unpack ffmpeg: {e}");
    let file = std::fs::File::open(archive).map_err(|e| unpack_err(&e))?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| unpack_err(&e))?;
    let mut found = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| unpack_err(&e))?;
        let name = entry.name().rsplit('/').next().unwrap_or("").to_string();
        let Some(wanted) = wanted_names.iter().find(|w| entry.is_file() && name == **w) else { continue };
        let to = bin.join(wanted);
        let mut out = std::fs::File::create(&to).map_err(|e| unpack_err(&e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| unpack_err(&e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755)).map_err(|e| unpack_err(&e))?;
        }
        found += 1;
    }
    if found == 0 {
        return Err(format!("the ffmpeg download didn't contain {}", wanted_names.join(" or ")));
    }
    Ok(())
}

/// Linux: the same from a `.tar.xz`, made executable.
#[cfg(all(unix, not(target_os = "macos")))]
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

    #[test]
    fn a_mac_zip_gives_its_one_program() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ffprobe.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        let opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        for (name, data) in [("ffprobe", &b"MACHO ffprobe"[..]), ("readme.txt", &b"docs"[..])] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        let bin = dir.path().join("bin");
        unpack_zip(&archive, &bin, &["ffmpeg", "ffprobe"]).unwrap();
        assert_eq!(std::fs::read(bin.join("ffprobe")).unwrap(), b"MACHO ffprobe");
        let names: Vec<_> = std::fs::read_dir(&bin).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        assert_eq!(names, ["ffprobe"], "no docs");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(bin.join("ffprobe")).unwrap().permissions().mode() & 0o111, 0o111, "runnable");
        }
        assert!(unpack_zip(&archive, &dir.path().join("bin2"), &["ffmpeg"]).is_err(), "nothing wanted inside");
        assert_eq!(parse_sha256(&format!("{}  ffmpeg.zip\n", "c".repeat(64))), Some("c".repeat(64)), "the .sha256 next to the zip");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
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
