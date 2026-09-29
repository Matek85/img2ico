// Everything related to turning a source image into a correctly-sized,
// correctly-padded square icon: the alpha-aware resize logic, and the
// upscaling warning that goes along with it.

use image::imageops::FilterType;
use image::{Rgba, RgbaImage};

/// Warns (once, to stderr) if any of the given target sizes would exceed
/// the source image's native resolution - i.e. would need to be
/// upscaled. Not an error: upscaling still produces a perfectly valid
/// icon, just a softer one, since resizing up can't add detail that
/// wasn't in the source to begin with (see make_square_icon /
/// resize_rgba_premultiplied below for how the resize itself works -
/// Lanczos3 makes the result smooth rather than blocky, but it's still an
/// estimate, not real detail). `silent` suppresses this - see --silent's
/// own doc comment in cli.rs for exactly what that flag covers.
pub fn warn_about_upscaling(source_width: u32, source_height: u32, sizes: &[u32], silent: bool) {
    if silent {
        return;
    }
    let native_max = source_width.max(source_height);
    let upscaled: Vec<u32> = sizes.iter().copied().filter(|&s| s > native_max).collect();
    if !upscaled.is_empty() {
        eprintln!(
            "Warning: the source image is {source_width}x{source_height} pixels, smaller than {upscaled:?} - those sizes will be upscaled and may look soft or blurry rather than sharp. For crisp results at every size, use a higher-resolution source image."
        );
    }
}

/// Below this many pixels, the shorter side of a letterboxed/pillarboxed
/// image counts as "just a sliver" for warn_about_thin_content - not
/// invisible, but not really recognizable as the original artwork either.
const MIN_RECOGNIZABLE_CONTENT_PIXELS: u32 = 3;

/// Warns (once, to stderr) about icon sizes where either an extreme
/// source aspect ratio, a high --padding value, or the combination of
/// both leaves only a thin sliver of actual content.
///
/// Background: make_square_icon fits the source image into the icon
/// canvas WITHOUT distorting it (see its doc comment), which for a very
/// elongated source (e.g. an 800x100 banner, 8:1) means the shorter side
/// ends up tiny once scaled down - at a 16x16 icon, that 8:1 ratio leaves
/// only about 2 pixels of actual visible height, the rest is transparent
/// padding above and below. --padding shrinks the usable area further,
/// and can cause the exact same symptom completely on its own even for a
/// perfectly square source (e.g. --padding 100 leaves zero content
/// regardless of aspect ratio) - so the message below deliberately
/// doesn't blame one specific cause, since either one (or both together)
/// can be responsible. The result is still a valid, undistorted icon;
/// it's just one where the artwork itself may be hard to make out at
/// that particular size. This is a genuinely different situation from
/// warn_about_upscaling: the proportion of the icon actually covered by
/// content is the SAME at every size, so this only becomes a practical
/// problem once that fraction, applied to a given icon's pixel count,
/// rounds down to almost nothing - which is why this takes --padding and
/// every individual target size into account, rather than a single
/// blanket check.
pub fn warn_about_thin_content(
    source_width: u32,
    source_height: u32,
    padding_percent: u8,
    sizes: &[u32],
    silent: bool,
) {
    if silent || source_width == 0 || source_height == 0 {
        return; // guards the division below; shouldn't happen for a real image
    }
    let padding_percent = padding_percent.min(100);
    let minor = source_width.min(source_height) as f32;
    let major = source_width.max(source_height) as f32;

    let affected: Vec<String> = sizes
        .iter()
        .copied()
        .filter_map(|size| {
            let content_size = size as f32 * (1.0 - padding_percent as f32 / 100.0);
            let minor_scaled = (content_size * minor / major).round() as u32;
            if minor_scaled <= MIN_RECOGNIZABLE_CONTENT_PIXELS {
                Some(format!("{size}x{size} (~{minor_scaled}px of actual content)"))
            } else {
                None
            }
        })
        .collect();

    if !affected.is_empty() {
        eprintln!(
            "Warning: at these sizes, only a thin sliver of the actual artwork will be visible: {} - this can be caused by an elongated source image, a high --padding value, or both. Consider a less elongated source image and/or less padding if that looks too thin.",
            affected.join(", ")
        );
    }
}

