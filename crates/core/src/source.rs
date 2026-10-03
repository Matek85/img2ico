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
        return Err(crate::msg!(
            "source.too_many_pixels",
            "the image is {width}x{height} pixels ({megapixels} megapixels), more than the limit of {limit} megapixels. Raise the limit with --max-pixels N (0 for no limit) if you trust the file.",
            width = width,
            height = height,
            megapixels = format!("{:.1}", pixels as f64 / 1e6),
            limit = format!("{:.1}", max_pixels as f64 / 1e6)
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
        let (width, height) = reader()?.into_dimensions().map_err(image_error)?;
        check_pixel_limit(width, height, max_pixels)?;
    }
    let mut reader = reader()?;
    reader.limits(decoder_limits(max_pixels));
    reader.decode().map_err(image_error)
}

/// The message for what the image library could not do. The most common case - a file that is
/// no picture at all - has a code of its own; the rest keep the library's own sentence.
fn image_error(error: image::ImageError) -> String {
    use image::error::{ImageFormatHint, UnsupportedErrorKind};
    match &error {
        image::ImageError::Unsupported(unsupported)
            if matches!(
                unsupported.kind(),
                UnsupportedErrorKind::Format(ImageFormatHint::Unknown)
            ) =>
        {
            crate::msg!(
                "source.format_unknown",
                "The image format could not be determined"
            )
        }
        _ => error.to_string(),
    }
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
        return Err(crate::msg!(
            "gif.frame_zero",
            "--gif-frame must be 1 or greater (frames are numbered starting at 1)."
        ));
    }

    let decoder = GifDecoder::new(reader).map_err(|e| {
        crate::msg!(
            "gif.unreadable",
            "Could not read '{name}' as a GIF: {e}",
            name = name,
            e = e
        )
    })?;
    let (width, height) = image::ImageDecoder::dimensions(&decoder);
    check_pixel_limit(width, height, max_pixels).map_err(|e| {
        crate::msg!(
            "source.unreadable",
            "Could not read '{name}': {e}",
            name = name,
            e = e
        )
    })?;

    // A single pass over the frames: decoding happens once per frame
    // regardless, so counting the total (for a helpful error message if
    // frame_number is out of range) costs nothing extra on top of finding
    // the requested one.
    let mut selected: Option<image::RgbaImage> = None;
    let mut total = 0usize;
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame = frame.map_err(|e| {
            crate::msg!(
                "gif.frame_undecodable",
                "Could not decode a frame in '{name}': {e}",
                name = name,
                e = e
            )
        })?;
        total += 1;
        if index + 1 == frame_number {
            selected = Some(frame.into_buffer());
        }
    }

    selected.ok_or_else(|| {
        crate::msg!(
            "gif.frame_out_of_range",
            "--gif-frame {frame_number} is out of range - '{name}' has {total} frame(s), so valid values are 1..{total}.",
            frame_number = frame_number,
            name = name,
            total = total
        )
    })
}

/// One frame of an animated GIF, as the picture it shows and for how long.
pub struct GifFrame {
    pub image: image::RgbaImage,
    /// How long the frame is shown, in milliseconds (browsers show anything under
    /// 20 as 100, and so does this).
    pub delay_ms: u32,
}

/// The most pixels all frames of one GIF may have together when they are kept in
/// memory at once (64 million, 256 MB): more than that is not worth showing.
pub const MAX_GIF_TOTAL_PIXELS: u64 = 64_000_000;

