//! Thumbnails of downloaded pictures: decoded once, scaled down, kept in the thumbnail cache.
//! A grid of full-size photos would decode (and hold) every one at full size while scrolling.

use std::path::{Path, PathBuf};

/// The longest side of a picture's thumbnail (a grid card is about half that, at 2× density).
pub const MAX_SIDE: u32 = 480;

/// Scales `src` to fit `MAX_SIDE` and saves it as `<base>.jpg`; None if it can't be read.
pub fn make(src: &Path, base: &Path) -> Option<PathBuf> {
    let file = base.with_extension("jpg");
    let picture = image::ImageReader::open(src).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let small = picture.thumbnail(MAX_SIDE, MAX_SIDE).to_rgb8();
    std::fs::create_dir_all(file.parent()?).ok()?;
    small.save_with_format(&file, image::ImageFormat::Jpeg).ok()?;
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_becomes_a_small_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("wide.png");
        image::RgbImage::from_pixel(1000, 500, image::Rgb([255, 159, 10])).save(&src).unwrap();
        let out = make(&src, &dir.path().join("thumbs").join("abc")).expect("made");
        assert_eq!(out, dir.path().join("thumbs").join("abc.jpg"));
        let thumb = image::open(&out).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (480, 240), "fits, keeping its shape");
    }

    #[test]
    fn not_a_picture_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("fake.jpg");
        std::fs::write(&src, b"not an image").unwrap();
        assert_eq!(make(&src, &dir.path().join("x")), None);
        assert_eq!(make(&dir.path().join("missing.png"), &dir.path().join("y")), None);
    }
}
