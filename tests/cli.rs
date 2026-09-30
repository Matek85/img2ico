// End-to-end tests: every test starts the real, compiled `img2ico` binary
// in its own temporary working directory and checks what it prints, what
// it writes and how it exits. They cover the seams the unit tests in
// src/ can't reach on their own - argument parsing feeding into config
// resolution feeding into the actual conversion - the way a user would
// hit them.
//
// Conventions used throughout:
// - Every test runs inside a fresh temp directory (the process's current
//   directory), so relative file names work and an "img2ico.toml" is only
//   ever found if the test puts one there.
// - Conversion tests pass `--output-format ico` explicitly (through
//   `convert`), because the default output format is platform-dependent
//   (.icns on macOS) and CI runs on all three platforms.

use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GREEN: [u8; 4] = [0, 255, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];

// --- Running the binary --------------------------------------------------------

fn img2ico(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_img2ico"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to start img2ico")
}

/// A normal image conversion with the output format pinned to .ico.
fn convert(dir: &Path, args: &[&str]) -> Output {
    let mut full = vec!["--output-format", "ico"];
    full.extend_from_slice(args);
    img2ico(dir, &full)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn describe(out: &Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        stdout(out),
        stderr(out)
    )
}

#[track_caller]
fn assert_success(out: &Output) {
    assert!(out.status.success(), "expected success\n{}", describe(out));
}

#[track_caller]
fn assert_failure_containing(out: &Output, needle: &str) {
    assert!(!out.status.success(), "expected failure\n{}", describe(out));
    assert!(
        stderr(out).contains(needle),
        "stderr should contain {needle:?}\n{}",
        describe(out)
    );
}

// --- Fixtures -----------------------------------------------------------------

fn write_image(
    dir: &Path,
    name: &str,
    width: u32,
    height: u32,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> PathBuf {
    let mut img = RgbaImage::new(width, height);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgba(pixel(x, y));
    }
    let path = dir.join(name);
    img.save_with_format(&path, image::ImageFormat::Png)
        .unwrap();
    path
}

fn write_solid(dir: &Path, name: &str, size: u32, color: [u8; 4]) -> PathBuf {
    write_image(dir, name, size, size, |_, _| color)
}

/// 32x32: green border/background with a red 8x8 square in the middle.
fn write_green_background_red_square(dir: &Path, name: &str) -> PathBuf {
    write_image(dir, name, 32, 32, |x, y| {
        if (12..20).contains(&x) && (12..20).contains(&y) {
            RED
        } else {
            GREEN
        }
    })
}

/// 30x30: red, with a 6x6 green patch (10..16) that the border can't reach.
fn write_enclosed_patch(dir: &Path, name: &str) -> PathBuf {
    write_image(dir, name, 30, 30, |x, y| {
        if (10..16).contains(&x) && (10..16).contains(&y) {
            GREEN
        } else {
            RED
        }
    })
}

/// A GIF with one 8x8 solid frame per color.
fn write_gif(dir: &Path, name: &str, colors: &[[u8; 3]]) -> PathBuf {
    let path = dir.join(name);
    let mut encoder = image::codecs::gif::GifEncoder::new(std::fs::File::create(&path).unwrap());
    for c in colors {
        let frame = RgbaImage::from_pixel(8, 8, Rgba([c[0], c[1], c[2], 255]));
        encoder.encode_frame(image::Frame::new(frame)).unwrap();
    }
    path
}

fn read_ico(path: &Path) -> ico::IconDir {
    ico::IconDir::read(std::fs::File::open(path).unwrap()).unwrap()
}

fn ico_sizes(path: &Path) -> Vec<u32> {
    read_ico(path).entries().iter().map(|e| e.width()).collect()
}

/// Decodes the entry of the given edge length into an image.
fn icon_image(path: &Path, size: u32) -> RgbaImage {
    let dir = read_ico(path);
    let entry = dir
        .entries()
        .iter()
        .find(|e| e.width() == size)
        .expect("no entry of that size");
    let image = entry.decode().unwrap();
    RgbaImage::from_raw(size, size, image.rgba_data().to_vec()).unwrap()
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// =============================================================================
// Basic conversion
// =============================================================================

#[test]
fn converts_an_image_with_the_default_sizes() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    let out = convert(dir.path(), &["logo.png"]);
    assert_success(&out);
    assert!(stdout(&out).contains("Done"), "{}", describe(&out));
    assert_eq!(
        ico_sizes(&dir.path().join("logo.ico")),
        vec![16, 32, 48, 64, 128, 256]
    );
}

