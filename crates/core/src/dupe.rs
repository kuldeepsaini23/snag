//! "Already downloaded?": links compared in a normal form, so the same download reached through
//! a share link, a tracking parameter or a different spelling is still recognised.

/// Query parameters that only track where a click came from; they never change the download.
const TRACKING: [&str; 14] = ["fbclid", "gclid", "igshid", "si", "feature", "ref", "ref_src", "_ga", "mc_cid", "mc_eid", "pp", "ab_channel", "spm", "share"];

/// One spelling per download: scheme and `www.`/`m.` dropped, host lower-cased, YouTube share
/// and Shorts links turned into watch links, tracking parameters, fragments and trailing
/// slashes removed, the rest of the query sorted. Magnet links compare by their info hash.
pub fn normalize(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("magnet:") {
        let hash = url.split(['?', '&']).find_map(|kv| kv.strip_prefix("xt=urn:btih:")).unwrap_or(url);
        return format!("magnet:{}", hash.to_ascii_lowercase());
    }
    let Ok(parsed) = url::Url::parse(url) else { return url.to_string() };
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host).to_string();
    let path = parsed.path().trim_end_matches('/');
    let mut query: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && !TRACKING.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    // youtu.be/<id> and /shorts/<id> are the same video as watch?v=<id>.
    let youtube_id = match host.as_str() {
        "youtu.be" => Some(path.trim_start_matches('/').to_string()),
        "youtube.com" => path.strip_prefix("/shorts/").map(str::to_string),
        _ => None,
    };
    let (host, path) = match youtube_id {
        Some(id) if !id.is_empty() => {
            query.retain(|(k, _)| k != "v");
            query.push(("v".into(), id));
            ("youtube.com".to_string(), "/watch".to_string())
        }
        _ => (host, path.to_string()),
    };
    query.sort();
    let query: Vec<String> = query.iter().map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}={v}") }).collect();
    if query.is_empty() { format!("{host}{path}") } else { format!("{host}{path}?{}", query.join("&")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_download_is_recognised_however_it_is_written() {
        let same = |a: &str, b: &str| assert_eq!(normalize(a), normalize(b), "{a} vs {b}");
        same("https://www.youtube.com/watch?v=abc", "https://youtu.be/abc?si=XyZ");
        same("https://www.youtube.com/watch?v=abc&feature=share", "https://m.youtube.com/watch?v=abc");
        same("https://youtube.com/shorts/abc", "https://www.youtube.com/watch?v=abc");
        same("https://example.com/file.zip?utm_source=x&utm_medium=y", "https://EXAMPLE.com/file.zip");
        same("https://example.com/file.zip#section", "https://example.com/file.zip");
        same("https://example.com/a/?b=2&a=1", "https://example.com/a?a=1&b=2");
        same("http://example.com/file.zip", "https://example.com/file.zip");
        same(
            "magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01&dn=Movie&tr=udp://x",
            "magnet:?dn=Other&xt=urn:btih:abcdef0123456789abcdef0123456789abcdef01",
        );
    }

    #[test]
    fn different_downloads_stay_different() {
        let differ = |a: &str, b: &str| assert_ne!(normalize(a), normalize(b), "{a} vs {b}");
        differ("https://www.youtube.com/watch?v=abc", "https://www.youtube.com/watch?v=abd");
        differ("https://example.com/file.zip?version=2", "https://example.com/file.zip?version=3");
        differ("https://example.com/a.zip", "https://example.org/a.zip");
        differ("https://example.com/A.zip", "https://example.com/a.zip");
    }
}
