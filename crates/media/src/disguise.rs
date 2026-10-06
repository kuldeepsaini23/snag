//! Some streaming sites disguise each HLS segment as an image: a tiny PNG (signature … IEND)
//! is put in front of the real MPEG-TS data, to fool downloaders. Joined as-is, the pieces make
//! a file no player opens. This strips every fake header and leaves a normal `.ts` video.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const TS_PACKET: usize = 188;
const CHUNK: usize = 4 * 1024 * 1024;
/// A fake header is a few hundred bytes; never look further than this for its end.
const MAX_HEADER: usize = 64 * 1024;

/// If `path` is such a disguised stream, writes the real video next to it as `<stem>.ts`,
/// deletes the broken file and returns the new path. `Ok(None)`: not disguised (left alone).
pub fn unwrap(path: &Path) -> std::io::Result<Option<PathBuf>> {
    let mut input = File::open(path)?;
    let mut head = [0u8; 8];
    if input.read(&mut head)? < 8 || head != PNG {
        return Ok(None);
    }
    let out_path = path.with_extension("ts");
    let temp = path.with_extension("ts.part");
    let result = strip(File::open(path)?, &temp);
    match result {
        Ok(true) => {
            std::fs::rename(&temp, &out_path)?;
            std::fs::remove_file(path)?;
            Ok(Some(out_path))
        }
        Ok(false) => {
            let _ = std::fs::remove_file(&temp);
            Ok(None)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Copies `input` to `out` without the fake headers. False: the layout isn't what we expect
/// (a header with no video after it), so nothing should be changed.
fn strip(mut input: File, out: &Path) -> std::io::Result<bool> {
    let mut writer = BufWriter::new(File::create(out)?);
    let mut buf: Vec<u8> = Vec::with_capacity(CHUNK * 2);
    let mut eof = false;
    let mut chunk = vec![0u8; CHUNK];
    let mut wrote_any = false;
    loop {
        if !eof && buf.len() < CHUNK + MAX_HEADER {
            let n = input.read(&mut chunk)?;
            if n == 0 {
                eof = true;
            } else {
                buf.extend_from_slice(&chunk[..n]);
                continue;
            }
        }
        match memchr::memmem::find(&buf, PNG) {
            Some(start) => {
                writer.write_all(&buf[..start])?;
                wrote_any |= start > 0;
                let Some(video) = video_start(&buf[start..]) else {
                    if !eof && buf.len() - start < MAX_HEADER {
                        // The header's end isn't read yet.
                        buf.drain(..start);
                        let n = input.read(&mut chunk)?;
                        if n == 0 {
                            eof = true;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                        continue;
                    }
                    return Ok(false);
                };
                buf.drain(..start + video);
            }
            None if eof => {
                writer.write_all(&buf)?;
                wrote_any |= !buf.is_empty();
                break;
            }
            None => {
                // Keep a signature's worth of bytes: one may be split across reads.
                let keep = buf.len().saturating_sub(PNG.len() - 1);
                writer.write_all(&buf[..keep])?;
                wrote_any |= keep > 0;
                buf.drain(..keep);
            }
        }
    }
    writer.flush()?;
    Ok(wrote_any)
}

/// Where the video starts inside `bytes` (which begins with a fake PNG): the first MPEG-TS
/// packet after the PNG's IEND marker (sync byte 0x47, and again one packet later).
fn video_start(bytes: &[u8]) -> Option<usize> {
    let window = &bytes[..bytes.len().min(MAX_HEADER)];
    let iend = memchr::memmem::find(window, b"IEND")?;
    (iend + 4..(iend + 16).min(window.len())).find(|&k| bytes.get(k) == Some(&0x47) && bytes.get(k + TS_PACKET) == Some(&0x47))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake PNG header like the one sites put in front of each segment (IEND plus a
    /// truncated CRC, as seen in the wild).
    fn fake_png() -> Vec<u8> {
        let mut png = PNG.to_vec();
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xde]);
        png.extend_from_slice(&[0, 0, 0, 0]);
        png.extend_from_slice(b"IEND");
        png.push(0xae);
        png
    }

    fn segment(n: u8, packets: usize) -> Vec<u8> {
        let mut ts = Vec::new();
        for p in 0..packets {
            let mut packet = vec![n.wrapping_add(p as u8); TS_PACKET];
            packet[0] = 0x47;
            ts.extend_from_slice(&packet);
        }
        ts
    }

    #[test]
    fn disguised_segments_become_a_playable_ts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("clip [x].mp4");
        let (mut disguised, mut real) = (Vec::new(), Vec::new());
        // Enough segments that headers fall across the 4 MB reading chunks.
        for n in 0..40u8 {
            let ts = segment(n, 1200);
            disguised.extend_from_slice(&fake_png());
            disguised.extend_from_slice(&ts);
            real.extend_from_slice(&ts);
        }
        std::fs::write(&file, &disguised).unwrap();
        let fixed = unwrap(&file).unwrap().expect("repaired");
        assert_eq!(fixed, dir.path().join("clip [x].ts"));
        assert_eq!(std::fs::read(&fixed).unwrap(), real);
        assert!(!file.exists(), "the broken file is replaced");
        assert!(!dir.path().join("clip [x].ts.part").exists());
    }

    #[test]
    fn normal_videos_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let mp4 = dir.path().join("a.mp4");
        std::fs::write(&mp4, b"\0\0\0\x20ftypisom rest of a normal file").unwrap();
        assert_eq!(unwrap(&mp4).unwrap(), None);
        assert!(mp4.exists());
        // A real PNG (no video after it) is not touched either.
        let png = dir.path().join("b.mp4");
        let mut bytes = fake_png();
        bytes.extend_from_slice(&[1, 2, 3]);
        std::fs::write(&png, &bytes).unwrap();
        assert_eq!(unwrap(&png).unwrap(), None);
        assert_eq!(std::fs::read(&png).unwrap(), bytes);
        assert!(!dir.path().join("b.ts").exists());
    }
}
