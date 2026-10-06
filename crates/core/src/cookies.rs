//! Browser cookies handed over by the extension, so logged-in downloads work.
//! Kept in memory only: they are never written to `state.json`.

use serde::Deserialize;

/// One cookie as `chrome.cookies.getAll` reports it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// `example.com` / `.example.com` (domain cookie) or the exact host (host-only).
    pub domain: String,
    #[serde(default)]
    pub host_only: bool,
    #[serde(default = "root")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    /// Unix seconds; `None` for a session cookie.
    #[serde(default)]
    pub expiration_date: Option<f64>,
    pub name: String,
    pub value: String,
}

fn root() -> String {
    "/".into()
}

#[derive(Debug, Default)]
pub struct Jar {
    cookies: Vec<Cookie>,
}

/// (https?, host, path) of an http(s) URL.
/// Parsed exactly the way the HTTP client will parse it, so the host we match cookies
/// against is the host the request really goes to (e.g. `\` counts as `/`).
fn parts(url: &str) -> Option<(bool, String, String)> {
    let url = rdm_engine::Url::parse(url).ok()?;
    let https = match url.scheme() {
        "https" => true,
        "http" => false,
        _ => return None,
    };
    Some((https, url.host_str()?.to_ascii_lowercase(), url.path().to_string()))
}

impl Cookie {
    /// Lower-case domain without the leading dot.
    fn bare_domain(&self) -> String {
        self.domain.trim_start_matches('.').to_ascii_lowercase()
    }

    fn matches_host(&self, host: &str) -> bool {
        let domain = self.bare_domain();
        if self.host_only {
            host == domain
        } else {
            host == domain || host.strip_suffix(&domain).is_some_and(|sub| sub.ends_with('.'))
        }
    }

    fn matches_path(&self, path: &str) -> bool {
        let cp = self.path.as_str();
        path == cp || (path.starts_with(cp) && (cp.ends_with('/') || path[cp.len()..].starts_with('/')))
    }

    fn expired(&self, now: f64) -> bool {
        self.expiration_date.is_some_and(|t| t <= now)
    }

    fn key(&self) -> (String, &str, &str) {
        (self.bare_domain(), &self.path, &self.name)
    }
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

impl Jar {
    /// Adds cookies; one with the same domain, path and name replaces the old one.
    pub fn add(&mut self, cookies: Vec<Cookie>) {
        for cookie in cookies {
            match self.cookies.iter_mut().find(|c| c.key() == cookie.key()) {
                Some(slot) => *slot = cookie,
                None => self.cookies.push(cookie),
            }
        }
        let now = now();
        self.cookies.retain(|c| !c.expired(now));
    }

    /// Drops every cookie of `url`'s site (they stopped working, e.g. rotated by the browser).
    pub fn forget(&mut self, url: &str) {
        if let Some((_, host, _)) = parts(url) {
            self.cookies.retain(|c| !c.matches_host(&host));
        }
    }

    /// Cookies a browser would send to `url`.
    pub fn for_url(&self, url: &str) -> Vec<&Cookie> {
        let Some((https, host, path)) = parts(url) else { return Vec::new() };
        let now = now();
        self.cookies
            .iter()
            .filter(|c| c.matches_host(&host) && c.matches_path(&path) && (https || !c.secure) && !c.expired(now))
            .collect()
    }

    /// `Cookie:` header value for `url`, if any cookie applies.
    pub fn header(&self, url: &str) -> Option<String> {
        let pairs: Vec<String> = self.for_url(url).iter().map(|c| format!("{}={}", c.name, c.value)).collect();
        (!pairs.is_empty()).then(|| pairs.join("; "))
    }