/// Every frame of a GIF that is already in memory, in order, each with the time it
/// is shown. `max_pixels` limits one frame (0 means no limit), and the frames together
/// may not pass `MAX_GIF_TOTAL_PIXELS`.
pub fn decode_gif_frames_from_bytes(
    bytes: &[u8],
    name: &str,
    max_pixels: u64,
) -> Result<Vec<GifFrame>, String> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|e| {
        crate::msg!(
            "gif.unreadable",
            "Could not read '{name}' as a GIF: {e}",
            name = name,
            e = e
        )
    })?;
    let (width, height) = image::ImageDecoder::dimensions(&decoder);
    check_pixel_limit(width, height, max_pixels).map_err(|e| {
        crate::msg!(
            "source.unreadable",
            "Could not read '{name}': {e}",
            name = name,
            e = e
        )
    })?;
    let per_frame = u64::from(width) * u64::from(height);

    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        if (frames.len() as u64 + 1) * per_frame > MAX_GIF_TOTAL_PIXELS {
            return Err(crate::msg!(
                "gif.too_many_frames",
                "'{name}' has too many frames of {width}x{height} pixels to hold them all in memory.",
                name = name,
                width = width,
                height = height
            ));
        }
        let frame = frame.map_err(|e| {
            crate::msg!(
                "gif.frame_undecodable",
                "Could not decode a frame in '{name}': {e}",
                name = name,
                e = e
            )
        })?;
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay_ms = numerator.checked_div(denominator).unwrap_or(0);
        frames.push(GifFrame {
            image: frame.into_buffer(),
            delay_ms: if delay_ms < 20 { 100 } else { delay_ms },
        });
    }
    Ok(frames)
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
    fn a_file_that_is_no_picture_has_a_code_of_its_own() {
        crate::msg::remember_messages();
        let error = decode_source_bytes(b"not an image", "a", DEFAULT_MAX_PIXELS)
            .err()
            .unwrap();
        assert_eq!(error, "The image format could not be determined");
        assert_eq!(
            crate::msg::lookup(&error).unwrap().code,
            "source.format_unknown"
        );
    }

    #[test]
    fn the_pixel_limit_stops_a_large_image_before_decoding() {
        let error = decode_image_bytes_limited(&encoded(ImageFormat::Png), "a", 50).unwrap_err();
        assert!(error.contains("12x8"), "{error}");
        assert!(decode_image_bytes_limited(&encoded(ImageFormat::Png), "a", 96).is_ok());
    }

    fn animated_gif(frames: &[([u8; 4], u16)]) -> Vec<u8> {
        use image::codecs::gif::GifEncoder;
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            for (color, delay_ms) in frames {
                let frame = image::Frame::from_parts(
                    RgbaImage::from_pixel(6, 4, Rgba(*color)),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(u32::from(*delay_ms), 1),
                );
                encoder.encode_frame(frame).unwrap();
            }
        }
        bytes.into_inner()
    }

    #[test]
    fn every_frame_of_a_gif_comes_with_its_time() {
        let bytes = animated_gif(&[([255, 0, 0, 255], 40), ([0, 0, 255, 255], 250)]);
        let frames = decode_gif_frames_from_bytes(&bytes, "a.gif", 0).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].image.dimensions(), (6, 4));
        assert_eq!(frames[0].image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(frames[1].image.get_pixel(0, 0).0, [0, 0, 255, 255]);
        // GIF times are in hundredths of a second
        assert_eq!(frames[0].delay_ms, 40);
        assert_eq!(frames[1].delay_ms, 250);
    }

    #[test]
    fn a_frame_shown_for_next_to_no_time_is_shown_for_a_tenth_of_a_second() {
        let bytes = animated_gif(&[([1, 2, 3, 255], 0)]);
        assert_eq!(
            decode_gif_frames_from_bytes(&bytes, "a.gif", 0).unwrap()[0].delay_ms,
            100
        );
    }

    #[test]
    fn frames_that_do_not_fit_in_memory_together_are_refused() {
        let bytes = animated_gif(&[([1, 2, 3, 255], 40)]);
        // a single frame over the limit of one frame
        assert!(decode_gif_frames_from_bytes(&bytes, "a.gif", 10).is_err());
        assert!(decode_gif_frames_from_bytes(b"not a gif", "a.gif", 0).is_err());
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
