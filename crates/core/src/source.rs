// Reading a source image that is already in memory, in any supported format:
// PNG, JPEG, BMP, GIF, WebP, TIFF and TGA through the `image` crate, macOS
// .icns files (see icns.rs) and SVG through resvg (see vector.rs). Opening a
// file is the command line's business (it can stream from disk); this is what
// both it and the web page share.

use crate::icns::{decode_icns, is_icns};
use crate::vector::{VectorImage, is_svg};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage};
use std::io::{BufRead, Cursor, Seek};

/// What img2ico can read as a source image, as shown in messages.
pub const SUPPORTED_FORMATS: &str = "PNG, JPG, BMP, GIF, WebP, TIFF, TGA, ICNS, SVG";

/// How many pixels a raster source may have unless told otherwise: 100
/// million, a 10000 x 10000 picture. A decoded image takes four bytes a pixel,
/// so this is roughly 400 MB - and a small file can claim far more than that
/// (a "decompression bomb"), which this stops before anything is allocated.
pub const DEFAULT_MAX_PIXELS: u64 = 100_000_000;

/// Refuses an image of `width` x `height` pixels that has more than
/// `max_pixels` (0 means no limit).
pub fn check_pixel_limit(width: u32, height: u32, max_pixels: u64) -> Result<(), String> {
    let pixels = u64::from(width) * u64::from(height);
    if max_pixels != 0 && pixels > max_pixels {
        return Err(format!(
            "the image is {width}x{height} pixels ({:.1} megapixels), more than the limit of {:.1} megapixels. Raise the limit with --max-pixels N (0 for no limit) if you trust the file.",
            pixels as f64 / 1e6,
            max_pixels as f64 / 1e6
        ));
    }
    Ok(())
}

/// The decoder limits that go with `max_pixels`: memory for a decoder is
/// capped at eight bytes a pixel (the decoded image plus working copies).
pub fn decoder_limits(max_pixels: u64) -> image::Limits {
    if max_pixels == 0 {
        return image::Limits::no_limits();
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(max_pixels.saturating_mul(8));
    limits
}

/// A source: a picture of pixels, or a drawing that can be rendered at any
/// size.
pub enum Artwork {
    Raster(DynamicImage),
    Vector(VectorImage),
}

/// Decodes a source image already in memory. `name` is how it is called in
/// messages. The format comes from the content (SVG is recognized from its
/// text too), so TGA - which has no recognizable header - is not possible
/// this way.
pub fn decode_source_bytes(bytes: &[u8], name: &str, max_pixels: u64) -> Result<Artwork, String> {
    if is_svg(bytes) {
        return VectorImage::parse(bytes, name).map(Artwork::Vector);
    }
    decode_image_bytes_limited(bytes, name, max_pixels).map(Artwork::Raster)
}

/// Decodes a raster image (or an .icns) from memory.
pub fn decode_image_bytes_limited(
    bytes: &[u8],
    name: &str,
    max_pixels: u64,
) -> Result<DynamicImage, String> {
    if is_icns(bytes) {
        return decode_icns(bytes, name).map(DynamicImage::ImageRgba8);
    }
    let reader = || {
        image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| e.to_string())
    };
    if max_pixels != 0 {
        let (width, height) = reader()?.into_dimensions().map_err(|e| e.to_string())?;
        check_pixel_limit(width, height, max_pixels)?;
    }
    let mut reader = reader()?;
    reader.limits(decoder_limits(max_pixels));
    reader.decode().map_err(|e| e.to_string())
}

/// Whether `bytes` - an image already in memory - start like a GIF. This goes
/// by the content, not by a file name.
pub fn is_gif_bytes(bytes: &[u8]) -> bool {
    image::guess_format(bytes).is_ok_and(|format| format == image::ImageFormat::Gif)
}

/// Extracts one specific frame (1-indexed - "frame 1" means the first
/// frame) from a GIF that is already in memory. Returns a clear,
/// bounds-checked error - naming the GIF's actual frame count - if
/// `frame_number` does not exist.
pub fn extract_gif_frame_from_bytes(
    bytes: &[u8],
    name: &str,
    frame_number: usize,
    max_pixels: u64,
) -> Result<image::RgbaImage, String> {
    extract_gif_frame_from(Cursor::new(bytes), name, frame_number, max_pixels)
}

