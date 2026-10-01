// How a source image gets onto the square icon canvas, beyond plain scaling:
// fitting it in or filling the canvas (--fit), cutting out a part (--crop),
// removing empty margins (--trim), rounding the corners (--corner-radius)
// and choosing the sizes the source can really support (--sizes auto).
//
// The scaling itself stays in resize.rs; this module decides WHAT is scaled
// and what is done to the result.

use crate::resize::{AlphaMode, make_square_icon, resize_rgba};
use image::RgbaImage;

/// The highest --corner-radius: 50% of the shorter edge makes a circle (or a
/// pill for a non-square picture).
pub const MAX_CORNER_RADIUS: u8 = 50;

/// How the picture meets the square canvas, selectable via --fit.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    /// The whole picture fits inside the square; a picture that is not
    /// square leaves transparent bars. The default.
    #[default]
    Contain,
    /// The picture fills the whole square; what does not fit is cut off at
    /// the edges.
    Cover,
}

/// What the canvas looks like around and on the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Layout {
    pub fit: FitMode,
    /// Transparent margin in percent of the icon size (0-100).
    pub padding: u8,
    /// Corner rounding in percent of the picture's shorter edge (0-50).
    pub corner_radius: u8,
}

/// The size of the square the picture is placed in: the canvas minus the
/// padding on both sides, at least one pixel.
pub fn content_edge(size: u32, padding_percent: u8) -> f32 {
    let padding = f32::from(padding_percent.min(100));
    (size as f32 * (1.0 - padding / 100.0)).max(1.0)
}

/// Builds the `size` x `size` icon from `rgba` according to `layout`.
/// `alpha` says what the alpha channel of `rgba` needs while it is scaled
/// (a plain `has_transparency()` bool works too), settled once by the caller.
pub fn make_icon(
    rgba: &RgbaImage,
    size: u32,
    layout: &Layout,
    alpha: impl Into<AlphaMode>,
) -> RgbaImage {
    let alpha = alpha.into();
    if layout.fit == FitMode::Contain && layout.corner_radius == 0 {
        return make_square_icon(rgba, size, layout.padding, alpha);
    }

    let (orig_w, orig_h) = rgba.dimensions();
    let content = content_edge(size, layout.padding);
    let placed = match layout.fit {
        FitMode::Contain => {
            let scale = (content / orig_w as f32).min(content / orig_h as f32);
            let w = ((orig_w as f32 * scale).round() as u32).max(1);
            let h = ((orig_h as f32 * scale).round() as u32).max(1);
            resize_rgba(rgba, w, h, alpha)
        }
        FitMode::Cover => {
            // Scale until the shorter edge fills the square, then cut the
            // middle square out of the longer edge.
            let edge = (content.round() as u32).max(1);
            let scale = (edge as f32 / orig_w as f32).max(edge as f32 / orig_h as f32);
            let w = ((orig_w as f32 * scale).round() as u32).max(edge);
            let h = ((orig_h as f32 * scale).round() as u32).max(edge);
            let resized = resize_rgba(rgba, w, h, alpha);
            image::imageops::crop_imm(&resized, (w - edge) / 2, (h - edge) / 2, edge, edge)
                .to_image()
        }
    };

    let mut placed = placed;
    let (w, h) = placed.dimensions();
    round_corners(&mut placed, 0, 0, w, h, layout.corner_radius);

    let mut canvas = RgbaImage::new(size, size);
    image::imageops::overlay(
        &mut canvas,
        &placed,
        i64::from((size.saturating_sub(w)) / 2),
        i64::from((size.saturating_sub(h)) / 2),
    );
    canvas
}

/// Rounds the corners of the rectangle at (`x`, `y`) of size `width` x
/// `height` inside `img` by making what lies outside the rounded shape
/// transparent. The radius is `percent` of the rectangle's shorter edge
/// (at most 50, which makes a circle for a square). The edge is smooth: a
/// pixel the curve cuts through keeps the matching share of its opacity.
pub fn round_corners(img: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, percent: u8) {
    if percent == 0 || width == 0 || height == 0 {
        return;
    }
    let (w, h) = (width as f32, height as f32);
    let radius = (w.min(h) * f32::from(percent.min(MAX_CORNER_RADIUS)) / 100.0).max(0.0);
    let (half_w, half_h) = (w / 2.0, h / 2.0);

    for py in 0..height {
        for px in 0..width {
            // Distance of the pixel center from the rounded rectangle's
            // outline: negative inside, positive outside.
            let dx = (px as f32 + 0.5 - half_w).abs() - (half_w - radius);
            let dy = (py as f32 + 0.5 - half_h).abs() - (half_h - radius);
            let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
            let distance = outside + dx.max(dy).min(0.0) - radius;
            let coverage = (0.5 - distance).clamp(0.0, 1.0);
            if coverage < 1.0 {
                let pixel = img.get_pixel_mut(x + px, y + py);
                pixel.0[3] = (f32::from(pixel.0[3]) * coverage).round() as u8;
            }
        }
    }
}

