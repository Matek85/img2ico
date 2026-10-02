// Reading a source image of any supported format into memory: PNG, JPEG,
// BMP, GIF, WebP, TIFF and TGA through the `image` crate, macOS .icns
// files through the `icns` crate (see icns.rs) and SVG through resvg (see
// vector.rs). Used by the conversion and by
// --inspect, so both agree on what counts as an image.

use crate::icns::{decode_icns, is_icns};
use crate::vector::{VectorImage, is_svg};
use image::DynamicImage;
use img2ico_core::source::decoder_limits;
use std::io::Read;
use std::path::Path;

// What the web version shares: what counts as an image, the limits, and
// decoding from memory. This file adds opening a file, which can stream from
// disk instead of reading it all first.
pub use img2ico_core::source::{Artwork, DEFAULT_MAX_PIXELS, SUPPORTED_FORMATS, check_pixel_limit};

/// The pixel count of the raster image at `path`, read from its header alone,
/// or 0 if it cannot be told (not a raster image, unreadable). For deciding
/// how much memory a conversion is about to need.
pub fn peek_pixels(path: &Path) -> u64 {
    image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .map_or(0, |(width, height)| u64::from(width) * u64::from(height))
}

/// The first bytes of the file at `path` - enough to tell an SVG.
fn read_header(path: &Path) -> Result<Vec<u8>, String> {
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
    Ok(header[..filled].to_vec())
}

/// Opens the file at `path` as a source of either kind. SVG is recognized
/// from the content, like the raster formats.
pub fn open_source(path: &Path, max_pixels: u64) -> Result<Artwork, String> {
    if is_svg(&read_header(path)?) {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return VectorImage::parse(&bytes, &path.display().to_string()).map(Artwork::Vector);
    }
    open_image_limited(path, max_pixels).map(Artwork::Raster)
}

/// What --inspect needs to know about a source: the size of a raster image,
/// read from its header without decoding a single pixel - so it works for a
/// picture of any size - or the parsed drawing of an SVG.
pub enum Probe {
    Raster { width: u32, height: u32 },
    Vector(VectorImage),
}

/// Looks at the file at `path` the way `open_source` would, but only as far
/// as its size.
pub fn probe_source(path: &Path) -> Result<Probe, String> {
    let header = read_header(path)?;
    if is_svg(&header) {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return VectorImage::parse(&bytes, &path.display().to_string()).map(Probe::Vector);
    }
    if is_icns(&header) {
        // Small by nature (at most 1024 x 1024), and its size is that of
        // its largest icon, which takes decoding to find.
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let image = decode_icns(&bytes, &path.display().to_string())?;
        return Ok(Probe::Raster {
            width: image.width(),
            height: image.height(),
        });
    }
    let reader = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|e| e.to_string())?;
    // An .ico is something --inspect reads as an icon file; when that has
    // failed it is a damaged one, not a picture with a size to report.
    if reader.format() == Some(image::ImageFormat::Ico) {
        return Err("it is an .ico file, not a source image".to_string());
    }
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    Ok(Probe::Raster { width, height })
}

/// The same for a source already in memory (read from standard input).
pub fn decode_source_bytes(bytes: &[u8], max_pixels: u64) -> Result<Artwork, String> {
    img2ico_core::source::decode_source_bytes(bytes, "standard input", max_pixels)
}

/// Opens the image file at `path`. The format is recognized from the file's
/// CONTENT where the format has a recognizable header; only for a format
/// without one (TGA) the file extension decides. The error is the plain
/// reason; the caller says what it was trying to do.
fn open_image_limited(path: &Path, max_pixels: u64) -> Result<DynamicImage, String> {
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
    let reader = || {
        image::ImageReader::open(path)
            .and_then(|reader| reader.with_guessed_format())
            .map_err(|e| e.to_string())
    };
    // The size comes from the header alone, before a single pixel is decoded.
    if max_pixels != 0 {
        let (width, height) = reader()?.into_dimensions().map_err(|e| e.to_string())?;
        check_pixel_limit(width, height, max_pixels)?;
    }
    let mut reader = reader()?;
    reader.limits(decoder_limits(max_pixels));
    reader.decode().map_err(|e| e.to_string())
}

/// `open_image_limited` with the default limit.
#[cfg(test)]
pub fn open_image(path: &Path) -> Result<DynamicImage, String> {
    open_image_limited(path, DEFAULT_MAX_PIXELS)
}

/// Decoding from memory with the default limit.
#[cfg(test)]
pub fn decode_image_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    img2ico_core::source::decode_image_bytes_limited(bytes, "standard input", DEFAULT_MAX_PIXELS)
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
        let bytes = crate::icns::encode_icns(
            &|size| crate::resize::make_square_icon(&sample(), size, 0, false),
            1,
        )
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