#[test]
fn every_entry_in_the_output_is_png_encoded_and_keeps_transparency() {
    let dir = tempfile::tempdir().unwrap();
    // One transparent pixel is enough to make the icon "has transparency".
    write_image(dir.path(), "logo.png", 300, 300, |x, y| {
        if x < 150 && y < 150 {
            [0, 0, 0, 0]
        } else {
            RED
        }
    });
    assert_success(&convert(dir.path(), &["logo.png"]));
    let icons = read_ico(&dir.path().join("logo.ico"));
    for entry in icons.entries() {
        assert!(
            entry.is_png(),
            "entry {}x{} is not PNG",
            entry.width(),
            entry.height()
        );
        let payload = image::load_from_memory(entry.data()).unwrap();
        assert!(
            payload.color().has_alpha(),
            "PNG payload has no alpha channel"
        );
    }
}

#[test]
fn a_fully_opaque_source_still_gives_png_entries() {
    // The ico crate writes fully opaque icons as RGB PNGs (no alpha channel
    // needed - nothing to preserve). What matters is that they are PNG, not
    // the legacy BMP encoding with its 1-bit mask and reduced color depth.
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    assert_success(&convert(dir.path(), &["logo.png"]));
    let icons = read_ico(&dir.path().join("logo.ico"));
    assert!(icons.entries().iter().all(|e| e.is_png()));
}

#[test]
fn platform_default_output_format_decides_the_extension() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_success(&img2ico(dir.path(), &["logo.png", "--sizes", "16"]));
    let expected = if cfg!(target_os = "macos") {
        "logo.icns"
    } else {
        "logo.ico"
    };
    assert!(
        dir.path().join(expected).is_file(),
        "expected {expected}: {:?}",
        names_in(dir.path())
    );
}

#[test]
fn output_and_sizes_flags_are_honored() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    assert_success(&convert(
        dir.path(),
        &["logo.png", "-o", "custom.ico", "--sizes", "16, 32 ,64"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("custom.ico")), vec![16, 32, 64]);
    assert!(!dir.path().join("logo.ico").exists());
}

#[test]
fn preset_replaces_sizes() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    assert_success(&convert(
        dir.path(),
        &["logo.png", "--sizes", "99", "--preset", "favicon"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![16, 32, 48]);
}

#[test]
fn windows_preset_produces_the_ten_recommended_sizes() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    assert_success(&convert(dir.path(), &["logo.png", "--preset", "windows"]));
    assert_eq!(
        ico_sizes(&dir.path().join("logo.ico")),
        vec![16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
    );
}

#[test]
fn jpeg_and_bmp_sources_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let img = RgbaImage::from_pixel(40, 40, Rgba([10, 20, 30, 255]));
    image::DynamicImage::ImageRgba8(img.clone())
        .to_rgb8()
        .save(dir.path().join("a.jpg"))
        .unwrap();
    image::DynamicImage::ImageRgba8(img)
        .to_rgb8()
        .save(dir.path().join("b.bmp"))
        .unwrap();
    assert_success(&convert(dir.path(), &["a.jpg", "--sizes", "16"]));
    assert_success(&convert(dir.path(), &["b.bmp", "--sizes", "16"]));
    assert_eq!(ico_sizes(&dir.path().join("a.ico")), vec![16]);
    assert_eq!(ico_sizes(&dir.path().join("b.ico")), vec![16]);
}

#[test]
fn existing_output_is_protected_until_force_is_given() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_success(&convert(dir.path(), &["logo.png", "--sizes", "16"]));

    let again = convert(dir.path(), &["logo.png", "--sizes", "32"]);
    assert_failure_containing(&again, "--force");
    assert_eq!(
        ico_sizes(&dir.path().join("logo.ico")),
        vec![16],
        "old result untouched"
    );

    assert_success(&convert(
        dir.path(),
        &["logo.png", "--sizes", "32", "--force"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![32]);
}

#[test]
fn invalid_size_lists_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "--sizes", "16,abc"]),
        "Invalid size",
    );
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "--sizes", ","]),
        "At least one size",
    );
    assert!(
        !dir.path().join("logo.ico").exists(),
        "nothing may be written on error"
    );
}

#[test]
fn sizes_outside_the_ico_range_are_skipped_with_a_notice() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 600, RED);
    let out = convert(dir.path(), &["logo.png", "--sizes", "16,512,0"]);
    assert_success(&out);
    assert!(
        stderr(&out).contains("Skipping size 512"),
        "{}",
        describe(&out)
    );
    assert!(
        stderr(&out).contains("Skipping size 0"),
        "{}",
        describe(&out)
    );
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![16]);
}

