//! Files dropped on the window: a `.txt` list of links, a `.torrent` file or an Internet
//! shortcut (`.url`) become links to add. (winit only delivers dropped files, not dragged text.)

use std::path::Path;

/// More than this many links in one file are cut off (a dropped log file isn't a download list).
pub const MAX_LINKS: usize = 500;
/// Files bigger than this aren't read.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// The links in a dropped file, or why there are none (shown as a notice).
pub fn read(path: &Path) -> Result<Vec<String>, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "txt" | "torrent" | "url") {
        return Err("Drop a list of links (.txt), a .torrent file or a web shortcut (.url) to download it".into());
    }
    let size = std::fs::metadata(path).map_err(|e| format!("Couldn't read {name}: {e}"))?.len();
    if size > MAX_BYTES {
        return Err(format!("{name} is too big to be a list of links"));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {name}: {e}"))?;
    let found = match ext.as_str() {
        "torrent" => return torrent_magnet(&bytes).map(|m| vec![m]).ok_or_else(|| format!("{name} isn't a valid torrent file")),
        "url" => url_shortcut(&String::from_utf8_lossy(&bytes)).into_iter().collect(),
        _ => links_in(&String::from_utf8_lossy(&bytes)),
    };
    if found.is_empty() {
        return Err(format!("No web links found in {name}"));
    }
    Ok(found.into_iter().take(MAX_LINKS).collect())
}

/// Every http(s) and magnet link in `text`, in order, once each. Punctuation a sentence puts
/// right after a link ("see https://x/a.zip.") isn't part of it.
pub fn links_in(text: &str) -> Vec<String> {
    const SCHEMES: [&str; 3] = ["https://", "http://", "magnet:?"];
    let mut links: Vec<String> = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"')) {
        let lower = word.to_ascii_lowercase();
        let Some((start, scheme)) = SCHEMES.iter().filter_map(|s| lower.find(s).map(|i| (i, *s))).min_by_key(|(i, _)| *i) else { continue };
        let link = word[start..].trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}', '\'']);
        if link.len() <= scheme.len() {
            continue;
        }
        // "HTTPS://" works too: the rest of Snag expects the scheme in lower case.
        let link = format!("{scheme}{}", &link[scheme.len()..]);
        if !links.contains(&link) {
            links.push(link);
        }
    }
    links
}

/// The link in an Internet shortcut (`[InternetShortcut]` … `URL=…`).
pub fn url_shortcut(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.trim_start().get(..4).is_some_and(|k| k.eq_ignore_ascii_case("url=")))?;
    links_in(&line.trim_start()[4..]).into_iter().find(|l| l.starts_with("http"))
}