/// Converts every pixel to grayscale (sets R=G=B to a single weighted
/// brightness value), leaving alpha completely untouched. Unlike
/// --chroma-key/--replace-color (which only affect a detected background
/// region), this uniformly affects every pixel in the image - the whole
/// point being "no color left anywhere", not just in the background.
///
/// Applied as the LAST color transformation in the pipeline (see main.rs's
/// run()), after any --chroma-key/--replace-color processing - so if
/// someone combines --grayscale with --replace-color, the replacement
/// color itself also ends up grayscaled rather than staying colorful,
/// which matches what "the whole icon is grayscale" should mean without
/// a confusing exception.
pub fn apply_grayscale(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        // ITU-R BT.601 luma weights - the standard formula for how bright
        // an RGB color appears to the human eye: green contributes the
        // most (eyes are most sensitive to it), blue the least. Using
        // this instead of a plain average (r+g+b)/3 keeps the perceived
        // brightness of the original colors intact - a naive average
        // makes pure blue look almost as bright as pure yellow, even
        // though blue looks much darker to us.
        let gray = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
        pixel[0] = gray;
        pixel[1] = gray;
        pixel[2] = gray;
        // pixel[3] (alpha) is deliberately left as-is.
    }
}

/// Returns true if any pixel in the image has an alpha value below 255 -
/// i.e. whether the image has any transparency at all.
///
/// Used to skip the premultiply/un-premultiply round trip in
/// resize_rgba_premultiplied for fully opaque images: for those, that
/// round trip is mathematically a no-op anyway (every channel gets
/// multiplied by 1.0, then divided by 1.0 again), but would still cost a
/// full extra pass over every single pixel for nothing. Computed ONCE by
/// the caller (see "has_alpha" in main.rs's run()) rather than repeating
/// this same full-image scan for every single icon size.
pub fn has_transparency(img: &RgbaImage) -> bool {
    img.pixels().any(|p| p[3] != 255)
}

/// Resizes an RGBA image with Lanczos3 filtering, taking the alpha-fringe
/// fix in resize_rgba_premultiplied only when the image actually has any
/// transparency to worry about - see has_transparency() above for why.
fn resize_rgba(src: &RgbaImage, new_width: u32, new_height: u32, has_alpha: bool) -> RgbaImage {
    if has_alpha {
        resize_rgba_premultiplied(src, new_width, new_height)
    } else {
        image::imageops::resize(src, new_width, new_height, FilterType::Lanczos3)
    }
}