#[test]
fn missing_input_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert_failure_containing(&convert(dir.path(), &["nope.png"]), "Could not read");
}

#[test]
fn unreadable_input_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("junk.png"), b"not an image").unwrap();
    assert_failure_containing(
        &convert(dir.path(), &["junk.png"]),
        "Could not read input file",
    );
}

#[test]
fn several_inputs_without_merge_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    write_solid(dir.path(), "b.png", 16, RED);
    assert_failure_containing(
        &convert(dir.path(), &["a.png", "b.png"]),
        "exactly one input",
    );
}

#[test]
fn no_arguments_at_all_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = img2ico(dir.path(), &[]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("Usage"), "{}", describe(&out));
}

#[test]
fn help_and_version_style_flags_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let out = img2ico(dir.path(), &["--help"]);
    assert_success(&out);
    assert!(stdout(&out).contains("--chroma-key"));
}

// =============================================================================
// Warnings and --silent
// =============================================================================

#[test]
fn upscaling_is_warned_about_and_silent_suppresses_it() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "tiny.png", 16, RED);

    let loud = convert(
        dir.path(),
        &["tiny.png", "--sizes", "16,256", "-o", "loud.ico"],
    );
    assert_success(&loud);
    assert!(stderr(&loud).contains("upscaled"), "{}", describe(&loud));

    let quiet = convert(
        dir.path(),
        &[
            "tiny.png",
            "--sizes",
            "16,256",
            "-o",
            "quiet.ico",
            "--silent",
        ],
    );
    assert_success(&quiet);
    assert_eq!(stderr(&quiet), "", "--silent must leave stderr empty");
}

#[test]
fn a_thin_sliver_of_content_is_warned_about() {
    let dir = tempfile::tempdir().unwrap();
    write_image(dir.path(), "banner.png", 800, 50, |_, _| RED);
    let out = convert(dir.path(), &["banner.png", "--sizes", "16", "-o", "b.ico"]);
    assert_success(&out);
    assert!(stderr(&out).contains("thin sliver"), "{}", describe(&out));
}

#[test]
fn silent_does_not_hide_errors() {
    let dir = tempfile::tempdir().unwrap();
    assert_failure_containing(
        &convert(dir.path(), &["nope.png", "--silent"]),
        "Could not read",
    );
}

// =============================================================================
// Chroma key, replace color, grayscale, padding
// =============================================================================

#[test]
fn chroma_key_makes_the_background_transparent() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png", "-c", "#00FF00", "--sizes", "32", "-o", "out.ico",
        ],
    ));
    let icon = icon_image(&dir.path().join("out.ico"), 32);
    assert_eq!(icon.get_pixel(0, 0)[3], 0, "background is transparent");
    assert_eq!(icon.get_pixel(31, 31)[3], 0);
    assert_eq!(icon.get_pixel(16, 16).0, RED, "the subject is untouched");
}

#[test]
fn chroma_key_accepts_the_hex_code_without_a_hash() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_success(&convert(
        dir.path(),
        &["shot.png", "-c", "00ff00", "--sizes", "32", "-o", "out.ico"],
    ));
    assert_eq!(
        icon_image(&dir.path().join("out.ico"), 32).get_pixel(0, 0)[3],
        0
    );
}

#[test]
fn an_invalid_chroma_key_color_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    assert_failure_containing(
        &convert(dir.path(), &["a.png", "-c", "banana"]),
        "Invalid --chroma-key value",
    );
}

#[test]
fn tolerance_controls_how_similar_a_color_must_be() {
    let dir = tempfile::tempdir().unwrap();
    // Slightly-off green background around a red square.
    write_image(dir.path(), "shot.png", 32, 32, |x, y| {
        if (12..20).contains(&x) && (12..20).contains(&y) {
            RED
        } else {
            [0, 215, 0, 255]
        }
    });
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "00FF00",
            "-t",
            "0",
            "--sizes",
            "32",
            "-o",
            "strict.ico",
        ],
    ));
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "00FF00",
            "-t",
            "30",
            "--sizes",
            "32",
            "-o",
            "lenient.ico",
        ],
    ));
    assert_eq!(
        icon_image(&dir.path().join("strict.ico"), 32).get_pixel(0, 0)[3],
        255
    );
    assert_eq!(
        icon_image(&dir.path().join("lenient.ico"), 32).get_pixel(0, 0)[3],
        0
    );
}