/// The shared work of reading a frame from memory or from a file: `name` is
/// how the GIF is called in messages.
pub fn extract_gif_frame_from<R: BufRead + Seek>(
    reader: R,
    name: &str,
    frame_number: usize,
    max_pixels: u64,
) -> Result<image::RgbaImage, String> {
    if frame_number == 0 {
        return Err(
            "--gif-frame must be 1 or greater (frames are numbered starting at 1).".to_string(),
        );
    }

    let decoder =
        GifDecoder::new(reader).map_err(|e| format!("Could not read '{name}' as a GIF: {e}"))?;
    let (width, height) = image::ImageDecoder::dimensions(&decoder);
    check_pixel_limit(width, height, max_pixels)
        .map_err(|e| format!("Could not read '{name}': {e}"))?;

    // A single pass over the frames: decoding happens once per frame
    // regardless, so counting the total (for a helpful error message if
    // frame_number is out of range) costs nothing extra on top of finding
    // the requested one.
    let mut selected: Option<image::RgbaImage> = None;
    let mut total = 0usize;
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame = frame.map_err(|e| format!("Could not decode a frame in '{name}': {e}"))?;
        total += 1;
        if index + 1 == frame_number {
            selected = Some(frame.into_buffer());
        }
    }

    selected.ok_or_else(|| {
        format!(
            "--gif-frame {frame_number} is out of range - '{name}' has {total} frame(s), so valid values are 1..{total}."
        )
    })
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

    fn encoded(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        sample().write_to(&mut bytes, format).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn formats_are_recognized_by_content() {
        for format in [
            ImageFormat::Png,
            ImageFormat::WebP,
            ImageFormat::Tiff,
            ImageFormat::Bmp,
            ImageFormat::Gif,
        ] {
            let bytes = encoded(format);
            let artwork = decode_source_bytes(&bytes, "a", DEFAULT_MAX_PIXELS)
                .unwrap_or_else(|e| panic!("{format:?}: {e}"));
            let Artwork::Raster(image) = artwork else {
                panic!("{format:?} is not a drawing");
            };
            assert_eq!((image.width(), image.height()), (12, 8), "{format:?}");
        }
        assert!(decode_source_bytes(b"not an image", "a", DEFAULT_MAX_PIXELS).is_err());
    }

    #[test]
    fn lossless_formats_keep_the_pixels() {
        for format in [ImageFormat::Png, ImageFormat::WebP, ImageFormat::Tiff] {
            let image = decode_image_bytes_limited(&encoded(format), "a", 0).unwrap();
            assert_eq!(image.to_rgba8(), sample(), "{format:?}");
        }
    }

    #[test]
    fn a_tga_cannot_be_told_from_memory() {
        assert!(decode_image_bytes_limited(&encoded(ImageFormat::Tga), "a", 0).is_err());
    }

    #[test]
    fn an_svg_is_a_drawing() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
        assert!(matches!(
            decode_source_bytes(svg, "a.svg", DEFAULT_MAX_PIXELS),
            Ok(Artwork::Vector(_))
        ));
    }

    #[test]
    fn the_pixel_limit_stops_a_large_image_before_decoding() {
        let error = decode_image_bytes_limited(&encoded(ImageFormat::Png), "a", 50).unwrap_err();
        assert!(error.contains("12x8"), "{error}");
        assert!(decode_image_bytes_limited(&encoded(ImageFormat::Png), "a", 96).is_ok());
    }

    #[test]
    fn gif_frames_are_numbered_from_one() {
        let bytes = encoded(ImageFormat::Gif);
        assert!(is_gif_bytes(&bytes));
        assert!(!is_gif_bytes(&encoded(ImageFormat::Png)));
        assert!(extract_gif_frame_from_bytes(&bytes, "a", 1, 0).is_ok());
        assert!(
            extract_gif_frame_from_bytes(&bytes, "a", 0, 0)
                .unwrap_err()
                .contains("1 or greater")
        );
        assert!(
            extract_gif_frame_from_bytes(&bytes, "a", 2, 0)
                .unwrap_err()
                .contains("has 1 frame")
        );
    }
}
