//! Turning a picture: by quarters exactly, by any whole number of degrees with a canvas that grows
//! to hold the turned picture (the corners that are left are transparent). Positive is clockwise.
//!
//! The browser page shows the turned picture in the crop view from this same code, so what the
//! frame is placed on is what the icon is made from.

use crate::layout::{CropRect, check_crop};
use crate::source::DEFAULT_MAX_PIXELS;
use image::{Rgba, RgbaImage, imageops};

/// The picture mirrored: left to right (`horizontal`) and/or top to bottom (`vertical`). Done before the turn, so
/// the picture is mirrored as it lies and then turned.
pub fn flip(img: &mut RgbaImage, horizontal: bool, vertical: bool) {
    if horizontal {
        imageops::flip_horizontal_in_place(img);
    }
    if vertical {
        imageops::flip_vertical_in_place(img);
    }
}

/// The turn as 0 to 359 degrees (-90 is 270).
pub fn normalize(degrees: i32) -> i32 {
    degrees.rem_euclid(360)
}

/// The size of the canvas a `width` x `height` picture turned by `degrees` needs: the box around
/// it. A quarter turn swaps the sides; any other turn grows both.
pub fn rotated_size(width: u32, height: u32, degrees: i32) -> (u32, u32) {
    match normalize(degrees) {
        0 | 180 => (width, height),
        90 | 270 => (height, width),
        turn => {
            let (sin, cos) = f64::from(turn).to_radians().sin_cos();
            let (w, h) = (f64::from(width), f64::from(height));
            // A hair is taken off before rounding up, so 100.0000000001 stays 100.
            let across =
                |a: f64, b: f64| ((a * cos.abs() + b * sin.abs()) - 1e-6).ceil().max(1.0) as u32;
            (across(w, h), across(h, w))
        }
    }
}

/// The picture turned clockwise by `degrees` (any whole number; negative turns the other way).
pub fn rotate(img: &RgbaImage, degrees: i32) -> Result<RgbaImage, String> {
    rotate_within(img, degrees, DEFAULT_MAX_PIXELS)
}

/// `rotate`, refusing a canvas of more than `max_pixels`.
pub fn rotate_within(img: &RgbaImage, degrees: i32, max_pixels: u64) -> Result<RgbaImage, String> {
    let turn = normalize(degrees);
    match turn {
        0 => return Ok(img.clone()),
        90 => return Ok(imageops::rotate90(img)),
        180 => return Ok(imageops::rotate180(img)),
        270 => return Ok(imageops::rotate270(img)),
        _ => {}
    }
    let (canvas_w, canvas_h) = canvas_for(img, degrees, max_pixels)?;
    Ok(turn_part(
        img,
        turn,
        (canvas_w, canvas_h),
        CropRect {
            x: 0,
            y: 0,
            width: canvas_w,
            height: canvas_h,
        },
    ))
}

/// Whether the turn is by a free angle: the quarters (and no turn) are done exactly and fast, any other angle
/// is made pixel by pixel.
pub fn is_free_angle(degrees: i32) -> bool {
    !matches!(normalize(degrees), 0 | 90 | 180 | 270)
}

/// The same as `rotate` followed by `crop` of `rect` (a frame on the turned picture), but only the pixels of the
/// frame are made: a picture of many megapixels turned by a free angle is then not made as a whole just to
/// throw most of it away. The result is the same, pixel for pixel. For a free angle only; the quarters are
/// turned whole, which is fast.
pub fn rotate_then_crop(
    img: &RgbaImage,
    degrees: i32,
    rect: CropRect,
    max_pixels: u64,
) -> Result<RgbaImage, String> {
    debug_assert!(is_free_angle(degrees));
    let canvas = canvas_for(img, degrees, max_pixels)?;
    check_crop(canvas, rect)?;
    Ok(turn_part(img, normalize(degrees), canvas, rect))
}

/// The canvas a turned picture needs, refused if it is bigger than `max_pixels`.
fn canvas_for(img: &RgbaImage, degrees: i32, max_pixels: u64) -> Result<(u32, u32), String> {
    let (width, height) = img.dimensions();
    let (canvas_w, canvas_h) = rotated_size(width, height, degrees);
    if u64::from(canvas_w) * u64::from(canvas_h) > max_pixels {
        return Err(crate::msg!(
            "rotate.too_big",
            "Turned by {degrees} degrees the picture would need a canvas of {width}x{height} pixels, which is too big. Turn it by a quarter, or use a smaller picture.",
            degrees = degrees,
            width = canvas_w,
            height = canvas_h
        ));
    }
    Ok((canvas_w, canvas_h))
}