#[test]
fn seed_reaches_an_enclosed_region() {
    let dir = tempfile::tempdir().unwrap();
    write_enclosed_patch(dir.path(), "patch.png");
    assert_success(&convert(
        dir.path(),
        &[
            "patch.png",
            "-c",
            "00FF00",
            "--sizes",
            "30",
            "-o",
            "plain.ico",
        ],
    ));
    assert_success(&convert(
        dir.path(),
        &[
            "patch.png",
            "-c",
            "00FF00",
            "--seed",
            "12,12",
            "--sizes",
            "30",
            "-o",
            "seeded.ico",
        ],
    ));
    assert_eq!(
        icon_image(&dir.path().join("plain.ico"), 30).get_pixel(12, 12)[3],
        255
    );
    assert_eq!(
        icon_image(&dir.path().join("seeded.ico"), 30).get_pixel(12, 12)[3],
        0
    );
}

#[test]
fn an_invalid_seed_is_an_error_and_an_outside_seed_only_warns() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_failure_containing(
        &convert(dir.path(), &["shot.png", "-c", "00FF00", "--seed", "oops"]),
        "Invalid --seed value",
    );
    let out = convert(
        dir.path(),
        &[
            "shot.png", "-c", "00FF00", "--seed", "999,999", "--sizes", "32", "-o", "o.ico",
        ],
    );
    assert_success(&out);
    assert!(
        stderr(&out).contains("outside the image"),
        "{}",
        describe(&out)
    );
}

#[test]
fn find_previews_regions_without_writing_anything() {
    let dir = tempfile::tempdir().unwrap();
    write_enclosed_patch(dir.path(), "patch.png");
    let out = convert(
        dir.path(),
        &["patch.png", "--find", "00FF00", "--out-toml", "snap.toml"],
    );
    assert_success(&out);
    let text = stdout(&out);
    assert!(
        text.contains("Found 1 additional region"),
        "{}",
        describe(&out)
    );
    assert!(
        text.contains("~36 pixel(s) near (12, 12)"),
        "{}",
        describe(&out)
    );
    assert!(text.contains("--seed 12,12"), "{}", describe(&out));
    assert_eq!(
        names_in(dir.path()),
        vec!["patch.png"],
        "a --find preview must not create any file"
    );
}

#[test]
fn find_reports_when_there_is_nothing_to_find() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(dir.path(), &["shot.png", "--find", "00FF00"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("No additional regions"),
        "{}",
        describe(&out)
    );
}

#[test]
fn find_min_size_filters_small_regions() {
    let dir = tempfile::tempdir().unwrap();
    write_enclosed_patch(dir.path(), "patch.png"); // the patch has 36 pixels
    let out = convert(
        dir.path(),
        &["patch.png", "--find", "00FF00", "--find-min-size", "37"],
    );
    assert_success(&out);
    assert!(
        stdout(&out).contains("No additional regions"),
        "{}",
        describe(&out)
    );
}

#[test]
fn find_with_auto_apply_converts_using_the_discovered_seeds() {
    let dir = tempfile::tempdir().unwrap();
    write_enclosed_patch(dir.path(), "patch.png");
    assert_success(&convert(
        dir.path(),
        &[
            "patch.png",
            "--find",
            "00FF00",
            "--auto-apply",
            "--sizes",
            "30",
            "-o",
            "out.ico",
        ],
    ));
    let icon = icon_image(&dir.path().join("out.ico"), 30);
    assert_eq!(
        icon.get_pixel(12, 12)[3],
        0,
        "the enclosed patch is removed"
    );
    assert_eq!(icon.get_pixel(2, 2).0, RED, "the surroundings stay");
}

#[test]
fn replace_color_swaps_the_background_instead_of_making_it_transparent() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "00FF00",
            "--replace-color",
            "0000FF",
            "--sizes",
            "32",
            "-o",
            "out.ico",
        ],
    ));
    let icon = icon_image(&dir.path().join("out.ico"), 32);
    assert_eq!(icon.get_pixel(0, 0).0, [0, 0, 255, 255]);
    assert_eq!(icon.get_pixel(16, 16).0, RED);
}

#[test]
fn grayscale_removes_all_color() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "red.png", 32, RED);
    assert_success(&convert(
        dir.path(),
        &["red.png", "--grayscale", "--sizes", "32", "-o", "out.ico"],
    ));
    assert_eq!(
        icon_image(&dir.path().join("out.ico"), 32)
            .get_pixel(16, 16)
            .0,
        [76, 76, 76, 255]
    );
}

#[test]
fn grayscale_also_covers_a_replacement_color() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "00FF00",
            "--replace-color",
            "0000FF",
            "--grayscale",
            "--sizes",
            "32",
            "-o",
            "o.ico",
        ],
    ));
    // Pure blue in BT.601 luma is 29.
    assert_eq!(
        icon_image(&dir.path().join("o.ico"), 32).get_pixel(0, 0).0,
        [29, 29, 29, 255]
    );
}

