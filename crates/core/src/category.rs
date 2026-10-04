use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Video,
    Music,
    Archive,
    Document,
    Program,
    Other,
}

impl Category {
    pub fn from_name(file_name: &str) -> Self {
        let ext = file_name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v" | "flv" | "wmv" | "ts" | "3gp" => Self::Video,
            "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" | "opus" | "wma" => Self::Music,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" | "iso" => Self::Archive,
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "epub" | "csv" | "md" | "odt" => Self::Document,
            "exe" | "msi" | "apk" | "dmg" | "deb" | "rpm" | "appimage" => Self::Program,
            _ => Self::Other,
        }
    }

    pub fn folder(self) -> &'static str {
        match self {
            Self::Video => "Videos",
            Self::Music => "Music",
            Self::Archive => "Archives",
            Self::Document => "Documents",
            Self::Program => "Programs",
            Self::Other => "Other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_from_name() {
        assert_eq!(Category::from_name("a.MP4"), Category::Video);
        assert_eq!(Category::from_name("x.mp3"), Category::Music);
        assert_eq!(Category::from_name("y.zip"), Category::Archive);
        assert_eq!(Category::from_name("z.pdf"), Category::Document);
        assert_eq!(Category::from_name("setup.exe"), Category::Program);
        assert_eq!(Category::from_name("noext"), Category::Other);
        assert_eq!(Category::Video.folder(), "Videos");
        assert_eq!(Category::Other.folder(), "Other");
    }
}
