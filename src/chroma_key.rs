// Everything related to the --chroma-key feature: parsing the hex color
// the user gives on the command line, the flood-fill algorithm that turns
// a matching background region transparent, and --find's automatic
// discovery of additional same-colored regions the border-based flood
// fill can't reach on its own.

use image::{Rgba, RgbaImage};
use std::collections::VecDeque;

/// Converts a hex color code like "#00FF00" or "00ff00" into its
/// red/green/blue components (0-255 each).
///
/// Rust note: A "Result<T, String>" means: on success the function returns
/// a value of type T (here [u8; 3], i.e. an array of 3 bytes), on failure
/// an error message as a String instead. The caller has to decide via
/// match/if-let, or as here via unwrap_or_else()/the "?" operator, what
/// happens in the error case.
pub fn parse_hex_color(input: &str) -> Result<[u8; 3], String> {
    // Strip an optional leading '#', so both "#00ff00" and "00ff00" work.
    let hex = input.trim().trim_start_matches('#');

    // IMPORTANT (hardening): reject non-ASCII input before doing any
    // byte-index slicing below. Rust strings are UTF-8, where a single
    // character can take up multiple bytes - slicing at a fixed byte
    // position (like hex[0..2]) PANICS if that position happens to fall
    // in the middle of such a multi-byte character, instead of returning
    // an error. hex.len() alone doesn't protect against this: it counts
    // bytes, so e.g. "aØaaa" is 6 bytes (matching our length check right
    // below) despite only being 5 characters, with byte index 2 landing
    // inside "Ø". Checking is_ascii() first guarantees every byte is
    // exactly one character, so the slicing below can never panic.
    if !hex.is_ascii() {
        return Err(format!(
            "'{input}' is not a valid hex color code (must be plain ASCII hex digits)"
        ));
    }

    if hex.len() != 6 {
        return Err(format!(
            "'{input}' is not a valid hex color code (expected exactly 6 hexadecimal digits, e.g. 00FF00)"
        ));
    }

    // u8::from_str_radix(_, 16) interprets a text as a hexadecimal number.
    // We split the string into three two-character chunks (RR, GG, BB).
    let parse_byte = |s: &str| -> Result<u8, String> {
        u8::from_str_radix(s, 16).map_err(|_| format!("'{s}' is not a valid hexadecimal number"))
    };

    let r = parse_byte(&hex[0..2])?;
    let g = parse_byte(&hex[2..4])?;
    let b = parse_byte(&hex[4..6])?;

    Ok([r, g, b])
}

/// The largest possible distance between two RGB colors (black to white).
/// Needed to convert the user's 0-100 tolerance value into an actual
/// color-distance threshold.
const MAX_RGB_DISTANCE: f32 = 441.672_9; // sqrt(255^2 * 3)

/// Converts a 0-100 tolerance percentage into an actual color-distance
/// threshold. Shared by apply_chroma_key and find_isolated_regions so
/// both agree on exactly what counts as "close enough" to the target
/// color - if these ever disagreed, --find could suggest seeds for
/// regions that the real conversion would then handle differently.
fn tol_distance_from_percent(tolerance_percent: u8) -> f32 {
    MAX_RGB_DISTANCE * (tolerance_percent.min(100) as f32 / 100.0)
}

/// Computes the Euclidean distance between an image color and the target
/// color in RGB space. A distance of 0 means "identical", larger values
/// mean "less similar". We simply treat red/green/blue as coordinates in a
/// 3D space and compute the "ordinary" distance between two points
/// (Pythagorean theorem, just with 3 axes instead of 2).
fn color_distance(pixel: &Rgba<u8>, target: [u8; 3]) -> f32 {
    let dr = pixel[0] as f32 - target[0] as f32;
    let dg = pixel[1] as f32 - target[1] as f32;
    let db = pixel[2] as f32 - target[2] as f32;
    (dr * dr + dg * dg + db * db).sqrt()
}

/// The 4 direct neighbors (up/down/left/right) of (x, y), each as
/// `Some((nx, ny))` when it's actually inside the image, `None` when it
/// would fall outside. Shared by the two flood-fill algorithms in this
/// file (the border-based one in flood_fill_reachable, and the
/// component-labeling one in find_isolated_regions) so both walk the
/// image the exact same way.
fn in_bounds_neighbors(x: u32, y: u32, width: u32, height: u32) -> [Option<(u32, u32)>; 4] {
    [
        x.checked_sub(1).map(|nx| (nx, y)),
        Some(x + 1).filter(|&v| v < width).map(|nx| (nx, y)),
        y.checked_sub(1).map(|ny| (x, ny)),
        Some(y + 1).filter(|&v| v < height).map(|ny| (x, ny)),
    ]
}