#[test]
fn padding_leaves_a_transparent_margin() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "red.png", 64, RED);
    assert_success(&convert(
        dir.path(),
        &[
            "red.png",
            "--padding",
            "50",
            "--sizes",
            "32",
            "-o",
            "out.ico",
        ],
    ));
    let icon = icon_image(&dir.path().join("out.ico"), 32);
    assert_eq!(icon.get_pixel(2, 16)[3], 0);
    assert_eq!(icon.get_pixel(16, 16)[3], 255);
}

#[test]
fn non_square_images_are_letterboxed_not_stretched() {
    let dir = tempfile::tempdir().unwrap();
    write_image(dir.path(), "wide.png", 128, 32, |_, _| RED);
    assert_success(&convert(
        dir.path(),
        &["wide.png", "--sizes", "32", "-o", "out.ico"],
    ));
    let icon = icon_image(&dir.path().join("out.ico"), 32);
    assert_eq!(icon.get_pixel(16, 0)[3], 0, "transparent bar above");
    assert_eq!(icon.get_pixel(16, 16).0, RED);
}

// =============================================================================
// Conflicting options
// =============================================================================

#[test]
fn chroma_key_and_find_are_mutually_exclusive() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    assert_failure_containing(
        &convert(dir.path(), &["a.png", "-c", "00FF00", "--find", "00FF00"]),
        "mutually exclusive",
    );
}

#[test]
fn auto_apply_needs_find() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    assert_failure_containing(
        &convert(dir.path(), &["a.png", "--auto-apply"]),
        "--auto-apply has no effect without --find",
    );
}

#[test]
fn replace_color_needs_a_background_color() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    assert_failure_containing(
        &convert(dir.path(), &["a.png", "--replace-color", "000000"]),
        "--replace-color requires",
    );
}

#[test]
fn modes_cannot_be_combined() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 16, RED);
    assert_failure_containing(
        &img2ico(dir.path(), &["a.png", "--merge", "--inspect"]),
        "mutually exclusive",
    );
    assert_failure_containing(
        &img2ico(dir.path(), &["a.png", "--extract", "--select"]),
        "mutually exclusive",
    );
    assert_failure_containing(
        &img2ico(
            dir.path(),
            &["a.png", "--inspect", "--output-format", "ico"],
        ),
        "mutually exclusive",
    );
}

// =============================================================================
// ICNS output
// =============================================================================

#[test]
fn icns_output_has_the_icns_magic_and_a_consistent_length() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    let out = img2ico(dir.path(), &["logo.png", "--output-format", "icns"]);
    assert_success(&out);
    let bytes = std::fs::read(dir.path().join("logo.icns")).unwrap();
    assert_eq!(&bytes[0..4], b"icns");
    assert_eq!(
        u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize,
        bytes.len()
    );
}

// =============================================================================
// Merge, inspect, extract, select
// =============================================================================

fn make_icos(dir: &Path) {
    write_solid(dir, "logo.png", 300, RED);
    assert_success(&convert(dir, &["logo.png", "--sizes", "16", "-o", "a.ico"]));
    assert_success(&convert(
        dir,
        &["logo.png", "--sizes", "32,64", "-o", "b.ico"],
    ));
}

#[test]
fn merge_combines_icons_into_one_file() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let out = img2ico(
        dir.path(),
        &["--merge", "a.ico", "b.ico", "-o", "merged.ico"],
    );
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("merged.ico")), vec![16, 32, 64]);
}

#[test]
fn merge_requires_an_explicit_output() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_failure_containing(
        &img2ico(dir.path(), &["--merge", "a.ico", "b.ico"]),
        "explicit output path",
    );
}

#[test]
fn merge_skips_duplicate_sizes_with_a_warning_unless_silent() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let loud = img2ico(dir.path(), &["--merge", "a.ico", "a.ico", "-o", "loud.ico"]);
    assert_success(&loud);
    assert!(
        stderr(&loud).contains("already present"),
        "{}",
        describe(&loud)
    );
    let quiet = img2ico(
        dir.path(),
        &["--merge", "a.ico", "a.ico", "-o", "quiet.ico", "--silent"],
    );
    assert_success(&quiet);
    assert_eq!(stderr(&quiet), "");
}

#[test]
fn merge_with_delete_source_removes_the_inputs_but_keeps_the_result() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(
        dir.path(),
        &[
            "--merge",
            "a.ico",
            "b.ico",
            "-o",
            "merged.ico",
            "--delete-source",
        ],
    ));
    assert_eq!(names_in(dir.path()), vec!["logo.png", "merged.ico"]);
}