/// The pixels of `part` of the canvas (of the size `canvas`) the picture turned by `turn` (a free angle,
/// 1 to 359) makes.
fn turn_part(img: &RgbaImage, turn: i32, canvas: (u32, u32), part: CropRect) -> RgbaImage {
    let (width, height) = img.dimensions();
    let (sin, cos) = f64::from(turn).to_radians().sin_cos();
    let (canvas_cx, canvas_cy) = (f64::from(canvas.0) / 2.0, f64::from(canvas.1) / 2.0);
    let (source_cx, source_cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
    let mut out = RgbaImage::new(part.width, part.height);
    for y in 0..part.height {
        for x in 0..part.width {
            // Where this pixel's centre comes from in the picture (the turn undone).
            let dx = f64::from(part.x + x) + 0.5 - canvas_cx;
            let dy = f64::from(part.y + y) + 0.5 - canvas_cy;
            let u = dx * cos + dy * sin + source_cx;
            let v = -dx * sin + dy * cos + source_cy;
            out.put_pixel(x, y, sample(img, u, v));
        }
    }
    out
}

/// The colour at the point (`u`, `v`) of the picture (pixel centres are at .5), mixed from the four
/// pixels around it with their transparency counted in, and nothing outside the picture, so its
/// edge is soft rather than jagged.
fn sample(img: &RgbaImage, u: f64, v: f64) -> Rgba<u8> {
    let (width, height) = img.dimensions();
    let (fu, fv) = (u - 0.5, v - 0.5);
    let (x0, y0) = (fu.floor(), fv.floor());
    if x0 < -1.0 || y0 < -1.0 || x0 >= f64::from(width) || y0 >= f64::from(height) {
        return Rgba([0, 0, 0, 0]);
    }
    let (tx, ty) = (fu - x0, fv - y0);
    let (mut r, mut g, mut b, mut a) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (dy, wy) in [(0.0, 1.0 - ty), (1.0, ty)] {
        for (dx, wx) in [(0.0, 1.0 - tx), (1.0, tx)] {
            let (px, py) = (x0 + dx, y0 + dy);
            if px < 0.0 || py < 0.0 || px >= f64::from(width) || py >= f64::from(height) {
                continue;
            }
            let pixel = img.get_pixel(px as u32, py as u32).0;
            let weight = wx * wy * f64::from(pixel[3]) / 255.0;
            r += weight * f64::from(pixel[0]);
            g += weight * f64::from(pixel[1]);
            b += weight * f64::from(pixel[2]);
            a += weight;
        }
    }
    if a <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let channel = |sum: f64| (sum / a).round().clamp(0.0, 255.0) as u8;
    Rgba([
        channel(r),
        channel(g),
        channel(b),
        (a * 255.0).round().clamp(0.0, 255.0) as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(color))
    }

    #[test]
    fn mirroring_swaps_left_and_right_or_top_and_bottom() {
        let mut img = RgbaImage::new(3, 2);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([x as u8, y as u8, 0, 255]);
        }
        let mut horizontal = img.clone();
        flip(&mut horizontal, true, false);
        assert_eq!(horizontal.get_pixel(0, 0).0, [2, 0, 0, 255]);
        assert_eq!(horizontal.get_pixel(2, 1).0, [0, 1, 0, 255]);
        let mut vertical = img.clone();
        flip(&mut vertical, false, true);
        assert_eq!(vertical.get_pixel(0, 0).0, [0, 1, 0, 255]);
        let mut both = img.clone();
        flip(&mut both, true, true);
        assert_eq!(both.get_pixel(0, 0).0, [2, 1, 0, 255]);
        // Twice is nothing; nothing asked is nothing.
        flip(&mut both, true, true);
        assert_eq!(both, img);
        let mut same = img.clone();
        flip(&mut same, false, false);
        assert_eq!(same, img);
    }

    #[test]
    fn the_turn_is_taken_as_0_to_359_degrees() {
        assert_eq!(normalize(-90), 270);
        assert_eq!(normalize(360), 0);
        assert_eq!(normalize(450), 90);
        assert_eq!(normalize(-1), 359);
    }

    #[test]
    fn the_canvas_swaps_for_quarters_and_grows_for_the_rest() {
        assert_eq!(rotated_size(200, 100, 0), (200, 100));
        assert_eq!(rotated_size(200, 100, 180), (200, 100));
        assert_eq!(rotated_size(200, 100, 90), (100, 200));
        assert_eq!(rotated_size(200, 100, -90), (100, 200));
        // 45 degrees: 100 * cos 45 + 100 * sin 45 = 141.42, rounded up.
        assert_eq!(rotated_size(100, 100, 45), (142, 142));
        assert_eq!(rotated_size(100, 100, 1), (102, 102));
        // The same numbers the page works with (see rotatedSize in the page's crop.ts).
        assert_eq!(rotated_size(7000, 5000, 30), (8563, 7831));
    }

    #[test]
    fn a_quarter_turn_goes_clockwise() {
        let mut img = solid(2, 1, [0, 0, 0, 255]);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        let turned = rotate(&img, 90).unwrap();
        assert_eq!(turned.dimensions(), (1, 2));
        // The left pixel is now at the top.
        assert_eq!(turned.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(
            rotate(&img, -90).unwrap().get_pixel(0, 1).0,
            [255, 0, 0, 255]
        );
        assert_eq!(rotate(&img, 270).unwrap(), rotate(&img, -90).unwrap());
    }

    #[test]
    fn no_turn_and_a_full_turn_leave_the_picture() {
        let img = solid(3, 2, [1, 2, 3, 255]);
        assert_eq!(rotate(&img, 0).unwrap(), img);
        assert_eq!(rotate(&img, 360).unwrap(), img);
        assert_eq!(rotate(&rotate(&img, 180).unwrap(), 180).unwrap(), img);
    }

    #[test]
    fn another_angle_makes_a_bigger_canvas_with_transparent_corners() {
        let img = solid(20, 20, [255, 0, 0, 255]);
        let turned = rotate(&img, 45).unwrap();
        assert_eq!(turned.dimensions(), (29, 29));
        assert_eq!(turned.get_pixel(14, 14).0, [255, 0, 0, 255]);
        assert_eq!(turned.get_pixel(0, 0).0[3], 0);
        assert_eq!(turned.get_pixel(28, 0).0[3], 0);
        // The edge of the square is soft: some pixels are only partly there.
        assert!(turned.pixels().any(|p| p.0[3] > 0 && p.0[3] < 255));
        // The colour is not darkened where it is only partly there.
        assert!(
            turned
                .pixels()
                .filter(|p| p.0[3] > 0)
                .all(|p| p.0[0] == 255 && p.0[1] == 0)
        );
    }

    #[test]
    fn the_turn_is_clockwise_for_any_angle() {
        // Left half red, right half blue; turned a little short of a quarter, red ends up on top.
        let mut img = solid(40, 10, [0, 0, 255, 255]);
        for y in 0..10 {
            for x in 0..20 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let turned = rotate(&img, 80).unwrap();
        let (w, h) = turned.dimensions();
        assert!(turned.get_pixel(w / 2, 3).0[0] > 200, "red on top");
        assert!(turned.get_pixel(w / 2, h - 4).0[2] > 200, "blue below");
    }

    #[test]
    fn turning_only_the_frame_gives_the_same_pixels_as_turning_all_and_cutting() {
        // A picture with something in every part, and a frame that is not the whole canvas.
        let mut img = RgbaImage::new(37, 23);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([
                (x * 7) as u8,
                (y * 11) as u8,
                (x * y) as u8,
                200 + (x % 56) as u8,
            ]);
        }
        for degrees in [1, 17, 45, 100, 181, 300, -5, -89] {
            let canvas = rotated_size(37, 23, degrees);
            let rect = CropRect {
                x: canvas.0 / 5,
                y: canvas.1 / 4,
                width: canvas.0 / 2,
                height: canvas.1 / 2,
            };
            let whole = rotate(&img, degrees).unwrap();
            let expected = crate::layout::crop(&whole, rect).unwrap();
            let part = rotate_then_crop(&img, degrees, rect, DEFAULT_MAX_PIXELS).unwrap();
            assert_eq!(part, expected, "{degrees} degrees");
        }
    }

    #[test]
    fn a_frame_outside_the_turned_picture_is_refused_as_before() {
        let img = solid(20, 10, [1, 2, 3, 255]);
        let canvas = rotated_size(20, 10, 30);
        let rect = CropRect {
            x: 0,
            y: 0,
            width: canvas.0 + 1,
            height: 1,
        };
        let whole = rotate(&img, 30).unwrap();
        let wanted = crate::layout::crop(&whole, rect).unwrap_err();
        assert_eq!(
            rotate_then_crop(&img, 30, rect, DEFAULT_MAX_PIXELS).unwrap_err(),
            wanted
        );
    }

    #[test]
    fn a_turn_that_needs_too_big_a_canvas_is_refused() {
        // 142 x 142 = 20164 pixels, one more than the limit.
        let img = RgbaImage::new(100, 100);
        let error = rotate_within(&img, 45, 20_000).unwrap_err();
        assert!(error.contains("too big"), "{error}");
    }
}
