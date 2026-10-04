//! A picture as an ordinary image file: PNG, JPEG, WebP, BMP or TIFF. The browser page offers these next to the
//! icon files, for the picture as it was edited (in its own size) and for the images of an icon file (one file
//! per size).
//!
//! JPEG and BMP have no transparency (a BMP with it is shown wrongly by many programs): transparent areas are
//! laid on a background color first. PNG, WebP (lossless) and TIFF keep it.

use crate::convert::flatten_onto;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use std::io::Cursor;

/// The quality of a JPEG, 1 to 100: high enough that the compression is hard to see, and the file still small.
pub const JPEG_QUALITY: u8 = 92;

/// The largest edge a JPEG can have.
const JPEG_MAX_EDGE: u32 = 65_535;

/// The kinds of image file that can be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    WebP,
    Bmp,
    Tiff,
}

impl ImageKind {
    /// The kind a name stands for (`png`, `jpg` or `jpeg`, `webp`, `bmp`, `tiff` or `tif`), whatever its case.
    pub fn parse(text: &str) -> Option<ImageKind> {
        match text.to_ascii_lowercase().as_str() {
            "png" => Some(ImageKind::Png),
            "jpg" | "jpeg" => Some(ImageKind::Jpeg),
            "webp" => Some(ImageKind::WebP),
            "bmp" => Some(ImageKind::Bmp),
            "tif" | "tiff" => Some(ImageKind::Tiff),
            _ => None,
        }
    }

    /// The ending of the file name, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
            ImageKind::WebP => "webp",
            ImageKind::Bmp => "bmp",
            ImageKind::Tiff => "tiff",
        }
    }

    /// Whether the files of this kind can be transparent.
    pub fn has_alpha(self) -> bool {
        !matches!(self, ImageKind::Jpeg | ImageKind::Bmp)
    }
}

/// The bytes of `image` as a file of this kind. A kind without transparency gets the picture laid on `background`.
pub fn encode_image(
    image: &RgbaImage,
    kind: ImageKind,
    background: [u8; 3],
) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    let failed = |error: &dyn std::fmt::Display| {
        crate::msg!(
            "export.encode_failed",
            "Could not make the {kind} image: {error}",
            kind = kind.extension().to_ascii_uppercase(),
            error = error
        )
    };
    if kind == ImageKind::Jpeg && width.max(height) > JPEG_MAX_EDGE {
        return Err(crate::msg!(
            "export.jpeg_too_big",
            "A JPG can be at most {max} pixels along an edge; this picture is {width}x{height}. Choose another file type, or the sizes of the icon.",
            max = JPEG_MAX_EDGE,
            width = width,
            height = height
        ));
    }
    let mut out = Cursor::new(Vec::new());
    if kind.has_alpha() {
        let raw = image.as_raw();
        match kind {
            ImageKind::Png => image::codecs::png::PngEncoder::new(&mut out).write_image(
                raw,
                width,
                height,
                ExtendedColorType::Rgba8,
            ),
            ImageKind::WebP => image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .write_image(raw, width, height, ExtendedColorType::Rgba8),
            _ => image::codecs::tiff::TiffEncoder::new(&mut out).write_image(
                raw,
                width,
                height,
                ExtendedColorType::Rgba8,
            ),
        }
        .map_err(|e| failed(&e))?;
    } else {
        // No transparency: the picture is laid on the background, and only its three color channels are written.
        let mut flat = image.clone();
        flatten_onto(&mut flat, background);
        let rgb: Vec<u8> = flat
            .pixels()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect();
        match kind {
            ImageKind::Jpeg => {
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
                    .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
            }
            _ => image::codecs::bmp::BmpEncoder::new(&mut out).write_image(
                &rgb,
                width,
                height,
                ExtendedColorType::Rgb8,
            ),
        }
        .map_err(|e| failed(&e))?;
    }
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba};

    fn picture() -> RgbaImage {
        // Left half opaque red, right half fully transparent.
        let mut img = RgbaImage::new(8, 4);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = if x < 4 {
                Rgba([200, 0, 0, 255])
            } else {
                Rgba([0, 0, 0, 0])
            };
        }
        img
    }

    fn decode(bytes: &[u8], format: ImageFormat) -> RgbaImage {
        image::load_from_memory_with_format(bytes, format)
            .unwrap()
            .to_rgba8()
    }

    #[test]
    fn the_names_of_the_kinds_are_understood_in_any_case() {
        assert_eq!(ImageKind::parse("PNG"), Some(ImageKind::Png));
        assert_eq!(ImageKind::parse("jpg"), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::parse("JPEG"), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::parse("tif"), Some(ImageKind::Tiff));
        assert_eq!(ImageKind::parse("gif"), None);
        assert_eq!(ImageKind::Jpeg.extension(), "jpg");
        assert_eq!(ImageKind::Tiff.extension(), "tiff");
    }

    #[test]
    fn png_webp_and_tiff_keep_the_transparency_and_the_pixels() {
        let original = picture();
        for (kind, format) in [
            (ImageKind::Png, ImageFormat::Png),
            (ImageKind::WebP, ImageFormat::WebP),
            (ImageKind::Tiff, ImageFormat::Tiff),
        ] {
            let bytes = encode_image(&original, kind, [255, 255, 255]).unwrap();
            let back = decode(&bytes, format);
            assert_eq!(back.dimensions(), (8, 4), "{kind:?}");
            assert_eq!(back.get_pixel(1, 1).0, [200, 0, 0, 255], "{kind:?}");
            assert_eq!(
                back.get_pixel(6, 1).0[3],
                0,
                "{kind:?} keeps the transparency"
            );
        }
    }

    #[test]
    fn jpeg_and_bmp_lay_the_picture_on_the_background() {
        let original = picture();
        let blue = [10, 20, 240];
        let bmp = decode(
            &encode_image(&original, ImageKind::Bmp, blue).unwrap(),
            ImageFormat::Bmp,
        );
        assert_eq!(bmp.get_pixel(1, 1).0, [200, 0, 0, 255]);
        assert_eq!(bmp.get_pixel(6, 1).0, [10, 20, 240, 255]);

        let jpeg = decode(
            &encode_image(&original, ImageKind::Jpeg, blue).unwrap(),
            ImageFormat::Jpeg,
        );
        // Lossy: close to the colors, not equal.
        let near = |a: u8, b: u8| a.abs_diff(b) < 40;
        let (red, back) = (jpeg.get_pixel(1, 1).0, jpeg.get_pixel(6, 1).0);
        assert!(near(red[0], 200) && near(red[1], 0) && near(red[2], 0));
        assert!(near(back[0], 10) && near(back[1], 20) && near(back[2], 240));
    }

    #[test]
    fn a_jpeg_too_big_for_the_format_is_refused_with_a_message() {
        let wide = RgbaImage::new(JPEG_MAX_EDGE + 1, 1);
        let error = encode_image(&wide, ImageKind::Jpeg, [255, 255, 255]).unwrap_err();
        assert!(error.contains("65535"), "{error}");
        // The other kinds have no such limit.
        assert!(encode_image(&RgbaImage::new(70_000, 1), ImageKind::Png, [0, 0, 0]).is_ok());
    }
}