#[test]
fn inspect_reports_the_entries_of_an_ico() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let out = img2ico(dir.path(), &["--inspect", "b.ico"]);
    assert_success(&out);
    let text = stdout(&out);
    assert!(text.contains("[0]") && text.contains("[1]"), "{text}");
    assert!(text.contains("32x32") && text.contains("64x64"), "{text}");
    assert!(text.contains("PNG"), "{text}");
    assert!(text.contains("missing common Windows sizes"), "{text}");
}

#[test]
fn inspect_reports_the_resolution_and_coverage_of_a_source_image() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "small.png", 100, RED);
    let out = img2ico(dir.path(), &["--inspect", "small.png"]);
    assert_success(&out);
    let text = stdout(&out);
    assert!(text.contains("source image, 100x100"), "{text}");
    assert!(text.contains("would need upscaling"), "{text}");
    assert!(text.contains("tip:"), "{text}");
}

#[test]
fn inspect_reports_the_frame_count_of_a_gif() {
    let dir = tempfile::tempdir().unwrap();
    write_gif(
        dir.path(),
        "anim.gif",
        &[[255, 0, 0], [0, 255, 0], [0, 0, 255]],
    );
    let out = img2ico(dir.path(), &["--inspect", "anim.gif"]);
    assert_success(&out);
    assert!(stdout(&out).contains("frames: 3"), "{}", describe(&out));
}

#[test]
fn inspect_never_writes_anything() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let before = names_in(dir.path());
    assert_success(&img2ico(
        dir.path(),
        &[
            "--inspect",
            "a.ico",
            "b.ico",
            "logo.png",
            "--out-toml",
            "x.toml",
        ],
    ));
    assert_eq!(names_in(dir.path()), before);
}

#[test]
fn inspect_of_garbage_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("junk.bin"), b"junk").unwrap();
    assert_failure_containing(
        &img2ico(dir.path(), &["--inspect", "junk.bin"]),
        "neither a readable .ico",
    );
}

#[test]
fn extract_saves_every_size_as_a_png() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(dir.path(), &["--extract", "b.ico", "-o", "pngs"]));
    assert_eq!(
        names_in(&dir.path().join("pngs")),
        vec!["b_32x32.png", "b_64x64.png"]
    );
    assert_eq!(
        image::open(dir.path().join("pngs").join("b_64x64.png"))
            .unwrap()
            .width(),
        64
    );
}

#[test]
fn extract_defaults_to_a_named_directory_and_needs_exactly_one_input() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(dir.path(), &["--extract", "a.ico"]));
    assert!(dir.path().join("a_extracted").join("a_16x16.png").is_file());
    assert_failure_containing(
        &img2ico(dir.path(), &["--extract", "a.ico", "b.ico"]),
        "exactly one input",
    );
}

#[test]
fn select_pulls_one_icon_by_index() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(dir.path(), &["--select", "b.ico", "--index", "1"]));
    assert_eq!(ico_sizes(&dir.path().join("b_64x64.ico")), vec![64]);
}

#[test]
fn select_defaults_to_the_first_icon() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(dir.path(), &["--select", "b.ico"]));
    assert_eq!(ico_sizes(&dir.path().join("b_32x32.ico")), vec![32]);
}

#[test]
fn select_combine_bundles_several_icons() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(
        dir.path(),
        &["--select", "b.ico", "--index", "1,0", "--combine"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("b_selected.ico")), vec![64, 32]);
}

#[test]
fn select_with_several_indices_writes_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(
        dir.path(),
        &["--select", "b.ico", "--index", "0,1"],
    ));
    assert_eq!(
        names_in(&dir.path().join("b_selected")),
        vec!["b_32x32.ico", "b_64x64.ico"]
    );
}

#[test]
fn select_rejects_an_out_of_range_index() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_failure_containing(
        &img2ico(dir.path(), &["--select", "b.ico", "--index", "5"]),
        "out of range",
    );
}

// =============================================================================
// --delete-source
// =============================================================================

#[test]
fn delete_source_removes_the_input_after_success() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let out = convert(
        dir.path(),
        &["logo.png", "--sizes", "16", "--delete-source"],
    );
    assert_success(&out);
    assert!(
        stdout(&out).contains("Deleted source file"),
        "{}",
        describe(&out)
    );
    assert!(!dir.path().join("logo.png").exists());
    assert!(dir.path().join("logo.ico").is_file());
}