/// A rectangle of the source image, from --crop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Parses a --crop value: "x,y,width,height" in pixels of the source image,
/// counted from its top left corner.
pub fn parse_crop(text: &str) -> Result<CropRect, String> {
    let wrong = || {
        format!(
            "Invalid --crop value '{text}': expected x,y,width,height in pixels, e.g. 10,10,200,200."
        )
    };
    let numbers: Vec<u32> = text
        .split(',')
        .map(|part| part.trim().parse::<u32>().map_err(|_| wrong()))
        .collect::<Result<_, _>>()?;
    let [x, y, width, height] = numbers[..] else {
        return Err(wrong());
    };
    if width == 0 || height == 0 {
        return Err(format!(
            "Invalid --crop value '{text}': the width and the height must be at least 1."
        ));
    }
    Ok(CropRect {
        x,
        y,
        width,
        height,
    })
}

/// Cuts `rect` out of `img`. It has to lie completely inside the image.
pub fn crop(img: &RgbaImage, rect: CropRect) -> Result<RgbaImage, String> {
    let (w, h) = img.dimensions();
    let fits = u64::from(rect.x) + u64::from(rect.width) <= u64::from(w)
        && u64::from(rect.y) + u64::from(rect.height) <= u64::from(h);
    if !fits {
        return Err(format!(
            "--crop {},{},{},{} reaches outside the image, which is {w}x{h} pixels.",
            rect.x, rect.y, rect.width, rect.height
        ));
    }
    Ok(image::imageops::crop_imm(img, rect.x, rect.y, rect.width, rect.height).to_image())
}