/// Downscales an RGBA image "alpha-correctly", without a colored fringe
/// appearing at transparent/semi-transparent edges.
///
/// Background: A resize filter (Lanczos3 here) averages several old
/// neighboring pixels for every new pixel. If a neighboring pixel still
/// has the old background color in its RGB channels (e.g. due to our
/// chroma key, where only the alpha channel was set to 0), that
/// "invisible" but technically still-present color would still factor
/// into the calculation when downscaling and become visible as a faint
/// color haze at the edges.
///
/// The standard solution for this is called "premultiplied alpha": before
/// resizing, we multiply every color channel by its own alpha value (a
/// fully transparent pixel thereby becomes (0,0,0), regardless of its
/// original color). After resizing we compute this back
/// ("un-premultiply"). This way invisible pixels no longer carry any
/// disruptive residual color into the calculation.
fn resize_rgba_premultiplied(src: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (width, height) = src.dimensions();

    // Step 1: convert into the "premultiplied" representation.
    let mut premultiplied = RgbaImage::new(width, height);
    for (x, y, pixel) in src.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        let r = (pixel[0] as f32 * alpha).round() as u8;
        let g = (pixel[1] as f32 * alpha).round() as u8;
        let b = (pixel[2] as f32 * alpha).round() as u8;
        premultiplied.put_pixel(x, y, Rgba([r, g, b, pixel[3]]));
    }

    // Step 2: resize completely normally, as before. Since the color
    // channels now already have the alpha weighting "baked in", the
    // resize behaves consistently across all four channels.
    let resized = image::imageops::resize(
        &premultiplied,
        new_width,
        new_height,
        FilterType::Lanczos3,
    );

    // Step 3: compute back ("un-premultiply") - divide every color channel
    // by its (new, resized) alpha value again. Without this step, all
    // semi-transparent areas would be too dark after resizing.
    let mut result = RgbaImage::new(new_width, new_height);
    for (x, y, pixel) in resized.enumerate_pixels() {
        let alpha = pixel[3];
        if alpha == 0 {
            // Fully transparent: color no longer matters, black is a safe,
            // neutral choice.
            result.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        } else {
            let alpha_f = alpha as f32 / 255.0;
            let r = ((pixel[0] as f32 / alpha_f).round()).clamp(0.0, 255.0) as u8;
            let g = ((pixel[1] as f32 / alpha_f).round()).clamp(0.0, 255.0) as u8;
            let b = ((pixel[2] as f32 / alpha_f).round()).clamp(0.0, 255.0) as u8;
            result.put_pixel(x, y, Rgba([r, g, b, alpha]));
        }
    }
    result
}

/// Scales an image into a square icon of the desired edge length `size`,
/// without distortion.
///
/// Expects the source image already as RGBA (not as DynamicImage), so the
/// (relatively expensive) format conversion happens only ONCE in main.rs's
/// run(), instead of again for every icon size that gets generated - see
/// the comment there near "rgba_source".
///
/// Important for transparency: if the source image isn't square, a plain
/// "resize_exact" (bluntly stretching to size x size) would distort the
/// image. Instead we scale proportionally ("fit inside") and place the
/// result centered on a fully transparent square canvas. The margins stay
/// transparent this way, instead of e.g. white or black.
///
/// `padding_percent` (0-100) additionally shrinks the area the artwork is
/// fitted into, leaving a transparent margin on all sides. 0 fits the
/// artwork as large as possible (previous behavior); e.g. 10 leaves
/// roughly a 10% margin around it.
///
/// `has_alpha` should be the result of has_transparency() on `rgba`,
/// computed ONCE by the caller and passed in here - see resize_rgba() for
/// why this matters.
pub fn make_square_icon(rgba: &RgbaImage, size: u32, padding_percent: u8, has_alpha: bool) -> RgbaImage {
    let (orig_w, orig_h) = rgba.dimensions();

    // The artwork is fitted into a smaller "content box" inside the full
    // canvas, shrunk by the padding percentage on each side.
    let padding_percent = padding_percent.min(100);
    let content_size = (size as f32 * (1.0 - padding_percent as f32 / 100.0)).max(1.0);

    // Choose the scale factor so the image fits completely inside the
    // content box, without changing the aspect ratio.
    let scale = (content_size / orig_w as f32).min(content_size / orig_h as f32);
    let new_w = ((orig_w as f32 * scale).round() as u32).max(1);
    let new_h = ((orig_h as f32 * scale).round() as u32).max(1);

    // Alpha-correct resize (see comment on resize_rgba_premultiplied) -
    // but only pay for it when there's actually alpha to worry about.
    let resized = resize_rgba(rgba, new_w, new_h, has_alpha);

    // Create the new target canvas, initialized fully transparent (0,0,0,0).
    let mut canvas = RgbaImage::new(size, size);
    // Compute the offset to center the scaled image.
    let x_offset = (size - new_w) / 2;
    let y_offset = (size - new_h) / 2;

    // "overlay" copies the scaled image onto the target canvas pixel by
    // pixel and already respects the source image's alpha channel (if the
    // original image itself already had transparent areas, e.g. a PNG
    // logo, those are preserved).
    image::imageops::overlay(&mut canvas, &resized, x_offset as i64, y_offset as i64);

    canvas
}