#[test]
fn delete_source_keeps_the_input_when_the_run_fails() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::write(dir.path().join("logo.ico"), b"existing").unwrap();
    let out = convert(dir.path(), &["logo.png", "--delete-source"]);
    assert_failure_containing(&out, "--force");
    assert!(
        dir.path().join("logo.png").is_file(),
        "the source must survive a failed run"
    );
}

#[test]
fn delete_source_never_deletes_the_file_it_just_wrote() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    // Output path == input path: the result replaces the source (with
    // --force); it must not then be deleted as "the source".
    let out = convert(
        dir.path(),
        &[
            "logo.png",
            "--sizes",
            "16",
            "-o",
            "logo.png",
            "--force",
            "--delete-source",
        ],
    );
    assert_success(&out);
    assert!(dir.path().join("logo.png").is_file(), "{}", describe(&out));
    assert!(
        stderr(&out).contains("also the output path"),
        "{}",
        describe(&out)
    );
}

// =============================================================================
// Config files and --out-toml
// =============================================================================

#[test]
fn config_file_values_apply_and_are_announced() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("my.toml"),
        "sizes = \"16,24\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    let out = img2ico(dir.path(), &["logo.png", "--config", "my.toml"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("Using settings from"),
        "{}",
        describe(&out)
    );
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![16, 24]);
}

#[test]
fn command_line_beats_the_config_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("my.toml"),
        "sizes = \"16,24\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    assert_success(&img2ico(
        dir.path(),
        &["logo.png", "--config", "my.toml", "--sizes", "48"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![48]);
}

#[test]
fn img2ico_toml_in_the_working_directory_is_picked_up_automatically() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("img2ico.toml"),
        "sizes = \"20\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    let out = img2ico(dir.path(), &["logo.png"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("Using settings from 'img2ico.toml'"),
        "{}",
        describe(&out)
    );
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![20]);
}

#[test]
fn an_explicit_config_beats_the_automatic_one() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("img2ico.toml"),
        "sizes = \"20\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("other.toml"),
        "sizes = \"40\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    assert_success(&img2ico(
        dir.path(),
        &["logo.png", "--config", "other.toml"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![40]);
}

#[test]
fn silent_hides_the_config_notice() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("img2ico.toml"),
        "sizes = \"20\"\noutput-format = \"ico\"\n",
    )
    .unwrap();
    let out = img2ico(dir.path(), &["logo.png", "--silent"]);
    assert_success(&out);
    assert!(
        !stdout(&out).contains("Using settings from"),
        "{}",
        describe(&out)
    );
}

#[test]
fn a_config_can_turn_on_flags_like_force() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::write(dir.path().join("logo.ico"), b"old").unwrap();
    std::fs::write(
        dir.path().join("c.toml"),
        "force = true\noutput-format = \"ico\"\nsizes = \"16\"\n",
    )
    .unwrap();
    assert_success(&img2ico(dir.path(), &["logo.png", "--config", "c.toml"]));
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![16]);
}

#[test]
fn unknown_config_settings_produce_a_warning_but_the_rest_applies() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(
        dir.path().join("c.toml"),
        "toleranse = 5\nsizes = \"16\"\noutput-format = \"ico\"\n",
    )
    .unwrap();

    let loud = img2ico(dir.path(), &["logo.png", "--config", "c.toml"]);
    assert_success(&loud);
    assert!(
        stderr(&loud).contains("unknown setting 'toleranse'"),
        "{}",
        describe(&loud)
    );
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![16]);

    let quiet = img2ico(
        dir.path(),
        &["logo.png", "--config", "c.toml", "--silent", "--force"],
    );
    assert_success(&quiet);
    assert!(
        !stderr(&quiet).contains("unknown setting"),
        "{}",
        describe(&quiet)
    );
}

#[test]
fn a_broken_config_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::write(dir.path().join("bad.toml"), "tolerance = \"high\"\n").unwrap();
    assert_failure_containing(
        &img2ico(dir.path(), &["logo.png", "--config", "bad.toml"]),
        "Could not parse config file",
    );
    assert_failure_containing(
        &img2ico(dir.path(), &["logo.png", "--config", "missing.toml"]),
        "Could not read config file",
    );
}

#[test]
fn out_toml_snapshots_the_settings_that_were_used() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    std::fs::write(dir.path().join("c.toml"), "tolerance = 35\npadding = 5\n").unwrap();
    assert_success(&convert(
        dir.path(),
        &[
            "logo.png",
            "--sizes",
            "16",
            "--config",
            "c.toml",
            "--padding",
            "9",
            "--out-toml",
            "snap.toml",
        ],
    ));
    let text = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(text.contains("tolerance = 35"), "config value kept: {text}");
    assert!(text.contains("padding = 9"), "command line wins: {text}");
    assert!(
        text.contains("gif-frame = 1"),
        "defaults are written explicitly: {text}"
    );
}