    /// A Netscape `cookies.txt` for yt-dlp with every cookie of `url`'s site, or `None` if there are none.
    pub fn netscape(&self, url: &str) -> Option<String> {
        let (_, host, _) = parts(url)?;
        let now = now();
        let unsafe_text = |s: &str| s.contains(['\t', '\n', '\r']);
        let lines: Vec<String> = self
            .cookies
            .iter()
            .filter(|c| c.matches_host(&host) && !c.expired(now) && !unsafe_text(&c.name) && !unsafe_text(&c.value) && !unsafe_text(&c.path))
            .map(|c| {
                let domain = if c.host_only { c.bare_domain() } else { format!(".{}", c.bare_domain()) };
                let flag = |b: bool| if b { "TRUE" } else { "FALSE" };
                let expires = c.expiration_date.map_or(0, |t| t as u64);
                format!("{domain}\t{}\t{}\t{}\t{expires}\t{}\t{}", flag(!c.host_only), c.path, flag(c.secure), c.name, c.value)
            })
            .collect();
        (!lines.is_empty()).then(|| format!("# Netscape HTTP Cookie File\n{}\n", lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie(domain: &str, host_only: bool, path: &str, secure: bool, name: &str, value: &str) -> Cookie {
        Cookie { domain: domain.into(), host_only, path: path.into(), secure, expiration_date: None, name: name.into(), value: value.into() }
    }

    fn jar(cookies: Vec<Cookie>) -> Jar {
        let mut j = Jar::default();
        j.add(cookies);
        j
    }

    fn names(j: &Jar, url: &str) -> Vec<String> {
        j.for_url(url).iter().map(|c| c.name.clone()).collect()
    }

    #[test]
    fn domain_cookie_matches_subdomains() {
        let j = jar(vec![cookie(".youtube.com", false, "/", true, "SID", "1")]);
        assert_eq!(names(&j, "https://www.youtube.com/watch?v=a"), ["SID"]);
        assert_eq!(names(&j, "https://youtube.com/"), ["SID"]);
        assert_eq!(names(&j, "https://m.YouTube.com/x"), ["SID"], "hosts compare case-insensitively");
    }

    #[test]
    fn host_only_cookie_matches_exact_host() {
        let j = jar(vec![cookie("www.site.test", true, "/", false, "a", "1")]);
        assert_eq!(names(&j, "http://www.site.test/f.zip"), ["a"]);
        assert!(names(&j, "http://cdn.www.site.test/f.zip").is_empty());
        assert!(names(&j, "http://site.test/f.zip").is_empty());
    }

    #[test]
    fn unrelated_host_gets_nothing() {
        let j = jar(vec![cookie(".youtube.com", false, "/", false, "SID", "1")]);
        assert!(names(&j, "https://notyoutube.com/").is_empty());
        assert!(names(&j, "https://youtube.com.evil.test/").is_empty());
        assert!(names(&j, "https://user:pw@evil.test:8080/?youtube.com").is_empty());
        assert!(j.header("https://evil.test/").is_none());
        assert!(j.netscape("https://evil.test/").is_none());
        assert!(names(&j, "not a url").is_empty());
        // Browsers and reqwest treat `\` like `/`: this request goes to evil.test, not youtube.com.
        assert!(names(&j, "https://evil.test\\@www.youtube.com/f.zip").is_empty());
        assert!(j.header("https://evil.test\\@www.youtube.com/f.zip").is_none());
        assert!(j.netscape("https://evil.test\\@www.youtube.com/f.zip").is_none());
    }

    #[test]
    fn secure_cookie_needs_https() {
        let j = jar(vec![cookie("site.test", false, "/", true, "s", "1"), cookie("site.test", false, "/", false, "p", "2")]);
        assert_eq!(names(&j, "http://site.test/"), ["p"]);
        assert_eq!(names(&j, "https://site.test/"), ["s", "p"]);
    }

    #[test]
    fn path_prefix_rule() {
        let j = jar(vec![cookie("site.test", false, "/docs", false, "d", "1")]);
        assert_eq!(names(&j, "http://site.test/docs"), ["d"]);
        assert_eq!(names(&j, "http://site.test/docs/a.pdf"), ["d"]);
        assert!(names(&j, "http://site.test/docsx/a.pdf").is_empty());
        assert!(names(&j, "http://site.test/").is_empty());
    }

    #[test]
    fn expired_cookie_skipped() {
        let mut old = cookie("site.test", false, "/", false, "old", "1");
        old.expiration_date = Some(1_000_000.0);
        let mut fresh = cookie("site.test", false, "/", false, "fresh", "2");
        fresh.expiration_date = Some(4_000_000_000.0);
        let j = jar(vec![old, fresh]);
        assert_eq!(names(&j, "http://site.test/"), ["fresh"]);
    }

    #[test]
    fn same_cookie_replaced() {
        let mut j = jar(vec![cookie(".site.test", false, "/", false, "a", "1"), cookie(".site.test", false, "/", false, "b", "2")]);
        j.add(vec![cookie(".site.test", false, "/", false, "a", "9")]);
        assert_eq!(j.header("http://site.test/"), Some("a=9; b=2".to_string()));
    }

    #[test]
    fn netscape_format_lines() {
        let mut long = cookie(".youtube.com", false, "/", true, "SID", "abc");
        long.expiration_date = Some(4_000_000_000.5);
        let j = jar(vec![long, cookie("www.youtube.com", true, "/feed", false, "PREF", "x=1"), cookie("other.test", false, "/", false, "o", "1")]);
        let text = j.netscape("https://www.youtube.com/watch?v=a").unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "# Netscape HTTP Cookie File");
        assert!(lines.contains(&".youtube.com\tTRUE\t/\tTRUE\t4000000000\tSID\tabc"), "{text}");
        // Every path of the site goes in: yt-dlp applies the path rules itself.
        assert!(lines.contains(&"www.youtube.com\tFALSE\t/feed\tFALSE\t0\tPREF\tx=1"), "{text}");
        assert!(!text.contains("other.test"));
    }
}
