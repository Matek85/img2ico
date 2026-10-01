// Reading a source image of any supported format into memory: PNG, JPEG,
// BMP, GIF, WebP, TIFF and TGA through the `image` crate, macOS .icns
// files through the `icns` crate (see icns.rs) and SVG through resvg (see
// vector.rs). Used by the conversion and by
// --inspect, so both agree on what counts as an image.

use crate::icns::{decode_icns, is_icns};
use crate::vector::{VectorImage, is_svg};
use image::DynamicImage;
use std::io::Read;
use std::path::Path;

/// What img2ico can read as a source image, as shown in messages.
pub const SUPPORTED_FORMATS: &str = "PNG, JPG, BMP, GIF, WebP, TIFF, TGA, ICNS, SVG";

/// A source: a picture of pixels, or a drawing that can be rendered at any
/// size.
pub enum Artwork {
    Raster(DynamicImage),
    Vector(VectorImage),
}

/// Opens the file at `path` as a source of either kind. SVG is recognized
/// from the content, like the raster formats.
pub fn open_source(path: &Path) -> Result<Artwork, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 4096];
    let mut filled = 0;
    while filled < header.len() {
        match file.read(&mut header[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    drop(file);
    if is_svg(&header[..filled]) {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return VectorImage::parse(&bytes, &path.display().to_string()).map(Artwork::Vector);
    }
    open_image(path).map(Artwork::Raster)
}

/// The same for a source already in memory (read from standard input).
pub fn decode_source_bytes(bytes: &[u8]) -> Result<Artwork, String> {
    if is_svg(bytes) {
        return VectorImage::parse(bytes, "standard input").map(Artwork::Vector);
    }
    decode_image_bytes(bytes).map(Artwork::Raster)
}

/// Opens the image file at `path`. The format is recognized from the file's
/// CONTENT where the format has a recognizable header; only for a format
/// without one (TGA) the file extension decides. The error is the plain
/// reason; the caller says what it was trying to do.
pub fn open_image(path: &Path) -> Result<DynamicImage, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 4];
    let read = file.read(&mut header).map_err(|e| e.to_string())?;
    if is_icns(&header[..read]) {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return decode_icns(&bytes, &path.display().to_string()).map(DynamicImage::ImageRgba8);
    }
    drop(file);

    // ImageReader starts from the format the extension suggests and lets the
    // content override it when the content is recognizable.
    image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())
}

/// Decodes an image already in memory (read from standard input). The format
/// can only come from the content here, so TGA - which has no recognizable
/// header - is not possible this way.
pub fn decode_image_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    if is_icns(bytes) {
        return decode_icns(bytes, "standard input").map(DynamicImage::ImageRgba8);
    }
    image::load_from_memory(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};

    fn sample() -> RgbaImage {
        RgbaImage::from_fn(12, 8, |x, y| {
            Rgba([(x * 20) as u8, (y * 30) as u8, 77, 255])
        })
    }

    fn save(dir: &Path, name: &str, format: ImageFormat) -> std::path::PathBuf {
        let path = dir.join(name);
        sample().save_with_format(&path, format).unwrap();
        path
    }

    #[test]
    fn every_supported_raster_format_round_trips_through_open_image() {
        let dir = tempfile::tempdir().unwrap();
        for (name, format) in [
            ("a.png", ImageFormat::Png),
            ("a.bmp", ImageFormat::Bmp),
            ("a.gif", ImageFormat::Gif),
            ("a.webp", ImageFormat::WebP),
            ("a.tiff", ImageFormat::Tiff),
            ("a.tga", ImageFormat::Tga),
        ] {
            let path = save(dir.path(), name, format);
            let image = open_image(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!((image.width(), image.height()), (12, 8), "{name}");
        }
    }

    #[test]
    fn lossless_formats_keep_the_pixels_exactly() {
        let dir = tempfile::tempdir().unwrap();
        for (name, format) in [
            ("a.png", ImageFormat::Png),
            ("a.webp", ImageFormat::WebP),
            ("a.tiff", ImageFormat::Tiff),
            ("a.tga", ImageFormat::Tga),
        ] {
            let path = save(dir.path(), name, format);
            assert_eq!(open_image(&path).unwrap().to_rgba8(), sample(), "{name}");
        }
    }

    #[test]
    fn the_content_wins_over_a_wrong_or_missing_extension() {
        let dir = tempfile::tempdir().unwrap();
        let png = save(dir.path(), "a.png", ImageFormat::Png);
        let webp = save(dir.path(), "a.webp", ImageFormat::WebP);
        for (source, copy) in [
            (&png, "wrong.webp"),
            (&webp, "wrong.png"),
            (&png, "noextension"),
        ] {
            let target = dir.path().join(copy);
            std::fs::copy(source, &target).unwrap();
            assert!(open_image(&target).is_ok(), "{copy}");
        }
    }

    #[test]
    fn a_tga_is_recognized_by_its_extension_only() {
        let dir = tempfile::tempdir().unwrap();
        let tga = save(dir.path(), "a.tga", ImageFormat::Tga);
        let bytes = std::fs::read(&tga).unwrap();
        assert!(open_image(&tga).is_ok());
        assert!(
            decode_image_bytes(&bytes).is_err(),
            "from memory there is no file name to go by"
        );
        let renamed = dir.path().join("a.dat");
        std::fs::write(&renamed, bytes).unwrap();
        assert!(open_image(&renamed).is_err());
    }

    #[test]
    fn decode_image_bytes_recognizes_formats_by_content() {
        let dir = tempfile::tempdir().unwrap();
        for (name, format) in [
            ("a.png", ImageFormat::Png),
            ("a.webp", ImageFormat::WebP),
            ("a.tiff", ImageFormat::Tiff),
            ("a.bmp", ImageFormat::Bmp),
        ] {
            let bytes = std::fs::read(save(dir.path(), name, format)).unwrap();
            assert!(decode_image_bytes(&bytes).is_ok(), "{name}");
        }
        assert!(decode_image_bytes(b"not an image").is_err());
    }

    #[test]
    fn an_icns_file_opens_as_its_largest_image() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = crate::icns::encode_icns(&|size| {
            crate::resize::make_square_icon(&sample(), size, 0, false)
        })
        .unwrap();
        let path = dir.path().join("a.icns");
        std::fs::write(&path, &bytes).unwrap();
        let image = open_image(&path).unwrap();
        assert_eq!(image.width(), 1024, "the largest icon in the file");
        assert!(decode_image_bytes(&bytes).is_ok());
    }

    #[test]
    fn unreadable_files_give_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open_image(&dir.path().join("missing.png")).is_err());
        let junk = dir.path().join("junk.png");
        std::fs::write(&junk, b"nope").unwrap();
        assert!(open_image(&junk).is_err());
        let empty = dir.path().join("empty.webp");
        std::fs::write(&empty, b"").unwrap();
        assert!(open_image(&empty).is_err());
    }
}