/// A `.torrent` file as a magnet link: its info-hash, name and trackers.
pub fn torrent_magnet(bytes: &[u8]) -> Option<String> {
    let (Value::Dict(top), end) = parse(bytes, 0, 0)? else { return None };
    if end != bytes.len() {
        return None;
    }
    let get = |entries: &[(&[u8], Value<'_>, std::ops::Range<usize>)], key: &[u8]| entries.iter().position(|(k, _, _)| *k == key);
    let info = &top[get(&top, b"info")?];
    let Value::Dict(fields) = &info.1 else { return None };
    let hash = sha1_smol::Sha1::from(&bytes[info.2.clone()]).digest().to_string();
    let mut magnet = format!("magnet:?xt=urn:btih:{hash}");
    if let Some(Value::Bytes(name)) = get(fields, b"name").map(|i| &fields[i].1) {
        magnet.push_str(&format!("&dn={}", encode(&String::from_utf8_lossy(name))));
    }
    let mut trackers: Vec<&[u8]> = Vec::new();
    if let Some(Value::Bytes(t)) = get(&top, b"announce").map(|i| &top[i].1) {
        trackers.push(t);
    }
    if let Some(Value::List(tiers)) = get(&top, b"announce-list").map(|i| &top[i].1) {
        for tier in tiers {
            if let Value::List(urls) = tier {
                trackers.extend(urls.iter().filter_map(|u| if let Value::Bytes(b) = u { Some(*b) } else { None }));
            }
        }
    }
    let mut seen: Vec<&[u8]> = Vec::new();
    for t in trackers {
        if !seen.contains(&t) {
            seen.push(t);
            magnet.push_str(&format!("&tr={}", encode(&String::from_utf8_lossy(t))));
        }
    }
    Some(magnet)
}

/// Percent-encodes everything but the unreserved characters (RFC 3986).
fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// A bencoded value. Dictionary entries keep where their value sits in the file (the info-hash
/// is the SHA-1 of the `info` value's exact bytes).
enum Value<'a> {
    Int,
    Bytes(&'a [u8]),
    List(Vec<Value<'a>>),
    Dict(Vec<(&'a [u8], Value<'a>, std::ops::Range<usize>)>),
}

/// The value starting at `at`, and where it ends. None for anything malformed (or nested
/// absurdly deep).
fn parse(b: &[u8], at: usize, depth: usize) -> Option<(Value<'_>, usize)> {
    if depth > 64 {
        return None;
    }
    match *b.get(at)? {
        b'i' => {
            let len = b.get(at + 1..)?.iter().position(|&c| c == b'e')?;
            Some((Value::Int, at + 1 + len + 1))
        }
        b'l' => {
            let (mut i, mut items) = (at + 1, Vec::new());
            while *b.get(i)? != b'e' {
                let (v, next) = parse(b, i, depth + 1)?;
                items.push(v);
                i = next;
            }
            Some((Value::List(items), i + 1))
        }
        b'd' => {
            let (mut i, mut entries) = (at + 1, Vec::new());
            while *b.get(i)? != b'e' {
                let (Value::Bytes(key), value_at) = parse(b, i, depth + 1)? else { return None };
                let (value, next) = parse(b, value_at, depth + 1)?;
                entries.push((key, value, value_at..next));
                i = next;
            }
            Some((Value::Dict(entries), i + 1))
        }
        b'0'..=b'9' => {
            let colon = at + b.get(at..)?.iter().position(|&c| c == b':')?;
            let len: usize = std::str::from_utf8(&b[at..colon]).ok()?.parse().ok()?;
            let end = colon.checked_add(1)?.checked_add(len)?;
            Some((Value::Bytes(b.get(colon + 1..end)?), end))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-file torrent named "a b+c.txt" (info-hash worked out separately).
    fn torrent() -> Vec<u8> {
        let mut info = b"d6:lengthi5e4:name9:a b+c.txt12:piece lengthi16384e6:pieces20:".to_vec();
        info.extend(0u8..20);
        info.push(b'e');
        let mut t = b"d8:announce33:udp://tracker.example:80/announce13:announce-listll33:udp://tracker.example:80/announceel22:https://t2.example/annee4:info".to_vec();
        t.extend(&info);
        t.push(b'e');
        t
    }

    #[test]
    fn links_in_text() {
        let text = "Get https://a.com/x.zip, then (http://b.org/y.iso).\r\nmagnet:?xt=urn:btih:abc&dn=x\n<https://c.net/z?q=1>\nftp://no.pe/x https://a.com/x.zip\nhttps://";
        assert_eq!(links_in(text), vec!["https://a.com/x.zip", "http://b.org/y.iso", "magnet:?xt=urn:btih:abc&dn=x", "https://c.net/z?q=1"]);
        assert!(links_in("no links here").is_empty());
        assert_eq!(links_in("HTTPS://UP.CASE/A").len(), 1, "any case");
    }

    #[test]
    fn internet_shortcut() {
        assert_eq!(url_shortcut("[InternetShortcut]\r\nIDList=\r\nURL=https://x.com/a.pdf\r\n").as_deref(), Some("https://x.com/a.pdf"));
        assert_eq!(url_shortcut("[InternetShortcut]\nURL=file:///C:/x"), None, "only web links");
        assert_eq!(url_shortcut("hello"), None);
    }

    #[test]
    fn torrent_file_to_magnet() {
        let magnet = torrent_magnet(&torrent()).expect("valid torrent");
        assert!(magnet.starts_with("magnet:?xt=urn:btih:3aad62bb80ef1fd92323eb9a9ee4af95a92a413e&dn=a%20b%2Bc.txt"), "{magnet}");
        assert!(magnet.contains("&tr=udp%3A%2F%2Ftracker.example%3A80%2Fannounce"), "{magnet}");
        assert!(magnet.contains("&tr=https%3A%2F%2Ft2.example%2Fann"), "{magnet}");
        assert_eq!(magnet.matches("&tr=").count(), 2, "each tracker once: {magnet}");
        assert!(rdm_core::is_torrent_link(&magnet));
        for bad in [&b""[..], b"d4:infoi1ee", b"not bencode", b"d4:info", &torrent()[..40]] {
            assert_eq!(torrent_magnet(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn read_by_kind() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, body: &[u8]| {
            let p = dir.path().join(name);
            std::fs::write(&p, body).unwrap();
            p
        };
        let list = write("links.TXT", b"https://a.com/1.zip\nhttps://a.com/2.zip\n");
        assert_eq!(read(&list).unwrap(), vec!["https://a.com/1.zip", "https://a.com/2.zip"]);
        let t = read(&write("x.torrent", &torrent())).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].starts_with("magnet:?xt=urn:btih:3aad62bb"));
        assert_eq!(read(&write("page.url", b"[InternetShortcut]\nURL=https://x.com/\n")).unwrap(), vec!["https://x.com/"]);
        assert!(read(&write("empty.txt", b"nothing")).unwrap_err().contains("empty.txt"));
        assert!(read(&write("broken.torrent", b"junk")).is_err());
        assert!(read(&write("setup.exe", b"MZ")).unwrap_err().contains(".torrent"), "says what can be dropped");
        assert!(read(&dir.path().join("gone.txt")).is_err());
        let many: String = (0..MAX_LINKS + 50).map(|i| format!("https://a.com/{i}\n")).collect();
        assert_eq!(read(&write("many.txt", many.as_bytes())).unwrap().len(), MAX_LINKS);
    }
}
