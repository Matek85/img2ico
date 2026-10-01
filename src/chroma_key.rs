// Everything related to the --chroma-key feature: parsing the hex color
// the user gives on the command line, the flood-fill algorithm that turns
// a matching background region transparent, and --find's automatic
// discovery of additional same-colored regions the border-based flood
// fill can't reach on its own.

use crate::util::{file_prefix, warn};
use image::{Rgba, RgbaImage};
use std::collections::{BTreeMap, VecDeque};

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

/// The SQUARED distance between a pixel's color and the target, as a whole
/// number. Comparing squared distances against a squared threshold gives
/// the same answer as comparing real distances, but needs no square root.
/// That matters because this comparison runs once for every pixel of the
/// image, which is where the time goes for large sources. The per-channel
/// differences are whole numbers of at most 255, so the sum (at most
/// 195,075) is exact.
///
/// `pixel` is one pixel's RGBA bytes; only the color channels are used.
fn squared_color_distance(pixel: &[u8; 4], target: [u8; 3]) -> u32 {
    let dr = u32::from(pixel[0].abs_diff(target[0]));
    let dg = u32::from(pixel[1].abs_diff(target[1]));
    let db = u32::from(pixel[2].abs_diff(target[2]));
    dr * dr + dg * dg + db * db
}

/// The largest squared distance that still counts as "within
/// `max_distance`" - so `squared_color_distance(..) <= limit` is exactly
/// equivalent to `color_distance(..) <= max_distance`, without taking a
/// square root per pixel.
///
/// Starting from the mathematical answer (`max_distance` squared, rounded
/// down) and then nudging it until it agrees with the real `sqrt`
/// comparison makes the equivalence hold even at the boundary, where
/// floating-point rounding could otherwise decide a pixel differently
/// than before - the two approaches must never disagree about a single
/// pixel.
fn squared_distance_limit(max_distance: f32) -> u32 {
    let mut limit = (max_distance * max_distance) as u32;
    while ((limit + 1) as f32).sqrt() <= max_distance {
        limit += 1;
    }
    while limit > 0 && (limit as f32).sqrt() > max_distance {
        limit -= 1;
    }
    limit
}

/// Decides for every pixel, in one linear pass over the image, whether it
/// counts as "background": either already fully transparent (then it
/// already belongs to the background anyway, and the flood fill should be
/// able to pass through such areas), or close enough to the target color.
///
/// Computing this once up front (instead of re-checking a pixel every
/// time a neighbor looks at it) means each pixel's color is examined
/// exactly once, no matter how often the flood fill visits its
/// neighborhood. Shared by the border flood fill and --find's region
/// labeling, so both agree on what "background" means.
fn background_candidates(img: &RgbaImage, target: [u8; 3], tolerance_percent: u8) -> Vec<bool> {
    let limit = squared_distance_limit(tol_distance_from_percent(tolerance_percent));
    img.as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3] == 0 || squared_color_distance(pixel, target) <= limit)
        .collect()
}

/// The 4 direct neighbors (up/down/left/right) of (x, y), each as
/// `Some((nx, ny))` when it's actually inside the image, `None` when it
/// would fall outside. Used by --find's region labeling, which has to
/// visit a region's pixels in breadth-first order (see
/// find_isolated_regions for why the order matters there).
fn in_bounds_neighbors(x: u32, y: u32, width: u32, height: u32) -> [Option<(u32, u32)>; 4] {
    [
        x.checked_sub(1).map(|nx| (nx, y)),
        Some(x + 1).filter(|&v| v < width).map(|nx| (nx, y)),
        y.checked_sub(1).map(|ny| (x, ny)),
        Some(y + 1).filter(|&v| v < height).map(|ny| (x, ny)),
    ]
}

/// The position of pixel (x, y) in a flat, row-by-row `width * height`
/// buffer such as the ones below. Computed in `usize` so large images
/// can't overflow a 32-bit multiplication.
fn flat_index(x: u32, y: u32, width: u32) -> usize {
    y as usize * width as usize + x as usize
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
    silent: bool,
) -> Vec<bool> {
    let candidates = background_candidates(img, target, tolerance_percent);
    flood_fill_from_candidates(img.width(), img.height(), &candidates, extra_seeds, silent)
}