/// The bounding box of everything that is not fully transparent, as a
/// rectangle - `None` if the whole image is transparent.
pub fn content_bounds(img: &RgbaImage) -> Option<CropRect> {
    let (w, h) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0, 0);
    let mut any = false;
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel.0[3] > 0 {
            any = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    any.then(|| CropRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

/// What --trim did to an image.
#[derive(Debug, PartialEq, Eq)]
pub enum Trimmed {
    /// The image, cut down to its content.
    Cut(RgbaImage),
    /// There was no transparent margin: the image stays as it is.
    NothingToCut,
    /// The whole image is transparent, there is no content to keep.
    Empty,
}

/// Removes the fully transparent margin around the content of `img`.
pub fn trim_transparent(img: &RgbaImage) -> Trimmed {
    match content_bounds(img) {
        None => Trimmed::Empty,
        Some(bounds) if bounds.width == img.width() && bounds.height == img.height() => {
            Trimmed::NothingToCut
        }
        Some(bounds) => match crop(img, bounds) {
            Ok(cut) => Trimmed::Cut(cut),
            Err(_) => Trimmed::NothingToCut,
        },
    }
}

/// For --sizes auto: the sizes of `candidates` the source can provide
/// without upscaling - those up to its longer edge. A source smaller than all
/// of them gets one icon at its own size, so there is always something to
/// write.
pub fn auto_sizes(candidates: &[u32], longer_edge: u32) -> Vec<u32> {
    let fitting: Vec<u32> = candidates
        .iter()
        .copied()
        .filter(|&size| size <= longer_edge)
        .collect();
    if fitting.is_empty() {
        vec![longer_edge.max(1)]
    } else {
        fitting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
    const GREEN: Rgba<u8> = Rgba([0, 255, 0, 255]);

    fn layout(fit: FitMode, padding: u8, corner_radius: u8) -> Layout {
        Layout {
            fit,
            padding,
            corner_radius,
        }
    }

    /// 40x20: red left half, green right half.
    fn wide() -> RgbaImage {
        RgbaImage::from_fn(40, 20, |x, _| if x < 20 { RED } else { GREEN })
    }

    // --- fit ---------------------------------------------------------------

    #[test]
    fn contain_is_the_plain_square_icon() {
        let img = wide();
        let plain = make_square_icon(&img, 32, 0, false);
        let made = make_icon(&img, 32, &layout(FitMode::Contain, 0, 0), false);
        assert_eq!(plain, made);
    }

    #[test]
    fn contain_leaves_bars_and_cover_fills_the_square() {
        let img = wide();
        let contain = make_icon(&img, 32, &layout(FitMode::Contain, 0, 0), false);
        assert_eq!(contain.get_pixel(16, 1).0[3], 0, "bar above the picture");

        let cover = make_icon(&img, 32, &layout(FitMode::Cover, 0, 0), false);
        assert_eq!(cover.dimensions(), (32, 32));
        for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31), (16, 1), (16, 30)] {
            assert_eq!(cover.get_pixel(x, y).0[3], 255, "({x},{y}) is covered");
        }
        // The middle square of the 2:1 picture: 10..30 of 40 - red and green
        // each fill one half of the icon.
        assert_eq!(cover.get_pixel(4, 16).0, RED.0);
        assert_eq!(cover.get_pixel(27, 16).0, GREEN.0);
    }

    #[test]
    fn cover_with_padding_fills_only_the_inner_square() {
        let img = wide();
        let cover = make_icon(&img, 100, &layout(FitMode::Cover, 20, 0), false);
        assert_eq!(cover.get_pixel(5, 50).0[3], 0, "margin");
        assert_eq!(cover.get_pixel(15, 15).0[3], 255, "inner square corner");
        assert_eq!(cover.get_pixel(84, 84).0[3], 255);
        assert_eq!(cover.get_pixel(95, 50).0[3], 0, "margin");
    }

    #[test]
    fn cover_of_a_square_picture_equals_contain() {
        let img = RgbaImage::from_pixel(20, 20, RED);
        let a = make_icon(&img, 16, &layout(FitMode::Cover, 0, 0), false);
        let b = make_icon(&img, 16, &layout(FitMode::Contain, 0, 0), false);
        assert_eq!(a.get_pixel(0, 0), b.get_pixel(0, 0));
        assert_eq!(a.get_pixel(15, 15), b.get_pixel(15, 15));
    }

    // --- corner radius -------------------------------------------------------

    #[test]
    fn rounded_corners_clear_the_corners_and_keep_the_middle() {
        let img = RgbaImage::from_pixel(64, 64, RED);
        let icon = make_icon(&img, 64, &layout(FitMode::Contain, 0, 25), false);
        assert_eq!(icon.get_pixel(0, 0).0[3], 0, "corner");
        assert_eq!(icon.get_pixel(63, 63).0[3], 0, "corner");
        assert_eq!(icon.get_pixel(32, 0).0[3], 255, "middle of an edge");
        assert_eq!(icon.get_pixel(32, 32).0, RED.0);
        // The curve is smooth: somewhere along it there is a partly
        // transparent pixel.
        let partly = (0..16).any(|i| {
            let a = icon.get_pixel(i, i).0[3];
            a > 0 && a < 255
        });
        assert!(partly, "an anti-aliased edge");
    }

    #[test]
    fn a_radius_of_fifty_percent_makes_a_circle() {
        let img = RgbaImage::from_pixel(64, 64, RED);
        let icon = make_icon(&img, 64, &layout(FitMode::Contain, 0, 50), false);
        assert_eq!(icon.get_pixel(5, 5).0[3], 0);
        assert_eq!(icon.get_pixel(32, 1).0[3], 255);
        assert_eq!(icon.get_pixel(1, 32).0[3], 255);
    }

    #[test]
    fn the_rounding_follows_the_placed_picture_not_the_canvas() {
        // A 2:1 picture in a square canvas: its own corners are rounded, the
        // transparent bars stay transparent.
        let img = RgbaImage::from_pixel(40, 20, RED);
        let icon = make_icon(&img, 40, &layout(FitMode::Contain, 0, 50), false);
        // The picture sits at y 10..30; its middle row is y 20.
        assert!(icon.get_pixel(0, 20).0[3] > 240, "left end of the pill");
        assert_eq!(icon.get_pixel(1, 20).0[3], 255);
        assert_eq!(icon.get_pixel(1, 10).0[3], 0, "rounded corner");
        assert_eq!(icon.get_pixel(20, 5).0[3], 0, "bar above");
        assert_eq!(icon.get_pixel(20, 20).0, RED.0);
    }

    #[test]
    fn rounding_keeps_existing_transparency_and_zero_changes_nothing() {
        let mut img = RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 100]));
        let before = img.clone();
        round_corners(&mut img, 0, 0, 8, 8, 0);
        assert_eq!(img, before);
        round_corners(&mut img, 0, 0, 8, 8, 30);
        assert_eq!(img.get_pixel(4, 0).0, [1, 2, 3, 100]);
        assert!(
            img.get_pixel(0, 0).0[3] < 50,
            "the corner is mostly cut away"
        );
    }

    // --- crop --------------------------------------------------------------

    #[test]
    fn crop_values_are_parsed_and_checked() {
        assert_eq!(
            parse_crop("10, 20,30,40").unwrap(),
            CropRect {
                x: 10,
                y: 20,
                width: 30,
                height: 40
            }
        );
        for bad in [
            "",
            "1,2,3",
            "1,2,3,4,5",
            "a,b,c,d",
            "1,2,-3,4",
            "0,0,0,5",
            "0,0,5,0",
        ] {
            assert!(parse_crop(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn crop_cuts_out_the_rectangle() {
        let cut = crop(&wide(), parse_crop("15,5,10,10").unwrap()).unwrap();
        assert_eq!(cut.dimensions(), (10, 10));
        assert_eq!(cut.get_pixel(0, 0).0, RED.0);
        assert_eq!(cut.get_pixel(9, 9).0, GREEN.0);
    }

    #[test]
    fn a_crop_outside_the_image_is_an_error_naming_the_size() {
        for bad in ["30,0,20,10", "0,15,10,10", "40,0,1,1"] {
            let err = crop(&wide(), parse_crop(bad).unwrap()).unwrap_err();
            assert!(err.contains("40x20"), "{bad}: {err}");
        }
        assert!(crop(&wide(), parse_crop("0,0,40,20").unwrap()).is_ok());
        let huge = CropRect {
            x: u32::MAX,
            y: 0,
            width: u32::MAX,
            height: 1,
        };
        assert!(crop(&wide(), huge).is_err(), "no overflow");
    }

    // --- trim --------------------------------------------------------------

    #[test]
    fn trim_cuts_the_transparent_margin() {
        let mut img = RgbaImage::new(20, 20);
        for y in 5..9 {
            for x in 3..13 {
                img.put_pixel(x, y, RED);
            }
        }
        let Trimmed::Cut(cut) = trim_transparent(&img) else {
            panic!("expected a cut");
        };
        assert_eq!(cut.dimensions(), (10, 4));
        assert_eq!(cut.get_pixel(0, 0).0, RED.0);
        assert_eq!(cut.get_pixel(9, 3).0, RED.0);
    }

    #[test]
    fn a_nearly_transparent_pixel_still_counts_as_content() {
        let mut img = RgbaImage::new(10, 10);
        img.put_pixel(7, 2, Rgba([0, 0, 0, 1]));
        let Trimmed::Cut(cut) = trim_transparent(&img) else {
            panic!("expected a cut");
        };
        assert_eq!(cut.dimensions(), (1, 1));
    }

    #[test]
    fn trim_leaves_an_image_without_margin_and_reports_an_empty_one() {
        assert_eq!(trim_transparent(&wide()), Trimmed::NothingToCut);
        assert_eq!(trim_transparent(&RgbaImage::new(5, 5)), Trimmed::Empty);
    }

    // --- sizes auto -----------------------------------------------------------

    #[test]
    fn auto_sizes_keep_what_the_source_can_supply() {
        let all = [16, 32, 48, 64, 128, 256];
        assert_eq!(auto_sizes(&all, 256), all.to_vec());
        assert_eq!(auto_sizes(&all, 1000), all.to_vec());
        assert_eq!(auto_sizes(&all, 100), vec![16, 32, 48, 64]);
        assert_eq!(auto_sizes(&all, 64), vec![16, 32, 48, 64]);
    }

    #[test]
    fn a_tiny_source_gets_one_icon_at_its_own_size() {
        let all = [16, 32];
        assert_eq!(auto_sizes(&all, 12), vec![12]);
        assert_eq!(auto_sizes(&all, 0), vec![1]);
    }
}