#[test]
fn a_snapshot_can_be_fed_back_in_and_reproduces_itself() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    assert_success(&convert(
        dir.path(),
        &[
            "logo.png",
            "--sizes",
            "16,32",
            "--padding",
            "4",
            "--out-toml",
            "first.toml",
        ],
    ));
    assert_success(&img2ico(
        dir.path(),
        &[
            "logo.png",
            "--config",
            "first.toml",
            "-o",
            "second.ico",
            "--out-toml",
            "second.toml",
        ],
    ));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("first.toml")).unwrap(),
        std::fs::read_to_string(dir.path().join("second.toml")).unwrap()
    );
}

#[test]
fn out_toml_is_also_supported_by_the_other_modes() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_success(&img2ico(
        dir.path(),
        &[
            "--merge",
            "a.ico",
            "b.ico",
            "-o",
            "m.ico",
            "--out-toml",
            "m.toml",
        ],
    ));
    assert!(dir.path().join("m.toml").is_file());
    assert_success(&img2ico(
        dir.path(),
        &["--extract", "m.ico", "--out-toml", "e.toml"],
    ));
    assert!(dir.path().join("e.toml").is_file());
    assert_success(&img2ico(
        dir.path(),
        &["--select", "m.ico", "--out-toml", "s.toml"],
    ));
    assert!(dir.path().join("s.toml").is_file());
}

// =============================================================================
// Animated GIFs
// =============================================================================

fn dominant(pixel: &Rgba<u8>) -> usize {
    (0..3).max_by_key(|&c| pixel[c]).unwrap()
}

#[test]
fn gif_uses_the_first_frame_by_default() {
    let dir = tempfile::tempdir().unwrap();
    write_gif(
        dir.path(),
        "anim.gif",
        &[[255, 0, 0], [0, 255, 0], [0, 0, 255]],
    );
    assert_success(&convert(
        dir.path(),
        &["anim.gif", "--sizes", "8", "-o", "out.ico"],
    ));
    assert_eq!(
        dominant(icon_image(&dir.path().join("out.ico"), 8).get_pixel(4, 4)),
        0
    );
}

#[test]
fn gif_frame_selects_another_frame() {
    let dir = tempfile::tempdir().unwrap();
    write_gif(
        dir.path(),
        "anim.gif",
        &[[255, 0, 0], [0, 255, 0], [0, 0, 255]],
    );
    assert_success(&convert(
        dir.path(),
        &[
            "anim.gif",
            "--gif-frame",
            "2",
            "--sizes",
            "8",
            "-o",
            "g.ico",
        ],
    ));
    assert_success(&convert(
        dir.path(),
        &[
            "anim.gif",
            "--gif-frame",
            "3",
            "--sizes",
            "8",
            "-o",
            "b.ico",
        ],
    ));
    assert_eq!(
        dominant(icon_image(&dir.path().join("g.ico"), 8).get_pixel(4, 4)),
        1
    );
    assert_eq!(
        dominant(icon_image(&dir.path().join("b.ico"), 8).get_pixel(4, 4)),
        2
    );
}

#[test]
fn gif_frame_out_of_range_names_the_frame_count() {
    let dir = tempfile::tempdir().unwrap();
    write_gif(dir.path(), "anim.gif", &[[255, 0, 0], [0, 255, 0]]);
    let out = convert(dir.path(), &["anim.gif", "--gif-frame", "9"]);
    assert_failure_containing(&out, "out of range");
    assert!(stderr(&out).contains("2 frame(s)"), "{}", describe(&out));
    assert_failure_containing(
        &convert(dir.path(), &["anim.gif", "--gif-frame", "0"]),
        "1 or greater",
    );
}

#[test]
fn gif_frame_on_a_non_gif_only_warns() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let loud = convert(
        dir.path(),
        &[
            "a.png",
            "--gif-frame",
            "2",
            "--sizes",
            "16",
            "-o",
            "loud.ico",
        ],
    );
    assert_success(&loud);
    assert!(
        stderr(&loud).contains("only applies to GIF"),
        "{}",
        describe(&loud)
    );
    let quiet = convert(
        dir.path(),
        &[
            "a.png",
            "--gif-frame",
            "2",
            "--sizes",
            "16",
            "-o",
            "quiet.ico",
            "--silent",
        ],
    );
    assert_success(&quiet);
    assert_eq!(stderr(&quiet), "");
}