/// The flood fill itself, working on a precomputed candidate mask (see
/// background_candidates) instead of on colors: starting from every
/// candidate pixel on the image border (plus the extra seeds), it spreads
/// through all 4-connected candidate pixels.
///
/// This is a "scanline" fill: rather than handling one pixel at a time, it
/// grows a whole horizontal run of candidates at once, then looks at the
/// rows directly above and below that run for the next runs to grow.
/// That reaches exactly the same set of pixels as stepping from pixel to
/// pixel would (the result doesn't depend on the order), but touches
/// memory in long sequential stretches and needs far fewer stack
/// operations - a large, uniform background is a handful of runs per row
/// instead of one queue entry per pixel.
fn flood_fill_from_candidates(
    width: u32,
    height: u32,
    candidates: &[bool],
    extra_seeds: &[(u32, u32)],
    silent: bool,
) -> Vec<bool> {
    let (w, h) = (width as usize, height as usize);

    // "visited" tracks, for every pixel, whether it's part of the
    // background region found from the border. Organized as a
    // one-dimensional vector (index = y * width + x), since Rust doesn't
    // have native dynamically-sized 2D arrays.
    let mut visited = vec![false; w * h];
    if w == 0 || h == 0 {
        return visited;
    }

    // Run starting points: pixels that are already marked as visited, but
    // whose horizontal run hasn't been grown (and whose neighboring rows
    // haven't been looked at) yet.
    let mut starts: Vec<(usize, usize)> = Vec::new();

    // Marks (x, y) as visited and queues it, if it's a background
    // candidate that hasn't been reached yet.
    let mut start_at = |visited: &mut [bool], x: usize, y: usize| {
        let i = y * w + x;
        if !visited[i] && candidates[i] {
            visited[i] = true;
            starts.push((x, y));
        }
    };

    // Start with all border pixels that satisfy the condition.
    for x in 0..w {
        start_at(&mut visited, x, 0);
        start_at(&mut visited, x, h - 1);
    }
    for y in 0..h {
        start_at(&mut visited, 0, y);
        start_at(&mut visited, w - 1, y);
    }

    // Enqueue extra, user-specified (or --find --auto-apply-discovered)
    // starting points. These are ALWAYS accepted as a starting point
    // (even if their color doesn't perfectly match the target) - this
    // lets the flood fill "jump into" an enclosed area that would
    // otherwise be unreachable from the image border (e.g. because a
    // frame/ring sits in between). The actual spreading from that point
    // then works exactly as normal, via the candidate mask.
    for &(x, y) in extra_seeds {
        if x >= width || y >= height {
            let prefix = file_prefix();
            warn(
                silent,
                format_args!(
                    "Warning: {prefix}seed point ({x},{y}) is outside the image ({width}x{height}) and will be ignored."
                ),
            );
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        if !visited[y * w + x] {
            visited[y * w + x] = true;
            starts.push((x, y));
        }
    }

    while let Some((x, y)) = starts.pop() {
        let row = y * w;

        // Grow the run left and right from the starting pixel, for as long
        // as the neighboring pixel is an unreached candidate.
        let mut left = x;
        while left > 0 && !visited[row + left - 1] && candidates[row + left - 1] {
            left -= 1;
            visited[row + left] = true;
        }
        let mut right = x;
        while right + 1 < w && !visited[row + right + 1] && candidates[row + right + 1] {
            right += 1;
            visited[row + right] = true;
        }

        // Look at the pixels directly above and below the run. Only the
        // first pixel of each stretch of reachable candidates needs to
        // become a new starting point - growing it sideways finds the rest.
        for neighbor_y in [y.checked_sub(1), Some(y + 1).filter(|&ny| ny < h)]
            .into_iter()
            .flatten()
        {
            let neighbor_row = neighbor_y * w;
            let mut in_stretch = false;
            for nx in left..=right {
                let reachable = !visited[neighbor_row + nx] && candidates[neighbor_row + nx];
                if reachable && !in_stretch {
                    visited[neighbor_row + nx] = true;
                    starts.push((nx, neighbor_y));
                }
                in_stretch = reachable;
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
///
/// `feather_percent` (0-100) says how much of the tolerance range is used
/// for that soft transition (see `DEFAULT_FEATHER`); the rest is the hard
/// "core" around the target color. Returns how many pixels were affected -
/// background pixels that were not already fully transparent - so the caller
/// can tell when the key removed nothing, or nearly everything.
pub fn apply_chroma_key_feathered(
    img: &mut RgbaImage,
    target: [u8; 3],
    tolerance_percent: u8,
    feather_percent: u8,
    extra_seeds: &[(u32, u32)],
    replacement: Option<[u8; 3]>,
    silent: bool,
) -> usize {
    let tol_distance = tol_distance_from_percent(tolerance_percent);

    let visited = flood_fill_reachable(img, target, tolerance_percent, extra_seeds, silent);

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
    let core_distance = tol_distance * (1.0 - feather_percent.min(100) as f32 / 100.0);
    // The core zone covers the bulk of a typical background, so it gets
    // the cheap whole-number test; only pixels outside it need the real
    // distance (for the feather's gradual blend).
    let core_limit = squared_distance_limit(core_distance);

    let mut affected = 0usize;
    for (pixel, &reached) in img.pixels_mut().zip(&visited) {
        if !reached || pixel[3] == 0 {
            continue; // not background, or already fully transparent
        }
        affected += 1;
        // scale = 0.0 in the core zone (full effect), ramping up to
        // 1.0 at the tolerance boundary (no effect) - shared by both
        // modes below, just applied to a different channel.
        let scale = if squared_color_distance(&pixel.0, target) <= core_limit {
            0.0
        } else if tol_distance > core_distance {
            let distance = color_distance(pixel, target);
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
    affected
}

/// `apply_chroma_key_feathered` with the default feather, for the tests that
/// are not about the feather.
#[cfg(test)]
fn apply_chroma_key(
    img: &mut RgbaImage,
    target: [u8; 3],
    tolerance_percent: u8,
    extra_seeds: &[(u32, u32)],
    replacement: Option<[u8; 3]>,
    silent: bool,
) -> usize {
    apply_chroma_key_feathered(
        img,
        target,
        tolerance_percent,
        DEFAULT_FEATHER,
        extra_seeds,
        replacement,
        silent,
    )
}

/// Default for --feather: half of the tolerance range is the hard core
/// around the background color, the other half the soft transition. (The
/// behavior img2ico has always had.)
pub const DEFAULT_FEATHER: u8 = 50;

/// Warns when a chroma key did next to nothing, or nearly everything -
/// both usually mean the color or the tolerance is not what was intended.
/// `affected` is what `apply_chroma_key_feathered` returned, `total` the
/// number of pixels in the image, `color` the background color as shown to
/// the user. Counted as a warning either way; `silent` only hides the text.
pub fn warn_about_removal_extent(affected: usize, total: usize, color: &str, silent: bool) {
    if total == 0 {
        return;
    }
    let share = affected as f64 / total as f64;
    let prefix = file_prefix();
    if affected == 0 {
        warn(
            silent,
            format_args!(
                "Warning: {prefix}nothing was removed - no pixel connected to the image border is close to {color}. Check the color and --tolerance (or let img2ico detect it with --chroma-key auto)."
            ),
        );
    } else if share < MIN_EXPECTED_SHARE {
        warn(
            silent,
            format_args!(
                "Warning: {prefix}only {:.1}% of the image matched the background color {color} - almost nothing was removed. Check the color and --tolerance.",
                share * 100.0
            ),
        );
    } else if share > MAX_EXPECTED_SHARE {
        warn(
            silent,
            format_args!(
                "Warning: {prefix}{:.1}% of the image matched the background color {color} - almost everything was removed. The color may be too close to the artwork's, or --tolerance too high.",
                share * 100.0
            ),
        );
    }
}

/// Below this share of the image, a removal counts as "almost nothing".
const MIN_EXPECTED_SHARE: f64 = 0.005;

/// Above this share of the image, a removal counts as "almost everything".
const MAX_EXPECTED_SHARE: f64 = 0.98;

/// The background color found by `detect_background_color`.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedBackground {
    pub color: [u8; 3],
    /// The share (0-1) of the opaque border pixels within the tolerance of
    /// `color` - how clearly the border has just one color.
    pub coverage: f32,
}

/// The color channel values are grouped in steps of this size to find the
/// most common color, so that JPEG noise and faint gradients do not split
/// one background into many "different" colors.
const DETECTION_BUCKET: u8 = 16;

/// A border must be at least this uniform (share of its pixels within the
/// tolerance of the most common color) to count as one background color.
const MIN_BORDER_COVERAGE: f32 = 0.5;

/// Formats a color as "#RRGGBB".
pub fn format_hex(color: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

/// Finds the background color of an image by looking at its border: the most
/// common color along all four edges (colors that differ by less than a
/// small step count as the same, then their average is taken). Fully
/// transparent border pixels carry no color and are not counted.
///
/// Fails - with a message that names the best guess, so it can be taken over
/// with `-c` - if the border has no clearly dominant color (a gradient, a
/// photo) or is entirely transparent already.
pub fn detect_background_color(
    img: &RgbaImage,
    tolerance_percent: u8,
) -> Result<DetectedBackground, String> {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return Err("The image is empty - there is no background to detect.".to_string());
    }

    // Every border pixel once: the top and bottom rows, and the left and
    // right columns without their corners (already in the rows).
    let mut border: Vec<&Rgba<u8>> = Vec::new();
    for x in 0..width {
        border.push(img.get_pixel(x, 0));
        if height > 1 {
            border.push(img.get_pixel(x, height - 1));
        }
    }
    for y in 1..height.saturating_sub(1) {
        border.push(img.get_pixel(0, y));
        if width > 1 {
            border.push(img.get_pixel(width - 1, y));
        }
    }
    let opaque: Vec<&Rgba<u8>> = border.into_iter().filter(|p| p[3] != 0).collect();
    if opaque.is_empty() {
        return Err(
            "The border of the image is already fully transparent - there is no background color to detect."
                .to_string(),
        );
    }

    // bucket -> (pixel count, sum of the red, green and blue values).
    // A BTreeMap, so that ties between buckets always resolve the same way.
    let mut buckets: BTreeMap<[u8; 3], (usize, [u64; 3])> = BTreeMap::new();
    for pixel in &opaque {
        let key = [
            pixel[0] / DETECTION_BUCKET,
            pixel[1] / DETECTION_BUCKET,
            pixel[2] / DETECTION_BUCKET,
        ];
        let entry = buckets.entry(key).or_insert((0, [0; 3]));
        entry.0 += 1;
        for channel in 0..3 {
            entry.1[channel] += u64::from(pixel[channel]);
        }
    }
    let (count, sums) = buckets
        .values()
        .copied()
        .max_by_key(|&(count, _)| count)
        .expect("there is at least one opaque border pixel");
    let color = [
        ((sums[0] as f64 / count as f64).round()) as u8,
        ((sums[1] as f64 / count as f64).round()) as u8,
        ((sums[2] as f64 / count as f64).round()) as u8,
    ];

    let limit = squared_distance_limit(tol_distance_from_percent(tolerance_percent));
    let within = opaque
        .iter()
        .filter(|pixel| squared_color_distance(&pixel.0, color) <= limit)
        .count();
    let coverage = within as f32 / opaque.len() as f32;

    if coverage < MIN_BORDER_COVERAGE {
        return Err(format!(
            "Could not detect a single background color: the most common color along the border, {}, covers only {:.0}% of it (within a tolerance of {tolerance_percent}%). The background may be a gradient or a photo. Give the color yourself with --chroma-key #RRGGBB, or raise --tolerance.",
            format_hex(color),
            coverage * 100.0
        ));
    }
    Ok(DetectedBackground { color, coverage })
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
/// reach (flood_fill_from_candidates, with no extra seeds). Then scan every
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
    silent: bool,
) -> Vec<FoundRegion> {
    let (width, height) = img.dimensions();
    let candidates = background_candidates(img, target, tolerance_percent);

    // Anything the normal border-based flood fill already reaches doesn't
    // need to be "found" - it'll be handled anyway. We start our own
    // labeling from this, treating those pixels as already accounted for.
    let mut labeled = flood_fill_from_candidates(width, height, &candidates, &[], silent);

    let mut regions = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let start = flat_index(x, y, width);
            if labeled[start] || !candidates[start] {
                continue;
            }

            // Found the start of a new, previously-unaccounted-for region -
            // flood fill across it (same 4-connectivity as the border
            // fill), collecting every member pixel. Unlike the border
            // fill, this deliberately walks the region breadth-first, pixel
            // by pixel: the order members are collected in decides which
            // of several equally-central pixels becomes the suggested
            // seed below, and --find's output should stay stable.
            let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
            let mut members: Vec<(u32, u32)> = Vec::new();
            labeled[start] = true;
            queue.push_back((x, y));

            while let Some((cx, cy)) = queue.pop_front() {
                members.push((cx, cy));
                for (nx, ny) in in_bounds_neighbors(cx, cy, width, height)
                    .into_iter()
                    .flatten()
                {
                    let neighbor = flat_index(nx, ny, width);
                    if !labeled[neighbor] && candidates[neighbor] {
                        labeled[neighbor] = true;
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
            let (sum_x, sum_y) = members.iter().fold((0u64, 0u64), |(sx, sy), &(px, py)| {
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

    // --- Test image helpers ------------------------------------------------

    const GREEN: [u8; 3] = [0, 255, 0];
    const RED: [u8; 3] = [255, 0, 0];

    fn rgba(color: [u8; 3]) -> Rgba<u8> {
        Rgba([color[0], color[1], color[2], 255])
    }

    fn solid(width: u32, height: u32, color: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, rgba(color))
    }

    /// Fills the inclusive rectangle (x0,y0)..=(x1,y1) with `color`.
    fn fill_rect(img: &mut RgbaImage, (x0, y0): (u32, u32), (x1, y1): (u32, u32), color: [u8; 3]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                img.put_pixel(x, y, rgba(color));
            }
        }
    }

    /// A 9x9 image: green background, a red ring (2 pixels from the border)
    /// and a 3x3 green patch in the middle that the border can't reach.
    fn image_with_enclosed_patch() -> RgbaImage {
        let mut img = solid(9, 9, GREEN);
        fill_rect(&mut img, (2, 2), (6, 6), RED);
        fill_rect(&mut img, (3, 3), (5, 5), GREEN);
        img
    }

    // --- parse_hex_color -----------------------------------------------------

    #[test]
    fn parse_hex_color_accepts_with_and_without_hash() {
        assert_eq!(parse_hex_color("#00FF00"), Ok([0, 255, 0]));
        assert_eq!(parse_hex_color("00FF00"), Ok([0, 255, 0]));
    }

    #[test]
    fn parse_hex_color_is_case_insensitive() {
        assert_eq!(parse_hex_color("aBcDeF"), Ok([0xAB, 0xCD, 0xEF]));
    }

    #[test]
    fn parse_hex_color_trims_surrounding_whitespace() {
        assert_eq!(parse_hex_color("  #102030 "), Ok([0x10, 0x20, 0x30]));
    }

    #[test]
    fn parse_hex_color_rejects_wrong_length() {
        assert!(parse_hex_color("").is_err());
        assert!(parse_hex_color("#FFF").is_err());
        assert!(parse_hex_color("00FF000").is_err());
    }

    #[test]
    fn parse_hex_color_rejects_non_hex_digits() {
        assert!(parse_hex_color("GGGGGG").is_err());
        assert!(parse_hex_color("00FF0Z").is_err());
    }

    #[test]
    fn parse_hex_color_error_mentions_the_input() {
        let err = parse_hex_color("nope").unwrap_err();
        assert!(err.contains("nope"), "unexpected message: {err}");
    }

    // --- Distance and tolerance helpers --------------------------------------

    #[test]
    fn tolerance_zero_percent_means_zero_distance() {
        assert_eq!(tol_distance_from_percent(0), 0.0);
    }

    #[test]
    fn tolerance_hundred_percent_covers_the_whole_color_cube() {
        assert_eq!(tol_distance_from_percent(100), MAX_RGB_DISTANCE);
    }

    #[test]
    fn tolerance_above_hundred_is_clamped() {
        assert_eq!(tol_distance_from_percent(255), MAX_RGB_DISTANCE);
    }

    #[test]
    fn color_distance_is_zero_for_identical_colors() {
        assert_eq!(color_distance(&rgba(RED), RED), 0.0);
    }

    #[test]
    fn color_distance_black_to_white_is_the_maximum() {
        let distance = color_distance(&rgba([0, 0, 0]), [255, 255, 255]);
        assert!((distance - MAX_RGB_DISTANCE).abs() < 0.01, "got {distance}");
    }

    #[test]
    fn color_distance_ignores_alpha() {
        let translucent = Rgba([255, 0, 0, 10]);
        assert_eq!(color_distance(&translucent, RED), 0.0);
    }

    #[test]
    fn neighbors_in_the_middle_are_all_four_directions() {
        let neighbors: Vec<_> = in_bounds_neighbors(2, 2, 5, 5)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(neighbors.len(), 4);
        for expected in [(1, 2), (3, 2), (2, 1), (2, 3)] {
            assert!(neighbors.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn neighbors_at_a_corner_stay_inside_the_image() {
        let top_left: Vec<_> = in_bounds_neighbors(0, 0, 5, 5)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(top_left.len(), 2);
        let bottom_right: Vec<_> = in_bounds_neighbors(4, 4, 5, 5)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(bottom_right.len(), 2);
        assert!(bottom_right.contains(&(3, 4)) && bottom_right.contains(&(4, 3)));
    }

    #[test]
    fn neighbors_of_a_single_pixel_image_are_empty() {
        assert_eq!(
            in_bounds_neighbors(0, 0, 1, 1)
                .into_iter()
                .flatten()
                .count(),
            0
        );
    }

    // --- flood_fill_reachable ------------------------------------------------

    #[test]
    fn flood_fill_reaches_the_whole_uniform_background() {
        let img = solid(6, 4, GREEN);
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert_eq!(visited.len(), 24);
        assert!(visited.iter().all(|&v| v));
    }

    #[test]
    fn flood_fill_stops_at_a_differently_colored_subject() {
        let mut img = solid(7, 7, GREEN);
        fill_rect(&mut img, (2, 2), (4, 4), RED);
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        for y in 0..7u32 {
            for x in 0..7u32 {
                let inside_subject = (2..=4).contains(&x) && (2..=4).contains(&y);
                assert_eq!(
                    visited[(y * 7 + x) as usize],
                    !inside_subject,
                    "pixel ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn flood_fill_does_not_reach_an_enclosed_patch() {
        let visited = flood_fill_reachable(&image_with_enclosed_patch(), GREEN, 20, &[], true);
        assert!(
            !visited[4 * 9 + 4],
            "the enclosed centre must stay unreached"
        );
        assert!(visited[0], "the border must be reached");
    }

    #[test]
    fn flood_fill_enters_an_enclosed_patch_through_a_seed() {
        let visited =
            flood_fill_reachable(&image_with_enclosed_patch(), GREEN, 20, &[(4, 4)], true);
        assert!(visited[4 * 9 + 4]);
        assert!(visited[3 * 9 + 3]);
        assert!(
            !visited[2 * 9 + 2],
            "the red ring itself must stay unreached"
        );
    }

    #[test]
    fn flood_fill_ignores_seeds_outside_the_image() {
        let img = solid(4, 4, RED);
        let visited = flood_fill_reachable(&img, GREEN, 20, &[(4, 0), (0, 4), (100, 100)], true);
        assert!(visited.iter().all(|&v| !v));
    }

    #[test]
    fn flood_fill_reaches_nothing_when_no_border_pixel_matches() {
        let img = solid(5, 5, RED);
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert!(visited.iter().all(|&v| !v));
    }

    #[test]
    fn flood_fill_passes_through_already_transparent_pixels() {
        let mut img = solid(5, 1, RED);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 0]));
        img.put_pixel(1, 0, Rgba([255, 0, 0, 0]));
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert_eq!(visited, vec![true, true, false, false, false]);
    }

    #[test]
    fn flood_fill_does_not_move_diagonally() {
        // Green pixels touching only at a corner are not connected.
        let mut img = solid(3, 3, RED);
        img.put_pixel(0, 0, rgba(GREEN));
        img.put_pixel(1, 1, rgba(GREEN));
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert!(visited[0]);
        assert!(!visited[4]);
    }

    #[test]
    fn flood_fill_higher_tolerance_reaches_more() {
        let mut img = solid(5, 5, GREEN);
        fill_rect(&mut img, (1, 1), (3, 3), [0, 200, 0]); // distance 55 from GREEN
        fill_rect(&mut img, (2, 2), (2, 2), RED);
        let strict = flood_fill_reachable(&img, GREEN, 5, &[], true);
        let lenient = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert!(
            !strict[2 * 5 + 1],
            "5% (~22) must not cross the 55-distance ring"
        );
        assert!(lenient[2 * 5 + 1], "20% (~88) crosses the ring");
        assert!(!lenient[2 * 5 + 2], "but never the red centre");
    }

    // --- detect_background_color -------------------------------------------------

    #[test]
    fn detection_finds_a_solid_border_color() {
        let mut img = solid(8, 8, GREEN);
        fill_rect(&mut img, (2, 2), (5, 5), RED);
        let found = detect_background_color(&img, 20).unwrap();
        assert_eq!(found.color, GREEN);
        assert_eq!(found.coverage, 1.0);
    }

    #[test]
    fn detection_ignores_what_is_inside_the_border() {
        // A subject that is MORE common than the background, but not on the border.
        let mut img = solid(20, 20, GREEN);
        fill_rect(&mut img, (1, 1), (18, 18), RED);
        assert_eq!(detect_background_color(&img, 20).unwrap().color, GREEN);
    }

    #[test]
    fn detection_sees_through_noise_and_averages_it() {
        // A "JPEG-like" background: green with small deviations.
        let mut img = solid(16, 16, GREEN);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let jitter = ((x * 7 + y * 13) % 5) as u8; // 0..=4
            *pixel = Rgba([jitter, 255 - jitter, jitter, 255]);
        }
        let found = detect_background_color(&img, 20).unwrap();
        let [r, g, b] = found.color;
        assert!(r <= 4 && g >= 251 && b <= 4, "{:?}", found.color);
        assert_eq!(found.coverage, 1.0);
    }

    #[test]
    fn detection_takes_the_most_common_color_when_two_share_the_border() {
        // Three quarters of the border white, one quarter black.
        let mut img = solid(16, 16, [255, 255, 255]);
        for x in 0..16 {
            img.put_pixel(x, 0, rgba([0, 0, 0]));
        }
        let found = detect_background_color(&img, 20).unwrap();
        assert_eq!(found.color, [255, 255, 255]);
        assert!(
            found.coverage > 0.5 && found.coverage < 1.0,
            "{}",
            found.coverage
        );
    }

    #[test]
    fn a_gradient_border_is_reported_with_the_best_guess() {
        let mut img = RgbaImage::new(64, 64);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([(x * 4) as u8, (x * 4) as u8, 255 - (x * 4) as u8, 255]);
        }
        let err = detect_background_color(&img, 5).unwrap_err();
        assert!(
            err.contains("Could not detect a single background color"),
            "{err}"
        );
        assert!(err.contains("#"), "the guess is named: {err}");
        assert!(
            err.contains("--chroma-key #RRGGBB") && err.contains("--tolerance"),
            "{err}"
        );
        // A generous tolerance accepts the same border as one background.
        assert!(detect_background_color(&img, 100).is_ok());
    }

    #[test]
    fn a_transparent_border_has_no_color_to_detect() {
        let mut img = RgbaImage::new(6, 6); // all fully transparent
        img.put_pixel(3, 3, rgba(RED));
        let err = detect_background_color(&img, 20).unwrap_err();
        assert!(err.contains("already fully transparent"), "{err}");
    }

    #[test]
    fn transparent_border_pixels_are_not_counted_as_a_color() {
        let mut img = solid(10, 10, GREEN);
        for x in 0..10 {
            img.put_pixel(x, 0, Rgba([0, 0, 0, 0])); // the top row is already see-through
        }
        let found = detect_background_color(&img, 20).unwrap();
        assert_eq!(found.color, GREEN);
        assert_eq!(
            found.coverage, 1.0,
            "judged among the opaque border pixels only"
        );
    }

    #[test]
    fn detection_works_on_tiny_images() {
        assert_eq!(
            detect_background_color(&solid(1, 1, RED), 20)
                .unwrap()
                .color,
            RED
        );
        assert_eq!(
            detect_background_color(&solid(1, 5, RED), 20)
                .unwrap()
                .color,
            RED
        );
        assert_eq!(
            detect_background_color(&solid(5, 1, RED), 20)
                .unwrap()
                .color,
            RED
        );
        assert!(detect_background_color(&RgbaImage::new(0, 0), 20).is_err());
    }

    #[test]
    fn hex_formatting_is_uppercase_with_a_hash() {
        assert_eq!(format_hex([0, 255, 0]), "#00FF00");
        assert_eq!(format_hex([1, 2, 171]), "#0102AB");
    }

    // --- feather -----------------------------------------------------------------

    /// A green image whose pixel (1, 4) is an "edge" shade at distance 130
    /// from the green, reachable from the border.
    fn image_with_an_edge_pixel() -> RgbaImage {
        let mut img = solid(9, 9, GREEN);
        img.put_pixel(1, 4, rgba([0, 255, 130]));
        img
    }

    fn edge_alpha(feather: u8) -> u8 {
        let mut img = image_with_an_edge_pixel();
        apply_chroma_key_feathered(&mut img, GREEN, 40, feather, &[], None, true);
        img.get_pixel(1, 4)[3]
    }

    #[test]
    fn a_feather_of_zero_makes_a_hard_edge() {
        assert_eq!(
            edge_alpha(0),
            0,
            "everything within the tolerance is removed fully"
        );
    }

    #[test]
    fn a_wider_feather_keeps_more_of_the_edge() {
        let (hard, default, wide) = (edge_alpha(0), edge_alpha(DEFAULT_FEATHER), edge_alpha(100));
        assert!(
            hard < default && default < wide,
            "{hard} < {default} < {wide}"
        );
        assert!(wide < 255, "still partly transparent");
    }

    #[test]
    fn the_default_feather_is_the_behavior_from_before() {
        let mut with_default = image_with_an_edge_pixel();
        let mut old_style = image_with_an_edge_pixel();
        apply_chroma_key_feathered(
            &mut with_default,
            GREEN,
            40,
            DEFAULT_FEATHER,
            &[],
            None,
            true,
        );
        apply_chroma_key(&mut old_style, GREEN, 40, &[], None, true);
        assert_eq!(with_default, old_style);
        assert_eq!(DEFAULT_FEATHER, 50);
    }

    #[test]
    fn the_feather_never_changes_which_pixels_are_background() {
        // Only the edge softness differs; a pixel outside the tolerance stays.
        for feather in [0, 50, 100] {
            let mut img = solid(7, 7, GREEN);
            fill_rect(&mut img, (2, 2), (4, 4), RED);
            apply_chroma_key_feathered(&mut img, GREEN, 20, feather, &[], None, true);
            assert_eq!(img.get_pixel(0, 0)[3], 0, "feather {feather}");
            assert_eq!(*img.get_pixel(3, 3), rgba(RED), "feather {feather}");
        }
    }

    #[test]
    fn a_feather_above_100_counts_as_100() {
        assert_eq!(edge_alpha(200), edge_alpha(100));
    }

    // --- how much a chroma key affected --------------------------------------------

    #[test]
    fn the_key_reports_how_many_pixels_it_affected() {
        let mut img = solid(7, 7, GREEN);
        fill_rect(&mut img, (2, 2), (4, 4), RED);
        let affected = apply_chroma_key_feathered(&mut img, GREEN, 20, 50, &[], None, true);
        assert_eq!(affected, 49 - 9);
        // Already transparent pixels are not counted again.
        let again = apply_chroma_key_feathered(&mut img, GREEN, 20, 50, &[], None, true);
        assert_eq!(again, 0);
    }

    fn warnings_raised(affected: usize, total: usize) -> usize {
        let before = crate::util::warnings_so_far();
        warn_about_removal_extent(affected, total, "#00FF00", true);
        crate::util::warnings_so_far() - before
    }

    #[test]
    fn removing_nothing_or_almost_nothing_or_almost_everything_is_a_warning() {
        assert_eq!(warnings_raised(0, 1000), 1, "nothing");
        assert_eq!(warnings_raised(3, 1000), 1, "almost nothing (0.3%)");
        assert_eq!(warnings_raised(995, 1000), 1, "almost everything (99.5%)");
        assert_eq!(warnings_raised(1000, 1000), 1, "everything");
    }

    #[test]
    fn an_ordinary_removal_is_not_a_warning() {
        assert_eq!(warnings_raised(500, 1000), 0);
        assert_eq!(warnings_raised(10, 1000), 0, "1% is above the limit");
        assert_eq!(warnings_raised(970, 1000), 0, "97% is below the limit");
        assert_eq!(warnings_raised(0, 0), 0, "no pixels, nothing to say");
    }

    // --- apply_chroma_key ------------------------------------------------------

    #[test]
    fn chroma_key_makes_the_background_transparent_and_keeps_the_subject() {
        let mut img = solid(7, 7, GREEN);
        fill_rect(&mut img, (2, 2), (4, 4), RED);
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(6, 6)[3], 0);
        assert_eq!(*img.get_pixel(3, 3), rgba(RED));
    }

    #[test]
    fn chroma_key_leaves_an_enclosed_patch_alone_without_a_seed() {
        let mut img = image_with_enclosed_patch();
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(4, 4)[3], 255);
    }

    #[test]
    fn chroma_key_removes_an_enclosed_patch_with_a_seed() {
        let mut img = image_with_enclosed_patch();
        apply_chroma_key(&mut img, GREEN, 20, &[(4, 4)], None, true);
        assert_eq!(img.get_pixel(4, 4)[3], 0);
        assert_eq!(img.get_pixel(2, 2)[3], 255, "the ring stays");
    }

    #[test]
    fn chroma_key_replacement_recolors_the_background_and_keeps_alpha() {
        let mut img = solid(7, 7, GREEN);
        fill_rect(&mut img, (2, 2), (4, 4), RED);
        apply_chroma_key(&mut img, GREEN, 20, &[], Some([0, 0, 255]), true);
        assert_eq!(*img.get_pixel(0, 0), Rgba([0, 0, 255, 255]));
        assert_eq!(*img.get_pixel(3, 3), rgba(RED));
    }

    #[test]
    fn chroma_key_tolerance_zero_only_matches_the_exact_color() {
        let mut img = solid(4, 4, GREEN);
        img.put_pixel(0, 0, rgba([0, 250, 0]));
        apply_chroma_key(&mut img, GREEN, 0, &[], None, true);
        assert_eq!(img.get_pixel(1, 1)[3], 0, "exact match is removed");
        assert_eq!(
            img.get_pixel(0, 0)[3],
            255,
            "a slightly different shade survives"
        );
    }

    #[test]
    fn chroma_key_core_zone_is_fully_removed_despite_small_deviations() {
        let mut img = solid(4, 4, GREEN);
        img.put_pixel(0, 0, rgba([0, 235, 0])); // distance 20, inside the core zone at 20%
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn chroma_key_feather_zone_gives_partial_transparency() {
        let mut img = solid(4, 4, GREEN);
        img.put_pixel(0, 0, rgba([0, 189, 0])); // distance 66, between core (44) and limit (88)
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        let alpha = img.get_pixel(0, 0)[3];
        assert!(
            alpha > 0 && alpha < 255,
            "expected a partial alpha, got {alpha}"
        );
    }

    #[test]
    fn chroma_key_feather_is_monotonic_further_from_target_means_more_opaque() {
        let mut img = solid(4, 1, GREEN);
        img.put_pixel(0, 0, rgba([0, 205, 0])); // distance 50
        img.put_pixel(1, 0, rgba([0, 185, 0])); // distance 70
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert!(img.get_pixel(0, 0)[3] < img.get_pixel(1, 0)[3]);
    }

    #[test]
    fn chroma_key_does_not_touch_already_transparent_pixels() {
        let mut img = solid(3, 3, GREEN);
        img.put_pixel(1, 1, Rgba([12, 34, 56, 0]));
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(*img.get_pixel(1, 1), Rgba([12, 34, 56, 0]));
    }

    #[test]
    fn chroma_key_on_a_single_pixel_image_does_not_panic() {
        let mut img = solid(1, 1, GREEN);
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn chroma_key_without_any_match_changes_nothing() {
        let mut img = solid(5, 5, RED);
        let before = img.clone();
        apply_chroma_key(&mut img, GREEN, 20, &[], None, true);
        assert_eq!(img, before);
    }

    // --- find_isolated_regions -------------------------------------------------

    #[test]
    fn find_reports_nothing_when_the_border_reaches_everything() {
        let img = solid(8, 8, GREEN);
        assert!(find_isolated_regions(&img, GREEN, 20, 1, true).is_empty());
    }

    #[test]
    fn find_discovers_an_enclosed_patch() {
        let regions = find_isolated_regions(
            &image_with_enclosed_patch(),
            GREEN,
            20,
            DEFAULT_FIND_MIN_SIZE,
            true,
        );
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].pixel_count, 9);
        assert_eq!(regions[0].seed, (4, 4));
    }

    #[test]
    fn find_ignores_regions_smaller_than_the_minimum_size() {
        let img = image_with_enclosed_patch(); // the patch has exactly 9 pixels
        assert_eq!(find_isolated_regions(&img, GREEN, 20, 9, true).len(), 1);
        assert!(find_isolated_regions(&img, GREEN, 20, 10, true).is_empty());
    }

    #[test]
    fn find_with_minimum_one_catches_a_single_pixel() {
        let mut img = solid(7, 7, RED);
        img.put_pixel(3, 3, rgba(GREEN));
        let regions = find_isolated_regions(&img, GREEN, 20, 1, true);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].pixel_count, 1);
        assert_eq!(regions[0].seed, (3, 3));
    }

    #[test]
    fn find_reports_separate_regions_separately() {
        let mut img = solid(12, 6, RED);
        fill_rect(&mut img, (1, 1), (2, 2), GREEN);
        fill_rect(&mut img, (8, 2), (10, 4), GREEN);
        let mut regions = find_isolated_regions(&img, GREEN, 20, 1, true);
        regions.sort_by_key(|r| r.pixel_count);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].pixel_count, 4);
        assert_eq!(regions[1].pixel_count, 9);
    }

    #[test]
    fn find_suggests_a_seed_that_lies_inside_a_concave_region() {
        // An L-shape: its centroid falls outside the shape itself.
        let mut img = solid(14, 14, RED);
        fill_rect(&mut img, (2, 2), (3, 11), GREEN);
        fill_rect(&mut img, (4, 10), (11, 11), GREEN);
        let regions = find_isolated_regions(&img, GREEN, 20, 1, true);
        assert_eq!(regions.len(), 1);
        let (x, y) = regions[0].seed;
        assert_eq!(
            *img.get_pixel(x, y),
            rgba(GREEN),
            "seed ({x},{y}) is not part of the region"
        );
    }

    #[test]
    fn find_does_not_report_what_the_border_fill_already_reaches() {
        let img = image_with_enclosed_patch();
        // The green background around the ring is border-reachable and must
        // not show up; only the enclosed patch does.
        let regions = find_isolated_regions(&img, GREEN, 20, 1, true);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].pixel_count, 9);
    }

    #[test]
    fn default_find_min_size_is_nine() {
        assert_eq!(DEFAULT_FIND_MIN_SIZE, 9);
    }

    // --- Equivalence with a naive reference implementation -----------------------
    //
    // `reference` is the plain, unoptimized version of the three algorithms
    // above, written for obviousness rather than speed (pixel by pixel,
    // with a real square root for every comparison). The production code is
    // free to be structured for speed, but it must give EXACTLY the same
    // answer as this for every input - these property tests feed both
    // random images, colors, tolerances and seeds and compare the results.
    mod reference {
        use crate::chroma_key::MAX_RGB_DISTANCE;
        use image::{Rgba, RgbaImage};
        use std::collections::VecDeque;

        fn distance(pixel: &Rgba<u8>, target: [u8; 3]) -> f32 {
            let dr = pixel[0] as f32 - target[0] as f32;
            let dg = pixel[1] as f32 - target[1] as f32;
            let db = pixel[2] as f32 - target[2] as f32;
            (dr * dr + dg * dg + db * db).sqrt()
        }

        fn tolerance_distance(percent: u8) -> f32 {
            MAX_RGB_DISTANCE * (percent.min(100) as f32 / 100.0)
        }

        fn neighbors(x: u32, y: u32, w: u32, h: u32) -> Vec<(u32, u32)> {
            let mut out = Vec::new();
            if x > 0 {
                out.push((x - 1, y));
            }
            if x + 1 < w {
                out.push((x + 1, y));
            }
            if y > 0 {
                out.push((x, y - 1));
            }
            if y + 1 < h {
                out.push((x, y + 1));
            }
            out
        }

        pub fn flood_fill(
            img: &RgbaImage,
            target: [u8; 3],
            percent: u8,
            seeds: &[(u32, u32)],
        ) -> Vec<bool> {
            let (w, h) = img.dimensions();
            let tol = tolerance_distance(percent);
            let candidate = |x: u32, y: u32| {
                let p = img.get_pixel(x, y);
                p[3] == 0 || distance(p, target) <= tol
            };
            let mut visited = vec![false; (w * h) as usize];
            let mut queue = VecDeque::new();
            for y in 0..h {
                for x in 0..w {
                    let on_border = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                    if on_border && candidate(x, y) {
                        visited[(y * w + x) as usize] = true;
                        queue.push_back((x, y));
                    }
                }
            }
            for &(x, y) in seeds {
                if x < w && y < h && !visited[(y * w + x) as usize] {
                    visited[(y * w + x) as usize] = true;
                    queue.push_back((x, y));
                }
            }
            while let Some((x, y)) = queue.pop_front() {
                for (nx, ny) in neighbors(x, y, w, h) {
                    if !visited[(ny * w + nx) as usize] && candidate(nx, ny) {
                        visited[(ny * w + nx) as usize] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            visited
        }

        pub fn apply(
            img: &mut RgbaImage,
            target: [u8; 3],
            percent: u8,
            seeds: &[(u32, u32)],
            replacement: Option<[u8; 3]>,
        ) {
            let (w, _) = img.dimensions();
            let visited = flood_fill(img, target, percent, seeds);
            let tol = tolerance_distance(percent);
            let core = tol * 0.5;
            for (x, y, pixel) in img.enumerate_pixels_mut() {
                if !visited[(y * w + x) as usize] || pixel[3] == 0 {
                    continue;
                }
                let d = distance(pixel, target);
                let scale = if d <= core {
                    0.0
                } else if tol > core {
                    ((d - core) / (tol - core)).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                match replacement {
                    None => pixel[3] = (pixel[3] as f32 * scale).round() as u8,
                    Some(new) => {
                        for c in 0..3 {
                            pixel[c] = (pixel[c] as f32 * scale + new[c] as f32 * (1.0 - scale))
                                .round()
                                .clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }

        /// Returns `(pixel_count, seed)` for every region, in discovery order.
        pub fn find(
            img: &RgbaImage,
            target: [u8; 3],
            percent: u8,
            min_size: usize,
        ) -> Vec<(usize, (u32, u32))> {
            let (w, h) = img.dimensions();
            let tol = tolerance_distance(percent);
            let candidate = |x: u32, y: u32| {
                let p = img.get_pixel(x, y);
                p[3] == 0 || distance(p, target) <= tol
            };
            let mut labeled = flood_fill(img, target, percent, &[]);
            let mut regions = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if labeled[(y * w + x) as usize] || !candidate(x, y) {
                        continue;
                    }
                    let mut queue = VecDeque::from([(x, y)]);
                    let mut members = Vec::new();
                    labeled[(y * w + x) as usize] = true;
                    while let Some((cx, cy)) = queue.pop_front() {
                        members.push((cx, cy));
                        for (nx, ny) in neighbors(cx, cy, w, h) {
                            if !labeled[(ny * w + nx) as usize] && candidate(nx, ny) {
                                labeled[(ny * w + nx) as usize] = true;
                                queue.push_back((nx, ny));
                            }
                        }
                    }
                    if members.len() < min_size {
                        continue;
                    }
                    let n = members.len() as u64;
                    let cx = (members.iter().map(|m| m.0 as u64).sum::<u64>() / n) as i64;
                    let cy = (members.iter().map(|m| m.1 as u64).sum::<u64>() / n) as i64;
                    let seed = *members
                        .iter()
                        .min_by_key(|m| (m.0 as i64 - cx).pow(2) + (m.1 as i64 - cy).pow(2))
                        .unwrap();
                    regions.push((members.len(), seed));
                }
            }
            regions
        }
    }

    /// A pixel drawn mostly from colors that matter for chroma keying
    /// (the target, near-misses of it, other colors, transparent and
    /// translucent variants), sometimes from anywhere in the RGBA space.
    fn arbitrary_pixel() -> impl Strategy<Value = [u8; 4]> {
        prop_oneof![
            3 => proptest::sample::select(vec![
                [0, 255, 0, 255],
                [255, 0, 0, 255],
                [0, 235, 0, 255],
                [0, 215, 0, 255],
                [0, 189, 0, 255],
                [10, 250, 10, 255],
                [0, 255, 0, 0],
                [0, 255, 0, 128],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
            ]),
            1 => any::<[u8; 4]>(),
        ]
    }

    fn arbitrary_image() -> impl Strategy<Value = RgbaImage> {
        (1u32..=12, 1u32..=12).prop_flat_map(|(w, h)| {
            proptest::collection::vec(arbitrary_pixel(), (w * h) as usize).prop_map(move |px| {
                let raw: Vec<u8> = px.into_iter().flatten().collect();
                RgbaImage::from_raw(w, h, raw).unwrap()
            })
        })
    }

    fn arbitrary_target() -> impl Strategy<Value = [u8; 3]> {
        prop_oneof![4 => Just(GREEN), 1 => any::<[u8; 3]>()]
    }

    /// Mostly the meaningful 0-100 range, sometimes any u8 (values above
    /// 100 are clamped to 100).
    fn arbitrary_tolerance() -> impl Strategy<Value = u8> {
        prop_oneof![5 => 0u8..=100, 1 => any::<u8>()]
    }

    fn arbitrary_seeds() -> impl Strategy<Value = Vec<(u32, u32)>> {
        proptest::collection::vec((0u32..16, 0u32..16), 0..=3)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        #[test]
        fn flood_fill_matches_the_reference(
            img in arbitrary_image(),
            target in arbitrary_target(),
            tolerance in arbitrary_tolerance(),
            seeds in arbitrary_seeds(),
        ) {
            prop_assert_eq!(
                flood_fill_reachable(&img, target, tolerance, &seeds, true),
                reference::flood_fill(&img, target, tolerance, &seeds)
            );
        }

        #[test]
        fn apply_chroma_key_matches_the_reference(
            img in arbitrary_image(),
            target in arbitrary_target(),
            tolerance in arbitrary_tolerance(),
            seeds in arbitrary_seeds(),
            replacement in proptest::option::of(any::<[u8; 3]>()),
        ) {
            let mut optimized = img.clone();
            let mut expected = img;
            apply_chroma_key(&mut optimized, target, tolerance, &seeds, replacement, true);
            reference::apply(&mut expected, target, tolerance, &seeds, replacement);
            prop_assert_eq!(optimized, expected);
        }

        #[test]
        fn find_isolated_regions_matches_the_reference(
            img in arbitrary_image(),
            target in arbitrary_target(),
            tolerance in arbitrary_tolerance(),
            min_size in 1usize..=6,
        ) {
            let found: Vec<(usize, (u32, u32))> =
                find_isolated_regions(&img, target, tolerance, min_size, true)
                    .into_iter()
                    .map(|r| (r.pixel_count, r.seed))
                    .collect();
            prop_assert_eq!(found, reference::find(&img, target, tolerance, min_size));
        }
    }

    // --- Whole-number distance comparison ---------------------------------------

    #[test]
    fn squared_limit_agrees_with_the_real_distance_for_every_possible_distance() {
        // Every tolerance the program can produce (0-100%), against every
        // squared distance that can occur between two RGB colors.
        const MAX_SQUARED: u32 = 3 * 255 * 255;
        for percent in 0..=100u8 {
            let max_distance = tol_distance_from_percent(percent);
            let limit = squared_distance_limit(max_distance);
            for squared in 0..=MAX_SQUARED {
                let by_sqrt = (squared as f32).sqrt() <= max_distance;
                assert_eq!(
                    squared <= limit,
                    by_sqrt,
                    "{percent}% at squared distance {squared}"
                );
            }
        }
    }

    #[test]
    fn squared_limit_is_right_for_the_core_zone_thresholds_too() {
        const MAX_SQUARED: u32 = 3 * 255 * 255;
        for percent in 0..=100u8 {
            let core_distance = tol_distance_from_percent(percent) * 0.5;
            let limit = squared_distance_limit(core_distance);
            for squared in (0..=MAX_SQUARED).step_by(7) {
                let by_sqrt = (squared as f32).sqrt() <= core_distance;
                assert_eq!(
                    squared <= limit,
                    by_sqrt,
                    "{percent}% core, squared {squared}"
                );
            }
        }
    }

    #[test]
    fn squared_color_distance_matches_the_real_distance() {
        let a = [12u8, 200, 99, 255];
        let target = [250u8, 3, 99];
        let real = color_distance(&Rgba(a), target);
        assert_eq!((squared_color_distance(&a, target) as f32).sqrt(), real);
        assert_eq!(
            squared_color_distance(&[0, 0, 0, 0], [255, 255, 255]),
            195_075
        );
        assert_eq!(squared_color_distance(&[7, 8, 9, 0], [7, 8, 9]), 0);
    }

    #[test]
    fn candidate_mask_marks_matching_and_transparent_pixels() {
        let mut img = solid(3, 1, RED);
        img.put_pixel(0, 0, rgba(GREEN));
        img.put_pixel(1, 0, Rgba([255, 0, 0, 0])); // transparent, wrong color
        let mask = background_candidates(&img, GREEN, 20);
        assert_eq!(mask, vec![true, true, false]);
    }

    #[test]
    fn flood_fill_of_an_empty_image_does_not_panic() {
        let img = RgbaImage::new(0, 0);
        assert!(flood_fill_reachable(&img, GREEN, 20, &[(0, 0)], true).is_empty());
        assert!(find_isolated_regions(&img, GREEN, 20, 1, true).is_empty());
    }

    #[test]
    fn flood_fill_handles_single_row_and_single_column_images() {
        for (w, h) in [(9, 1), (1, 9)] {
            let img = solid(w, h, GREEN);
            assert!(
                flood_fill_reachable(&img, GREEN, 20, &[], true)
                    .iter()
                    .all(|&v| v)
            );
        }
    }

    #[test]
    fn flood_fill_follows_a_winding_corridor() {
        // A 1-pixel-wide serpentine path of green through red walls: the
        // fill has to turn around many times to get from one end to the
        // other.
        let mut img = solid(9, 9, RED);
        for y in (0..9).step_by(2) {
            fill_rect(&mut img, (0, y), (8, y), GREEN);
        }
        for (i, y) in (1..9).step_by(2).enumerate() {
            let x = if i % 2 == 0 { 8 } else { 0 };
            img.put_pixel(x, y, rgba(GREEN));
        }
        let visited = flood_fill_reachable(&img, GREEN, 20, &[], true);
        assert_eq!(visited, reference::flood_fill(&img, GREEN, 20, &[]));
        assert!(visited[8 * 9 + 8], "the far end of the corridor is reached");
    }

    /// Larger images with mostly background-colored pixels, so there are
    /// long runs and irregular shapes that stress the row-by-row fill
    /// (the small images above are dominated by the border).
    fn arbitrary_large_image() -> impl Strategy<Value = RgbaImage> {
        (20u32..=60, 20u32..=60).prop_flat_map(|(w, h)| {
            let pixel = prop_oneof![
                8 => Just([0u8, 255, 0, 255]),
                2 => Just([255u8, 0, 0, 255]),
                1 => arbitrary_pixel(),
            ];
            proptest::collection::vec(pixel, (w * h) as usize).prop_map(move |px| {
                let raw: Vec<u8> = px.into_iter().flatten().collect();
                RgbaImage::from_raw(w, h, raw).unwrap()
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]

        #[test]
        fn large_images_match_the_reference(
            img in arbitrary_large_image(),
            tolerance in arbitrary_tolerance(),
            seeds in proptest::collection::vec((0u32..70, 0u32..70), 0..=4),
            replacement in proptest::option::of(any::<[u8; 3]>()),
        ) {
            prop_assert_eq!(
                flood_fill_reachable(&img, GREEN, tolerance, &seeds, true),
                reference::flood_fill(&img, GREEN, tolerance, &seeds)
            );

            let mut optimized = img.clone();
            let mut expected = img.clone();
            apply_chroma_key(&mut optimized, GREEN, tolerance, &seeds, replacement, true);
            reference::apply(&mut expected, GREEN, tolerance, &seeds, replacement);
            prop_assert_eq!(optimized, expected);

            let found: Vec<(usize, (u32, u32))> =
                find_isolated_regions(&img, GREEN, tolerance, 1, true)
                    .into_iter()
                    .map(|r| (r.pixel_count, r.seed))
                    .collect();
            prop_assert_eq!(found, reference::find(&img, GREEN, tolerance, 1));
        }
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