/// Computes which pixels are reachable via flood fill from the image
/// border (plus any given extra seed points), matching `target` within
/// `tolerance_percent`. Returns a flat `width * height` vector where
/// `true` means "reachable" - see apply_chroma_key's doc comment for the
/// full explanation of why this border-based approach is safer than a
/// global color replace.
///
/// This is the shared core used by BOTH apply_chroma_key (which then
/// additionally reduces alpha for everything reachable) and
/// find_isolated_regions (which uses this purely to know what's ALREADY
/// reachable, so it only reports what isn't).
fn flood_fill_reachable(
    img: &RgbaImage,
    target: [u8; 3],
    tolerance_percent: u8,
    extra_seeds: &[(u32, u32)],
) -> Vec<bool> {
    let (width, height) = img.dimensions();
    let tol_distance = tol_distance_from_percent(tolerance_percent);

    // "visited" tracks, for every pixel, whether it's part of the
    // background region found from the border. Organized as a
    // one-dimensional vector (index = y * width + x), since Rust doesn't
    // have native dynamically-sized 2D arrays.
    let mut visited = vec![false; (width * height) as usize];
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };

    // Checks whether a pixel should count as "background": either already
    // fully transparent (then it already belongs to the background anyway,
    // and the flood fill should be able to pass through such areas), or
    // close enough to the target color.
    let is_background_candidate = |img: &RgbaImage, x: u32, y: u32| -> bool {
        let pixel = img.get_pixel(x, y);
        pixel[3] == 0 || color_distance(pixel, target) <= tol_distance
    };

    // The queue for the breadth-first search (BFS). We start with all
    // border pixels that satisfy the condition above.
    let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
    for x in 0..width {
        for &y in &[0, height - 1] {
            if !visited[idx(x, y)] && is_background_candidate(img, x, y) {
                visited[idx(x, y)] = true;
                queue.push_back((x, y));
            }
        }
    }
    for y in 0..height {
        for &x in &[0, width - 1] {
            if !visited[idx(x, y)] && is_background_candidate(img, x, y) {
                visited[idx(x, y)] = true;
                queue.push_back((x, y));
            }
        }
    }

    // Enqueue extra, user-specified (or --find --auto-apply-discovered)
    // starting points. These are ALWAYS accepted as a starting point
    // (even if their color doesn't perfectly match the target) - this
    // lets the flood fill "jump into" an enclosed area that would
    // otherwise be unreachable from the image border (e.g. because a
    // frame/ring sits in between). The actual spreading from that point
    // then works exactly as normal, via the tolerance comparison.
    for &(x, y) in extra_seeds {
        if x >= width || y >= height {
            eprintln!(
                "Warning: seed point ({x},{y}) is outside the image ({width}x{height}) and will be ignored."
            );
            continue;
        }
        if !visited[idx(x, y)] {
            visited[idx(x, y)] = true;
            queue.push_back((x, y));
        }
    }

    // Breadth-first search: from every pixel marked as "background", check
    // the 4 direct neighbors and, on a match, mark them as background too
    // and add them to the queue. This continues until no new connected
    // area is found.
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in in_bounds_neighbors(x, y, width, height).into_iter().flatten() {
            if !visited[idx(nx, ny)] && is_background_candidate(img, nx, ny) {
                visited[idx(nx, ny)] = true;
                queue.push_back((nx, ny));
            }
        }
    }

    visited
}

