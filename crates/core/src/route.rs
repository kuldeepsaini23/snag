//! Where a link goes: torrent, image gallery, video/audio page (yt-dlp knows 1,800+ sites),
//! or a plain file for the segmented downloader.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Torrent,
    Gallery,
    /// A page yt-dlp should read (a known video site, or any web page that isn't a file).
    Media,
    File,
}

/// Extensions that mean "this link is the file itself".
const FILE_EXTS: [&str; 52] = [
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "tgz", "iso", "img", "dmg", "exe", "msi", "apk", "deb", "rpm", "appimage", "pdf", "doc", "docx", "xls", "xlsx",
    "ppt", "pptx", "txt", "csv", "epub", "mp4", "m4v", "webm", "mkv", "mov", "avi", "flv", "wmv", "mp3", "m4a", "aac", "flac", "wav", "ogg", "opus", "jpg", "jpeg",
    "png", "gif", "webp", "svg", "bin", "torrent", "json", "xml",
];

pub fn route(url: &str) -> Route {
    let url = url.trim();
    if rdm_torrent::is_torrent_link(url) {
        return Route::Torrent;
    }
    if rdm_media::gallery::is_gallery_url(url) {
        return Route::Gallery;
    }
    if rdm_media::is_media_url(url) {
        return Route::Media;
    }
    let path = url.split_once("://").map_or(url, |(_, r)| r).split(['?', '#']).next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').skip(1).filter(|s| !s.is_empty()).collect();
    let last = segments.last().copied().unwrap_or("");
    let ext = last.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if FILE_EXTS.contains(&ext.as_str()) { Route::File } else { Route::Media }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_go_where_they_belong() {
        assert_eq!(route("https://www.youtube.com/watch?v=abc"), Route::Media);
        assert_eq!(route("https://www.pornhub.com/view_video.php?viewkey=abc"), Route::Media, "any video site yt-dlp knows");
        assert_eq!(route("https://www.xvideos.com/video123/some_title"), Route::Media);
        assert_eq!(route("https://news.example.com/2026/10/clip"), Route::Media, "an unknown page: yt-dlp looks for video");
        assert_eq!(route("https://cdn.example.com/files/setup.exe"), Route::File);
        assert_eq!(route("https://cdn.example.com/movie.MP4?token=1"), Route::File, "a direct video file needs no reading");
        assert_eq!(route("https://example.com/report.pdf#page=2"), Route::File);
        assert_eq!(route("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"), Route::Torrent);
        assert_eq!(route("https://example.com/x.torrent"), Route::Torrent);
        assert_eq!(route("https://www.pinterest.com/pin/1/"), Route::Gallery);
        assert_eq!(route("https://example.com/"), Route::Media);
    }
}
