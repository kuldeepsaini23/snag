#[derive(Clone, Debug, PartialEq)]
pub struct RemoteInfo {
    pub url: String,
    pub size: Option<u64>,
    pub accepts_ranges: bool,
    pub filename: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}