/// Removes a specific background color from an image ("chroma key") -
/// or, if `replacement` is given, REPLACES it with a different solid
/// color instead of making it transparent - WITHOUT accidentally
/// destroying identically-colored spots in the middle of the actual
/// subject.
///
/// The idea (flood fill): We start ONLY at the four image borders and
/// "walk" from there across all directly neighboring pixels that also
/// have (roughly) the target color. This is like the bucket fill tool in a
/// paint program, except we don't click with the mouse but start
/// automatically at every border pixel at once. Only pixels reachable via
/// such a "path" from the border count as background. An identical color
/// that randomly occurs in the MIDDLE of the image, not connected to the
/// border, stays untouched as a result - that's exactly what compensates
/// for the risk you mentioned. (See find_isolated_regions below for a way
/// to automatically discover such spots when they're ALSO meant to be
/// affected.)
///
/// For a soft rather than hard-cut transition, the change is additionally
/// applied proportionally to the color distance: pixels that (almost)
/// exactly match the target color are fully affected; pixels that just
/// barely fall within the tolerance are only slightly affected. This
/// avoids an ugly, jagged edge - whether that edge is a transparency
/// cutoff (`replacement: None`) or a color-replacement cutoff
/// (`replacement: Some(...)`).
pub fn apply_chroma_key(
    img: &mut RgbaImage,
    target: [u8; 3],
    tolerance_percent: u8,
    extra_seeds: &[(u32, u32)],
    replacement: Option<[u8; 3]>,
) {
    let (width, height) = img.dimensions();
    let tol_distance = tol_distance_from_percent(tolerance_percent);
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };

    let visited = flood_fill_reachable(img, target, tolerance_percent, extra_seeds);

    // Second pass: change every pixel identified as background - either
    // its alpha (transparency mode) or its RGB color (replacement mode).
    //
    // IMPORTANT: Instead of a single linear transition across the ENTIRE
    // tolerance range (0 to tol_distance), we use two zones:
    //   - "core zone" (distance <= core_distance): pixels are HARD-set to
    //     the full effect (fully transparent, or fully the replacement
    //     color), regardless of the tiniest color deviations. This
    //     matters because real-world backgrounds (JPEG artifacts, slight
    //     gradients) almost never match the given target color 100%
    //     exactly - without a core zone there would practically always be
    //     a tiny residual (5 out of 255 alpha left over, or a barely-off
    //     shade of the replacement color) - barely visible on its own,
    //     but in some display contexts (e.g. Windows Explorer, for the
    //     transparency case) it can show up as a faint haze over the
    //     ENTIRE area - exactly the problem we're fixing here.
    //   - "feather zone" (core_distance < distance <= tol_distance): the
    //     soft, linear transition from before is kept here. This zone
    //     then only affects real edges of the subject, not the whole
    //     background area anymore.
    let core_distance = tol_distance * 0.5;

    for y in 0..height {
        for x in 0..width {
            if !visited[idx(x, y)] {
                continue;
            }
            let pixel = img.get_pixel_mut(x, y);
            if pixel[3] == 0 {
                continue; // already fully transparent, nothing to do
            }
            let distance = color_distance(pixel, target);
            // scale = 0.0 in the core zone (full effect), ramping up to
            // 1.0 at the tolerance boundary (no effect) - shared by both
            // modes below, just applied to a different channel.
            let scale = if distance <= core_distance {
                0.0
            } else if tol_distance > core_distance {
                ((distance - core_distance) / (tol_distance - core_distance)).clamp(0.0, 1.0)
            } else {
                // Edge case (tolerance very small/0): no feather zone
                // exists, everything outside the core zone stays unchanged.
                1.0
            };

            match replacement {
                None => {
                    // Transparency mode (the original behavior): fade
                    // alpha down towards 0.
                    pixel[3] = (pixel[3] as f32 * scale).round() as u8;
                }
                Some(new_color) => {
                    // Replacement mode: blend the pixel's RGB towards the
                    // new color instead - "how much of the original color
                    // to keep" is just `scale` again, applied per
                    // channel. Alpha is deliberately left untouched: this
                    // is a solid recolor, not a transparency change.
                    for channel in 0..3 {
                        let original = pixel[channel] as f32;
                        let target_channel = new_color[channel] as f32;
                        pixel[channel] = (original * scale + target_channel * (1.0 - scale))
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

/// One region --find discovered: how many pixels it covers, and a single
/// representative point inside it suitable for use as a --seed value.
pub struct FoundRegion {
    pub pixel_count: usize,
    pub seed: (u32, u32),
}

/// Default for --find-min-size: regions smaller than this (in pixels) are
/// ignored by find_isolated_regions unless the user overrides it - a
/// handful of stray pixels is almost always anti-aliasing/JPEG noise
/// rather than a deliberate area someone would want to affect, and
/// reporting every single one would bury the genuinely useful suggestions
/// in noise. Set --find-min-size 1 to catch every matching pixel, however
/// small.
pub const DEFAULT_FIND_MIN_SIZE: usize = 9;

/// Finds additional regions matching `target` that the border-based flood
/// fill in apply_chroma_key can NOT reach on its own - i.e. exactly the
/// spots --seed is for, discovered automatically instead of requiring
/// someone to hunt for pixel coordinates in an image editor first.
///
/// How: first compute what the normal border flood fill would already
/// reach (flood_fill_reachable, with no extra seeds). Then scan every
/// remaining pixel for a color match; whatever's left is grouped into
/// separate connected regions the same way (flood fill), just without
/// requiring a path back to the border. Each resulting region becomes one
/// suggested seed point (the region's own pixel closest to its centroid,
/// so the point is guaranteed to actually be part of the region even for
/// oddly-shaped areas) - one entry in the returned list per region,
/// smaller than `min_region_size` filtered out (see DEFAULT_FIND_MIN_SIZE
/// above for the reasoning on why that's not simply 1 by default).
pub fn find_isolated_regions(
    img: &RgbaImage,
    target: [u8; 3],
    tolerance_percent: u8,
    min_region_size: usize,
) -> Vec<FoundRegion> {
    let (width, height) = img.dimensions();
    let tol_distance = tol_distance_from_percent(tolerance_percent);
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };

    let is_candidate = |x: u32, y: u32| -> bool {
        let pixel = img.get_pixel(x, y);
        pixel[3] == 0 || color_distance(pixel, target) <= tol_distance
    };

    // Anything the normal border-based flood fill already reaches doesn't
    // need to be "found" - it'll be handled anyway. We start our own
    // labeling from this, treating those pixels as already accounted for.
    let mut labeled = flood_fill_reachable(img, target, tolerance_percent, &[]);

    let mut regions = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if labeled[idx(x, y)] || !is_candidate(x, y) {
                continue;
            }

            // Found the start of a new, previously-unaccounted-for region -
            // flood fill across it (same 4-connectivity as everywhere else
            // in this file), collecting every member pixel.
            let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
            let mut members: Vec<(u32, u32)> = Vec::new();
            labeled[idx(x, y)] = true;
            queue.push_back((x, y));

            while let Some((cx, cy)) = queue.pop_front() {
                members.push((cx, cy));
                for (nx, ny) in in_bounds_neighbors(cx, cy, width, height).into_iter().flatten() {
                    if !labeled[idx(nx, ny)] && is_candidate(nx, ny) {
                        labeled[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }

            if members.len() < min_region_size {
                continue;
            }

            // Representative point: the actual member pixel closest to the
            // region's centroid. Using the raw centroid coordinate itself
            // could land outside the region for a concave/oddly-shaped
            // area (e.g. a crescent) - snapping to the nearest real member
            // guarantees the suggested --seed value is actually inside it.
            let (sum_x, sum_y) = members
                .iter()
                .fold((0u64, 0u64), |(sx, sy), &(px, py)| {
                    (sx + px as u64, sy + py as u64)
                });
            let centroid_x = (sum_x / members.len() as u64) as u32;
            let centroid_y = (sum_y / members.len() as u64) as u32;

            let seed = *members
                .iter()
                .min_by_key(|&&(mx, my)| {
                    let dx = mx as i64 - centroid_x as i64;
                    let dy = my as i64 - centroid_y as i64;
                    dx * dx + dy * dy
                })
                .expect("members is non-empty: we just pushed at least one pixel into it");

            regions.push(FoundRegion {
                pixel_count: members.len(),
                seed,
            });
        }
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // --- Fixed regression test -------------------------------------------
    //
    // This is the exact input that used to crash the program (see the
    // project's history): a Unicode character ("Ø", 2 bytes in UTF-8)
    // positioned so that our old fixed-byte-position slicing (hex[0..2],
    // hex[2..4], hex[4..6]) would cut it in half. Keeping this as its own
    // named test - separate from the property test below - means that
    // even if someone changes the property test later, this exact
    // historical case still gets checked on every test run.
    #[test]
    fn parse_hex_color_rejects_multibyte_unicode_without_panicking() {
        let result = parse_hex_color("aØaaa");
        assert!(result.is_err(), "expected an error, got {result:?}");
    }

    proptest! {
        /// parse_hex_color must return a normal Ok/Err for absolutely any
        /// string - never panic. This property, run automatically, would
        /// have caught the "aØaaa" bug above by itself, without anyone
        /// having to think up that specific input by hand. input in ".*"
        /// tells proptest to generate arbitrary strings - not just ASCII
        /// text, but the full range of valid Unicode.
        #[test]
        fn parse_hex_color_never_panics(input in ".*") {
            let _ = parse_hex_color(&input);
        }

        /// Round-trip property: any RGB triple, formatted as a 6-digit hex
        /// string the way a user would type it (e.g. "FF00AA"), must parse
        /// back to exactly those same three bytes.
        #[test]
        fn parse_hex_color_roundtrip(r: u8, g: u8, b: u8) {
            let hex = format!("{r:02X}{g:02X}{b:02X}");
            prop_assert_eq!(parse_hex_color(&hex), Ok([r, g, b]));
        }
    }
}