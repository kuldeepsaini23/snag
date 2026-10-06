use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Video,
    Music,
    Archive,
    Document,
    Program,
    Image,
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
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "avif" | "heic" | "tif" | "tiff" => Self::Image,
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
            Self::Image => "Images",
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

    #[test]
    fn image_extensions_categorised() {
        for name in ["a.jpg", "b.JPEG", "c.png", "d.gif", "e.webp", "f.bmp", "g.svg", "h.avif", "i.heic", "j.tif", "k.tiff"] {
            assert_eq!(Category::from_name(name), Category::Image, "{name}");
        }
        assert_eq!(Category::Image.folder(), "Images");
        // Older state files only know the earlier categories; they must still load.
        let old: Category = serde_json::from_str("\"Other\"").unwrap();
        assert_eq!(old, Category::Other);
        assert_eq!(serde_json::to_string(&Category::Image).unwrap(), "\"Image\"");
    }
}
