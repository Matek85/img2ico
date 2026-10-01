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
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const GREEN: [u8; 4] = [0, 255, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];

// --- Running the binary --------------------------------------------------------

fn img2ico(dir: &Path, args: &[&str]) -> Output {
    // Every run gets its own fake home directory, so what a test does never
    // depends on a real per-user settings file on the machine running it.
    // Setting all three variables makes this work on every platform (Windows
    // reads APPDATA, Linux XDG_CONFIG_HOME or HOME, macOS HOME).
    let home = fake_home(dir);
    Command::new(env!("CARGO_BIN_EXE_img2ico"))
        .current_dir(dir)
        .env("APPDATA", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("HOME", &home)
        .args(args)
        .output()
        .expect("failed to start img2ico")
}

/// Like `img2ico`, but with `input` piped into standard input.
fn img2ico_with_stdin(dir: &Path, args: &[&str], input: &[u8]) -> Output {
    let home = fake_home(dir);
    let mut child = Command::new(env!("CARGO_BIN_EXE_img2ico"))
        .current_dir(dir)
        .env("APPDATA", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("HOME", &home)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start img2ico");
    // Feed the input from a thread, so a large input can't deadlock against
    // the output being read.
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    out
}

/// The icon directory in the bytes an icon run wrote to standard output.
fn ico_in(bytes: &[u8]) -> ico::IconDir {
    ico::IconDir::read(std::io::Cursor::new(bytes))
        .unwrap_or_else(|e| panic!("standard output is not a valid ICO file: {e}"))
}

/// The fake home directory used for runs in `dir` (it doesn't exist until a
/// test creates something in it).
fn fake_home(dir: &Path) -> PathBuf {
    dir.join("home")
}

/// Where img2ico looks for the per-user settings file when run in `dir`.
fn user_config_file(dir: &Path) -> PathBuf {
    let config_base = if cfg!(target_os = "macos") {
        fake_home(dir).join("Library").join("Application Support")
    } else {
        fake_home(dir)
    };
    config_base.join("img2ico").join("config.toml")
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
    read_ico(path)
        .entries()
        .iter()
        .map(ico::IconDirEntry::width)
        .collect()
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
    assert!(icons.entries().iter().all(ico::IconDirEntry::is_png));
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

#[test]
fn version_prints_the_version_platform_and_compiler() {
    let dir = tempfile::tempdir().unwrap();
    let out = img2ico(dir.path(), &["--version"]);
    assert_success(&out);
    let text = stdout(&out);
    let first_line = text.lines().next().unwrap();
    assert_eq!(first_line, format!("img2ico {}", env!("CARGO_PKG_VERSION")));
    assert!(text.contains("target:"), "{text}");
    assert!(text.contains("compiler: rustc"), "{text}");
    assert_eq!(stderr(&out), "");
}

#[test]
fn short_version_flag_prints_only_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let out = img2ico(dir.path(), &["-V"]);
    assert_success(&out);
    assert_eq!(
        stdout(&out).trim(),
        format!("img2ico {}", env!("CARGO_PKG_VERSION"))
    );
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
fn an_output_that_is_the_input_itself_is_refused_even_with_force_and_delete_source() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let before = std::fs::read(dir.path().join("logo.png")).unwrap();
    // Output path == input path: the icon would replace the source image.
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
    assert_failure_containing(&out, "is the input file itself");
    assert_eq!(
        std::fs::read(dir.path().join("logo.png")).unwrap(),
        before,
        "the source is untouched"
    );

    // The same through a different spelling of the same path.
    let out = convert(
        dir.path(),
        &["logo.png", "--sizes", "16", "-o", "./logo.png", "--force"],
    );
    assert_failure_containing(&out, "is the input file itself");
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
// Per-user settings file and --no-config
// =============================================================================

/// Writes the per-user settings file into the fake home of this test.
fn write_user_config(dir: &Path, text: &str) -> PathBuf {
    let path = user_config_file(dir);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_user_config_file_supplies_defaults_and_is_announced() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    let user = write_user_config(dir.path(), "sizes = \"20\"\noutput-format = \"ico\"\n");

    let out = img2ico(dir.path(), &["logo.png"]);
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![20]);
    let notice = format!("Using settings from '{}'.", user.display());
    assert!(stdout(&out).contains(&notice), "{}", describe(&out));
}

#[test]
fn the_project_file_overrides_the_user_file_setting_by_setting() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    let user = write_user_config(
        dir.path(),
        "sizes = \"20\"\npadding = 10\noutput-format = \"ico\"\n",
    );
    std::fs::write(dir.path().join("img2ico.toml"), "sizes = \"40\"\n").unwrap();

    let out = img2ico(dir.path(), &["logo.png", "--out-toml", "snap.toml"]);
    assert_success(&out);
    assert_eq!(
        ico_sizes(&dir.path().join("logo.ico")),
        vec![40],
        "the project file's sizes win"
    );
    let snapshot = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(
        snapshot.contains("padding = 10"),
        "the user file fills the gap: {snapshot}"
    );

    // Both files are announced, the project file first.
    let text = stdout(&out);
    let project_at = text.find("Using settings from 'img2ico.toml'").unwrap();
    let user_at = text.find(&user.display().to_string()).unwrap();
    assert!(project_at < user_at, "{}", describe(&out));
}

#[test]
fn the_command_line_beats_both_settings_files() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    write_user_config(
        dir.path(),
        "sizes = \"20\"\npadding = 10\noutput-format = \"ico\"\n",
    );
    std::fs::write(dir.path().join("img2ico.toml"), "sizes = \"40\"\n").unwrap();

    let out = img2ico(
        dir.path(),
        &[
            "logo.png",
            "--sizes",
            "48",
            "--padding",
            "0",
            "--out-toml",
            "snap.toml",
        ],
    );
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![48]);
    let snapshot = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(snapshot.contains("padding = 0"), "{snapshot}");
}

#[test]
fn an_explicit_config_replaces_img2ico_toml_but_the_user_file_still_applies() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    write_user_config(
        dir.path(),
        "padding = 10\noutput-format = \"ico\"\nsizes = \"20\"\n",
    );
    std::fs::write(dir.path().join("img2ico.toml"), "sizes = \"30\"\n").unwrap();
    std::fs::write(dir.path().join("other.toml"), "sizes = \"40\"\n").unwrap();

    let out = img2ico(
        dir.path(),
        &[
            "logo.png",
            "--config",
            "other.toml",
            "--out-toml",
            "snap.toml",
        ],
    );
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("logo.ico")), vec![40]);
    assert!(
        !stdout(&out).contains("img2ico.toml"),
        "the automatic file must not be used: {}",
        describe(&out)
    );
    let snapshot = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(snapshot.contains("padding = 10"), "{snapshot}");
}

#[test]
fn no_config_ignores_every_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 300, RED);
    write_user_config(dir.path(), "sizes = \"20\"\npadding = 10\nforce = true\n");
    std::fs::write(dir.path().join("img2ico.toml"), "sizes = \"40\"\n").unwrap();

    let out = convert(
        dir.path(),
        &["logo.png", "--no-config", "--out-toml", "snap.toml"],
    );
    assert_success(&out);
    assert_eq!(
        ico_sizes(&dir.path().join("logo.ico")),
        vec![16, 32, 48, 64, 128, 256],
        "built-in default sizes"
    );
    assert!(
        !stdout(&out).contains("Using settings from"),
        "{}",
        describe(&out)
    );
    let snapshot = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(snapshot.contains("padding = 0"), "{snapshot}");
    assert!(snapshot.contains("force = false"), "{snapshot}");
}

#[test]
fn no_config_cannot_be_combined_with_config() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::write(dir.path().join("c.toml"), "padding = 5\n").unwrap();
    let out = img2ico(
        dir.path(),
        &["logo.png", "--no-config", "--config", "c.toml"],
    );
    assert_eq!(out.status.code(), Some(2), "{}", describe(&out));
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        describe(&out)
    );
}

#[test]
fn a_broken_user_config_is_an_error_but_no_config_avoids_it() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_user_config(dir.path(), "tolerance = \"high\"\n");

    let out = convert(dir.path(), &["logo.png", "--sizes", "16"]);
    assert_failure_containing(&out, "Could not parse config file");
    assert!(stderr(&out).contains("config.toml"), "{}", describe(&out));

    assert_success(&convert(
        dir.path(),
        &["logo.png", "--sizes", "16", "--no-config"],
    ));
}

#[test]
fn unknown_settings_in_the_user_file_are_reported_unless_silent() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_user_config(dir.path(), "toleranse = 5\noutput-format = \"ico\"\n");

    let loud = img2ico(dir.path(), &["logo.png", "--sizes", "16"]);
    assert_success(&loud);
    assert!(
        stderr(&loud).contains("unknown setting 'toleranse'"),
        "{}",
        describe(&loud)
    );

    let quiet = img2ico(
        dir.path(),
        &["logo.png", "--sizes", "16", "--silent", "--force"],
    );
    assert_success(&quiet);
    assert_eq!(stderr(&quiet), "");
    assert!(!stdout(&quiet).contains("Using settings from"));
}

// =============================================================================
// Standard input and output ("-") only work for converting one image
// =============================================================================

#[test]
fn dash_is_refused_in_the_modes_that_work_on_existing_ico_files() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    make_icos(dir.path());

    let attempts: [&[&str]; 6] = [
        &["--merge", "a.ico", "b.ico", "-o", "-"],
        &["--extract", "a.ico", "-o", "-"],
        &["--select", "b.ico", "-o", "-"],
        &["--inspect", "-"],
        &["--merge", "-", "b.ico", "-o", "m.ico"],
        &["--extract", "-", "-o", "dir"],
    ];
    for args in attempts {
        let out = img2ico(dir.path(), args);
        assert_failure_containing(&out, "only works when converting one image");
    }
    assert!(
        !dir.path().join("-").exists(),
        "no file or folder named '-' may have been created"
    );
}

#[test]
fn a_file_that_really_is_called_dash_can_still_be_written() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let out = convert(dir.path(), &["logo.png", "--sizes", "16", "-o", "./-"]);
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("-")), vec![16]);
}

// =============================================================================
// Pipelines: standard input, standard output and --quiet
// =============================================================================

#[test]
fn an_image_from_standard_input_becomes_an_icon_file() {
    let dir = tempfile::tempdir().unwrap();
    let png = std::fs::read(write_solid(dir.path(), "logo.png", 64, RED)).unwrap();
    let out = img2ico_with_stdin(
        dir.path(),
        &[
            "-",
            "-o",
            "icon.ico",
            "--output-format",
            "ico",
            "--sizes",
            "16,32",
        ],
        &png,
    );
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("icon.ico")), vec![16, 32]);
    assert!(
        stdout(&out).contains("Done: 'icon.ico' created"),
        "{}",
        describe(&out)
    );
}

#[test]
fn an_icon_can_be_written_to_standard_output() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let out = img2ico(
        dir.path(),
        &[
            "logo.png",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16,32",
        ],
    );
    assert_success(&out);
    let icons = ico_in(&out.stdout);
    let sizes: Vec<u32> = icons.entries().iter().map(|e| e.width()).collect();
    assert_eq!(sizes, vec![16, 32]);
    assert_eq!(
        &out.stdout[..4],
        &[0, 0, 1, 0],
        "an ICO header, and nothing before it"
    );
    assert!(
        stderr(&out).contains("Done: icon written to standard output"),
        "the Done line goes to standard error: {}",
        describe(&out)
    );
    assert!(!dir.path().join("-").exists(), "no file called '-'");
}

#[test]
fn a_whole_pipeline_works_from_standard_input_to_standard_output() {
    let dir = tempfile::tempdir().unwrap();
    let png = std::fs::read(write_solid(dir.path(), "logo.png", 64, GREEN)).unwrap();
    let out = img2ico_with_stdin(
        dir.path(),
        &["-", "-o", "-", "--output-format", "ico", "--sizes", "32"],
        &png,
    );
    assert_success(&out);
    let icons = ico_in(&out.stdout);
    assert_eq!(icons.entries().len(), 1);
    let image = icons.entries()[0].decode().unwrap();
    assert_eq!(&image.rgba_data()[..4], &[0, 255, 0, 255]);
}

#[test]
fn standard_output_holds_nothing_but_the_icon_even_with_warnings_and_notices() {
    let dir = tempfile::tempdir().unwrap();
    // Too small for 16 px (a warning), plus a detected background (a notice),
    // plus a settings file (a notice).
    write_solid(dir.path(), "tiny.png", 8, GREEN);
    std::fs::write(dir.path().join("img2ico.toml"), "padding = 0\n").unwrap();
    let out = img2ico(
        dir.path(),
        &[
            "tiny.png",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16",
            "-c",
            "auto",
        ],
    );
    assert_success(&out);
    let icons = ico_in(&out.stdout);
    assert_eq!(icons.entries().len(), 1, "exactly the icon");
    let errors = stderr(&out);
    assert!(errors.contains("Using settings from"), "{}", describe(&out));
    assert!(
        errors.contains("Detected background color"),
        "{}",
        describe(&out)
    );
    assert!(errors.contains("Warning:"), "{}", describe(&out));
}

#[test]
fn settings_and_reports_can_still_be_written_when_the_icon_goes_to_standard_output() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let out = img2ico(
        dir.path(),
        &[
            "logo.png",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16",
            "--out-toml",
            "snap.toml",
            "--report",
            "r.csv",
        ],
    );
    assert_success(&out);
    ico_in(&out.stdout);
    assert!(dir.path().join("snap.toml").is_file());
    assert!(dir.path().join("r.csv").is_file());
    assert!(
        stderr(&out).contains("Settings written to"),
        "{}",
        describe(&out)
    );
}

#[test]
fn an_icns_file_can_go_to_standard_output_too() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let out = img2ico(
        dir.path(),
        &["logo.png", "-o", "-", "--output-format", "icns"],
    );
    assert_success(&out);
    assert_eq!(&out.stdout[..4], b"icns");
    let declared = u32::from_be_bytes(out.stdout[4..8].try_into().unwrap()) as usize;
    assert_eq!(
        declared,
        out.stdout.len(),
        "the declared length is the real length"
    );
}

#[test]
fn a_gif_frame_can_be_picked_from_standard_input() {
    let dir = tempfile::tempdir().unwrap();
    let gif = std::fs::read(write_gif(dir.path(), "a.gif", &[[255, 0, 0], [0, 255, 0]])).unwrap();
    let out = img2ico_with_stdin(
        dir.path(),
        &[
            "-",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16",
            "--gif-frame",
            "2",
        ],
        &gif,
    );
    assert_success(&out);
    let image = ico_in(&out.stdout).entries()[0].decode().unwrap();
    assert_eq!(
        &image.rgba_data()[..4],
        &[0, 255, 0, 255],
        "frame 2 is green"
    );

    let too_far = img2ico_with_stdin(dir.path(), &["-", "-o", "-", "--gif-frame", "9"], &gif);
    assert_failure_containing(&too_far, "out of range");
    assert!(too_far.stdout.is_empty());
}

#[test]
fn the_format_of_standard_input_is_detected_from_its_content() {
    let dir = tempfile::tempdir().unwrap();
    write_jpeg(dir.path(), "photo.jpg");
    let jpeg = std::fs::read(dir.path().join("photo.jpg")).unwrap();
    let out = img2ico_with_stdin(
        dir.path(),
        &["-", "-o", "-", "--output-format", "ico", "--sizes", "16"],
        &jpeg,
    );
    assert_success(&out);
    ico_in(&out.stdout);
}

#[test]
fn bad_standard_input_is_an_error_and_standard_output_stays_empty() {
    let dir = tempfile::tempdir().unwrap();
    let junk = img2ico_with_stdin(dir.path(), &["-", "-o", "-"], b"this is not an image");
    assert_failure_containing(&junk, "Could not read input file");
    assert!(junk.stdout.is_empty(), "{}", describe(&junk));
    // On Windows the message also points at the PowerShell pitfall.
    assert_eq!(
        stderr(&junk).contains("Windows PowerShell"),
        cfg!(windows),
        "{}",
        describe(&junk)
    );

    let empty = img2ico_with_stdin(dir.path(), &["-", "-o", "-"], b"");
    assert_failure_containing(&empty, "Standard input is empty");
    assert!(empty.stdout.is_empty());
}

#[test]
fn standard_input_needs_an_output_and_cannot_be_mixed_or_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let png = std::fs::read(write_solid(dir.path(), "logo.png", 64, RED)).unwrap();

    let no_output = img2ico_with_stdin(dir.path(), &["-"], &png);
    assert_failure_containing(&no_output, "needs -o");

    let mixed = img2ico_with_stdin(dir.path(), &["-", "logo.png", "-o", "out"], &png);
    assert_failure_containing(&mixed, "can't be combined with other inputs");

    let delete = img2ico_with_stdin(dir.path(), &["-", "-o", "x.ico", "--delete-source"], &png);
    assert_failure_containing(&delete, "--delete-source can't be used with standard input");
    assert!(!dir.path().join("x.ico").exists());
}

#[test]
fn standard_output_takes_one_image_and_no_deletion() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    std::fs::create_dir(dir.path().join("folder")).unwrap();
    write_solid(&dir.path().join("folder"), "c.png", 64, RED);

    let several = img2ico(dir.path(), &["a.png", "b.png", "-o", "-"]);
    assert_failure_containing(&several, "takes one icon");
    assert!(several.stdout.is_empty());
    let folder = img2ico(dir.path(), &["folder", "-o", "-"]);
    assert_failure_containing(&folder, "takes one icon");

    let delete = img2ico(dir.path(), &["a.png", "-o", "-", "--delete-source"]);
    assert_failure_containing(&delete, "--delete-source can't be used with '-o -'");
    assert!(dir.path().join("a.png").is_file());

    let find = img2ico(dir.path(), &["a.png", "-o", "-", "--find", "FF0000"]);
    assert_failure_containing(&find, "nothing to send to standard output");
}

#[test]
fn a_file_called_dash_is_a_file_when_written_with_a_path() {
    let dir = tempfile::tempdir().unwrap();
    let gif = write_gif(dir.path(), "a.gif", &[[255, 0, 0]]);
    std::fs::copy(&gif, dir.path().join("-")).unwrap();
    // './-' is the file (its format is recognized by content, not by a file
    // extension - there is none); a plain '-' would be standard input, which
    // is empty here.
    let out = convert(dir.path(), &["./-", "-o", "x.ico", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("x.ico")), vec![16]);

    let stdin = convert(dir.path(), &["-", "-o", "y.ico"]);
    assert_failure_containing(&stdin, "Standard input is empty");
}

#[test]
fn an_image_is_recognized_by_its_content_not_its_file_extension() {
    let dir = tempfile::tempdir().unwrap();
    let png = write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::copy(&png, dir.path().join("no-extension")).unwrap();
    std::fs::copy(&png, dir.path().join("wrong.jpg")).unwrap();
    for name in ["no-extension", "wrong.jpg"] {
        let out = convert(
            dir.path(),
            &[name, "-o", "x.ico", "--sizes", "16", "--force"],
        );
        assert_success(&out);
        assert_eq!(ico_sizes(&dir.path().join("x.ico")), vec![16], "{name}");
    }
}

#[test]
fn skip_existing_and_force_do_not_look_for_a_file_called_dash_when_writing_to_standard_output() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    std::fs::write(dir.path().join("-"), b"in the way").unwrap();
    let out = img2ico(
        dir.path(),
        &[
            "logo.png",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16",
            "--skip-existing",
        ],
    );
    assert_success(&out);
    ico_in(&out.stdout);
    let plain = img2ico(
        dir.path(),
        &[
            "logo.png",
            "-o",
            "-",
            "--output-format",
            "ico",
            "--sizes",
            "16",
        ],
    );
    assert_success(&plain);
    assert_eq!(
        std::fs::read(dir.path().join("-")).unwrap(),
        b"in the way",
        "the file called '-' is untouched"
    );
}

#[test]
fn what_if_works_with_standard_input_and_output_without_reading_or_writing() {
    let dir = tempfile::tempdir().unwrap();
    let out = img2ico(
        dir.path(),
        &[
            "-",
            "-o",
            "out.ico",
            "--what-if",
            "--output-format",
            "ico",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert!(
        stdout(&out).contains("- -> out.ico (would convert, sizes [16])"),
        "{}",
        describe(&out)
    );
    let to_stdout = img2ico(dir.path(), &["-", "-o", "-", "--what-if"]);
    assert_success(&to_stdout);
    assert!(
        stdout(&to_stdout).contains("- -> - (would convert"),
        "{}",
        describe(&to_stdout)
    );
}

#[test]
fn a_find_preview_can_read_standard_input() {
    let dir = tempfile::tempdir().unwrap();
    let png = std::fs::read(write_enclosed_patch(dir.path(), "patch.png")).unwrap();
    let out = img2ico_with_stdin(dir.path(), &["-", "--find", "00FF00"], &png);
    assert_success(&out);
    assert!(stdout(&out).contains("--seed"), "{}", describe(&out));
}

#[test]
fn a_report_names_standard_input_as_a_dash() {
    let dir = tempfile::tempdir().unwrap();
    let png = std::fs::read(write_solid(dir.path(), "logo.png", 64, RED)).unwrap();
    let out = img2ico_with_stdin(
        dir.path(),
        &["-", "-o", "x.ico", "--sizes", "16", "--report", "r.csv"],
        &png,
    );
    assert_success(&out);
    let csv = std::fs::read_to_string(dir.path().join("r.csv")).unwrap();
    assert!(
        csv.lines()
            .nth(1)
            .unwrap()
            .starts_with("-,x.ico,converted,"),
        "{csv}"
    );
}

#[test]
fn quiet_hides_the_success_line_but_not_warnings_or_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "tiny.png", 8, RED);
    let out = convert(dir.path(), &["tiny.png", "--sizes", "16", "--quiet"]);
    assert_success(&out);
    assert_eq!(stdout(&out), "", "{}", describe(&out));
    assert!(stderr(&out).contains("Warning:"), "{}", describe(&out));
    assert!(dir.path().join("tiny.ico").is_file());

    let failing = convert(dir.path(), &["nope.png", "-q"]);
    assert_failure_containing(&failing, "Could not read");
}

#[test]
fn quiet_and_silent_together_leave_only_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "tiny.png", 8, RED);
    let out = convert(
        dir.path(),
        &["tiny.png", "--sizes", "16", "--quiet", "--silent"],
    );
    assert_success(&out);
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), "");
}

#[test]
fn quiet_silences_a_batch_but_the_exit_code_and_errors_remain() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "bad.png",
            "--sizes",
            "16",
            "--keep-going",
            "--quiet",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", describe(&out));
    assert_eq!(
        stdout(&out),
        "",
        "no progress, no summary: {}",
        describe(&out)
    );
    assert!(
        stderr(&out).contains("Error: bad.png"),
        "{}",
        describe(&out)
    );
    assert!(dir.path().join("a.ico").is_file());
}

#[test]
fn quiet_does_not_hide_what_a_mode_exists_to_report() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let inspect = img2ico(dir.path(), &["--inspect", "logo.png", "--quiet"]);
    assert_success(&inspect);
    assert!(
        stdout(&inspect).contains("source image"),
        "{}",
        describe(&inspect)
    );

    let find = img2ico(dir.path(), &["--find", "FF0000", "logo.png", "-q"]);
    assert_success(&find);
    assert!(!stdout(&find).is_empty(), "{}", describe(&find));

    let what_if = convert(dir.path(), &["logo.png", "--what-if", "-q"]);
    assert_success(&what_if);
    assert!(
        stdout(&what_if).contains("would convert"),
        "{}",
        describe(&what_if)
    );
}

#[test]
fn quiet_also_covers_the_other_modes_success_lines() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let out = img2ico(
        dir.path(),
        &["--merge", "a.ico", "b.ico", "-o", "m.ico", "-q"],
    );
    assert_success(&out);
    assert_eq!(stdout(&out), "", "{}", describe(&out));
    assert!(dir.path().join("m.ico").is_file());
}

// =============================================================================
// More source formats: WebP, TIFF, TGA and ICNS
// =============================================================================

/// A 32x32 image, red on the left half and green on the right, saved in the
/// format the extension of `name` names.
fn write_two_color(dir: &Path, name: &str) -> PathBuf {
    let img = RgbaImage::from_fn(32, 32, |x, _| if x < 16 { Rgba(RED) } else { Rgba(GREEN) });
    let path = dir.join(name);
    img.save(&path).unwrap();
    path
}

/// The color of the icon's pixel at (x, y) in the `size`x`size` entry.
fn icon_pixel(path: &Path, size: u32, x: u32, y: u32) -> [u8; 4] {
    icon_image(path, size).get_pixel(x, y).0
}

#[test]
fn webp_tiff_and_tga_sources_are_converted() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["logo.webp", "logo.tiff", "logo.tif", "logo.tga"] {
        write_two_color(dir.path(), name);
        let out = convert(
            dir.path(),
            &[name, "-o", "out.ico", "--sizes", "32", "--force"],
        );
        assert_success(&out);
        assert_eq!(ico_sizes(&dir.path().join("out.ico")), vec![32], "{name}");
        assert_eq!(
            icon_pixel(&dir.path().join("out.ico"), 32, 2, 16),
            RED,
            "{name}"
        );
        assert_eq!(
            icon_pixel(&dir.path().join("out.ico"), 32, 29, 16),
            GREEN,
            "{name}"
        );
    }
}

#[test]
fn an_icns_file_is_converted_using_its_largest_icon() {
    let dir = tempfile::tempdir().unwrap();
    write_two_color(dir.path(), "logo.png");
    assert_success(&img2ico(
        dir.path(),
        &["logo.png", "--output-format", "icns", "-o", "AppIcon.icns"],
    ));
    // ... and back to an .ico, as macOS icon -> Windows icon.
    let out = convert(
        dir.path(),
        &["AppIcon.icns", "-o", "back.ico", "--sizes", "64"],
    );
    assert_success(&out);
    assert!(
        !stderr(&out).contains("upscaled"),
        "the largest icon (1024 px) is the source: {}",
        describe(&out)
    );
    assert_eq!(icon_pixel(&dir.path().join("back.ico"), 64, 4, 32), RED);
    assert_eq!(icon_pixel(&dir.path().join("back.ico"), 64, 59, 32), GREEN);
}

#[test]
fn the_default_output_name_of_an_icns_is_a_different_file_with_ico_output() {
    let dir = tempfile::tempdir().unwrap();
    write_two_color(dir.path(), "logo.png");
    assert_success(&img2ico(
        dir.path(),
        &["logo.png", "--output-format", "icns", "-o", "AppIcon.icns"],
    ));
    let out = convert(dir.path(), &["AppIcon.icns", "--sizes", "16"]);
    assert_success(&out);
    assert!(dir.path().join("AppIcon.ico").is_file());
}

#[test]
fn an_icon_is_never_written_over_the_image_it_is_made_from() {
    let dir = tempfile::tempdir().unwrap();
    write_two_color(dir.path(), "logo.png");
    assert_success(&img2ico(
        dir.path(),
        &["logo.png", "--output-format", "icns", "-o", "AppIcon.icns"],
    ));
    let before = std::fs::read(dir.path().join("AppIcon.icns")).unwrap();

    // .icns -> .icns next to itself: the default output name is the input's own.
    for extra in [&[][..], &["--force"], &["--force", "--delete-source"]] {
        let mut args = vec!["AppIcon.icns", "--output-format", "icns"];
        args.extend_from_slice(extra);
        let out = img2ico(dir.path(), &args);
        assert_failure_containing(&out, "is the input file itself");
    }
    assert_eq!(
        std::fs::read(dir.path().join("AppIcon.icns")).unwrap(),
        before
    );

    // In a batch the whole run is refused before anything is written.
    let folder = dir.path().join("assets");
    std::fs::create_dir(&folder).unwrap();
    std::fs::copy(dir.path().join("AppIcon.icns"), folder.join("AppIcon.icns")).unwrap();
    write_solid(&folder, "other.png", 64, RED);
    let out = img2ico(
        dir.path(),
        &["assets", "--output-format", "icns", "--force"],
    );
    assert_failure_containing(&out, "is itself one of the input files");
    assert_eq!(names_in(&folder), vec!["AppIcon.icns", "other.png"]);
}

#[test]
fn a_folder_takes_all_the_supported_formats_and_ignores_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    for name in ["a.png", "b.webp", "c.tiff", "d.TIF", "e.tga"] {
        write_two_color(&assets, name);
    }
    write_two_color(dir.path(), "src.png");
    assert_success(&img2ico(
        dir.path(),
        &["src.png", "--output-format", "icns", "-o", "assets/f.icns"],
    ));
    write_junk(&assets, "notes.txt");
    write_junk(&assets, "design.psd");
    write_junk(&assets, "photo.jxl");

    let out = convert(dir.path(), &["assets", "-o", "out", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a.ico", "b.ico", "c.ico", "d.ico", "e.ico", "f.ico"]
    );
    assert!(stdout(&out).contains("6 converted"), "{}", describe(&out));
}

#[test]
fn the_empty_folder_message_lists_the_supported_formats() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("empty")).unwrap();
    write_junk(&dir.path().join("empty"), "notes.txt");
    assert_failure_containing(
        &convert(dir.path(), &["empty"]),
        "PNG, JPG, BMP, GIF, WebP, TIFF, TGA, ICNS",
    );
}

#[test]
fn webp_and_tiff_work_from_standard_input_but_tga_cannot() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["logo.webp", "logo.tiff"] {
        let bytes = std::fs::read(write_two_color(dir.path(), name)).unwrap();
        let out = img2ico_with_stdin(
            dir.path(),
            &["-", "-o", "-", "--output-format", "ico", "--sizes", "16"],
            &bytes,
        );
        assert_success(&out);
        ico_in(&out.stdout);
    }
    // A TGA has no recognizable header, so without a file name it is unreadable.
    let tga = std::fs::read(write_two_color(dir.path(), "logo.tga")).unwrap();
    let out = img2ico_with_stdin(dir.path(), &["-", "-o", "-"], &tga);
    assert_failure_containing(&out, "Could not read input file");
    assert!(out.stdout.is_empty());
}

#[test]
fn inspect_reports_the_new_formats_as_source_images() {
    let dir = tempfile::tempdir().unwrap();
    write_two_color(dir.path(), "logo.webp");
    write_two_color(dir.path(), "logo.tiff");
    let out = img2ico(dir.path(), &["--inspect", "logo.webp", "logo.tiff"]);
    assert_success(&out);
    assert_eq!(
        stdout(&out).matches("(source image, 32x32)").count(),
        2,
        "{}",
        describe(&out)
    );

    let json = img2ico(dir.path(), &["--inspect", "logo.webp", "--json"]);
    assert_success(&json);
    let report: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(report[0]["kind"], "image");
    assert_eq!(report[0]["width"], 32);
}

#[test]
fn a_broken_file_of_a_new_format_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["bad.webp", "bad.tiff", "bad.icns"] {
        write_junk(dir.path(), name);
        let out = convert(dir.path(), &[name]);
        assert_failure_containing(&out, "Could not read");
    }
    assert!(names_in(dir.path()).iter().all(|n| !n.ends_with(".ico")));
}

// =============================================================================
// SVG as a source
// =============================================================================

/// 100x50 units: a red left half and a green right half.
const TWO_SQUARES_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="50" height="50" fill="#ff0000"/><rect x="50" width="50" height="50" fill="#00ff00"/></svg>"##;

fn write_svg(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, TWO_SQUARES_SVG).unwrap();
    path
}

#[test]
fn an_svg_is_rendered_at_every_size() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    let out = convert(
        dir.path(),
        &["logo.svg", "-o", "out.ico", "--sizes", "16,64,256"],
    );
    assert_success(&out);
    let ico = dir.path().join("out.ico");
    assert_eq!(ico_sizes(&ico), vec![16, 64, 256]);
    for size in [16, 64, 256] {
        // The 2:1 drawing is fitted into the square, centered: red left,
        // green right, transparent above and below.
        assert_eq!(icon_pixel(&ico, size, size / 8, size / 2), RED, "{size}");
        assert_eq!(
            icon_pixel(&ico, size, size - 1 - size / 8, size / 2),
            GREEN,
            "{size}"
        );
        assert_eq!(icon_pixel(&ico, size, size / 2, 0)[3], 0, "{size}");
    }
}

#[test]
fn an_svg_never_triggers_the_upscaling_warning() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    let out = convert(dir.path(), &["logo.svg", "--sizes", "256"]);
    assert_success(&out);
    assert!(!stderr(&out).contains("upscal"), "{}", describe(&out));
}

#[test]
fn an_svg_makes_an_icns_and_works_from_standard_input() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    assert_success(&img2ico(
        dir.path(),
        &["logo.svg", "--output-format", "icns", "-o", "logo.icns"],
    ));
    assert!(dir.path().join("logo.icns").is_file());

    let out = img2ico_with_stdin(
        dir.path(),
        &["-", "-o", "-", "--output-format", "ico", "--sizes", "32"],
        TWO_SQUARES_SVG.as_bytes(),
    );
    assert_success(&out);
    ico_in(&out.stdout);
}

#[test]
fn a_compressed_svgz_works_and_the_format_is_found_by_content() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(TWO_SQUARES_SVG.as_bytes()).unwrap();
    std::fs::write(dir.path().join("logo.svgz"), encoder.finish().unwrap()).unwrap();
    // No extension at all: found by content.
    std::fs::write(dir.path().join("drawing"), TWO_SQUARES_SVG).unwrap();
    for name in ["logo.svgz", "drawing"] {
        let out = convert(
            dir.path(),
            &[name, "-o", "out.ico", "--sizes", "32", "--force"],
        );
        assert_success(&out);
        assert_eq!(
            icon_pixel(&dir.path().join("out.ico"), 32, 4, 16),
            RED,
            "{name}"
        );
    }
}

#[test]
fn a_folder_takes_svg_files_along_with_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    write_svg(&assets, "a.svg");
    write_solid(&assets, "b.png", 64, RED);
    let out = convert(dir.path(), &["assets", "-o", "out", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(names_in(&dir.path().join("out")), vec!["a.ico", "b.ico"]);
}

/// A 64x64 drawing with a white background and a red square in the middle.
const WHITE_BACKGROUND_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#ffffff"/><rect x="16" y="16" width="32" height="32" fill="#ff0000"/></svg>"##;

#[test]
fn a_background_color_is_removed_from_every_rendered_size_of_an_svg() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.svg"), WHITE_BACKGROUND_SVG).unwrap();
    for color in ["ffffff", "auto"] {
        let out = convert(
            dir.path(),
            &[
                "logo.svg",
                "-c",
                color,
                "-o",
                "out.ico",
                "--sizes",
                "16,64,256",
                "--force",
            ],
        );
        assert_success(&out);
        let ico = dir.path().join("out.ico");
        for size in [16, 64, 256] {
            assert_eq!(
                icon_pixel(&ico, size, 1, 1)[3],
                0,
                "{color} {size}: corner is clear"
            );
            assert_eq!(
                icon_pixel(&ico, size, size / 2, size / 2),
                RED,
                "{color} {size}"
            );
        }
        if color == "auto" {
            assert!(
                stdout(&out).contains("Detected background color #FFFFFF"),
                "{}",
                describe(&out)
            );
        }
    }
}

#[test]
fn the_background_of_an_svg_can_be_replaced_and_is_checked_once() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.svg"), WHITE_BACKGROUND_SVG).unwrap();
    let out = convert(
        dir.path(),
        &[
            "logo.svg",
            "-c",
            "ffffff",
            "--replace-color",
            "0000ff",
            "--sizes",
            "16,32,64",
        ],
    );
    assert_success(&out);
    assert_eq!(
        icon_pixel(&dir.path().join("logo.ico"), 32, 1, 1),
        [0, 0, 255, 255]
    );

    // A color that is not there: one warning, not one per size.
    let out = convert(
        dir.path(),
        &["logo.svg", "-c", "00ff00", "--sizes", "16,32,64", "--force"],
    );
    assert_success(&out);
    assert_eq!(
        stderr(&out).matches("nothing was removed").count(),
        1,
        "{}",
        describe(&out)
    );
}

#[test]
fn pixel_position_options_are_refused_for_an_svg() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    for args in [
        &["--find", "auto"][..],
        &["--chroma-key", "ffffff", "--seed", "0,0"],
    ] {
        let mut all = vec!["logo.svg", "--sizes", "16"];
        all.extend_from_slice(args);
        let out = convert(dir.path(), &all);
        assert_failure_containing(&out, "does not apply to an SVG");
    }
    assert!(!dir.path().join("logo.ico").exists());
}

#[test]
fn grayscale_applies_to_every_rendered_size_of_an_svg() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    assert_success(&convert(
        dir.path(),
        &["logo.svg", "--grayscale", "-o", "g.ico", "--sizes", "32"],
    ));
    let pixel = icon_pixel(&dir.path().join("g.ico"), 32, 4, 16);
    assert_eq!(pixel[0], pixel[1]);
    assert_eq!(pixel[1], pixel[2]);
    assert_eq!(pixel[3], 255);
}

#[test]
fn inspect_reports_an_svg_as_a_vector_image() {
    let dir = tempfile::tempdir().unwrap();
    write_svg(dir.path(), "logo.svg");
    let out = img2ico(dir.path(), &["--inspect", "logo.svg"]);
    assert_success(&out);
    assert!(stdout(&out).contains("vector image"), "{}", describe(&out));

    let json = img2ico(dir.path(), &["--inspect", "logo.svg", "--json"]);
    assert_success(&json);
    let report: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(report[0]["kind"], "vector");
    assert_eq!(report[0]["width"], 100.0);
}

#[test]
fn a_broken_svg_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("bad.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect",
    )
    .unwrap();
    let out = convert(dir.path(), &["bad.svg"]);
    assert_failure_containing(&out, "Could not read");
    assert!(!dir.path().join("bad.ico").exists());
}

// =============================================================================
// Layout: --fit, --crop, --trim, --corner-radius and --sizes auto
// =============================================================================

/// 40x20: red left half, green right half, fully opaque.
fn write_wide(dir: &Path, name: &str) -> PathBuf {
    write_image(dir, name, 40, 20, |x, _| if x < 20 { RED } else { GREEN })
}

/// 64x64, transparent except for a red 16x16 square at (24, 8).
fn write_floating_square(dir: &Path, name: &str) -> PathBuf {
    write_image(dir, name, 64, 64, |x, y| {
        if (24..40).contains(&x) && (8..24).contains(&y) {
            RED
        } else {
            [0, 0, 0, 0]
        }
    })
}

#[test]
fn fit_contain_is_the_default_and_leaves_bars() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    for extra in [&[][..], &["--fit", "contain"]] {
        let mut args = vec!["wide.png", "-o", "out.ico", "--sizes", "32", "--force"];
        args.extend_from_slice(extra);
        assert_success(&convert(dir.path(), &args));
        let ico = dir.path().join("out.ico");
        assert_eq!(icon_pixel(&ico, 32, 16, 1)[3], 0, "bar above");
        assert_eq!(icon_pixel(&ico, 32, 4, 16), RED);
    }
}

#[test]
fn fit_cover_fills_the_square_and_keeps_the_middle() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    assert_success(&convert(
        dir.path(),
        &[
            "wide.png", "--fit", "cover", "-o", "out.ico", "--sizes", "32",
        ],
    ));
    let ico = dir.path().join("out.ico");
    for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
        assert_eq!(icon_pixel(&ico, 32, x, y)[3], 255, "({x},{y})");
    }
    assert_eq!(icon_pixel(&ico, 32, 4, 16), RED);
    assert_eq!(icon_pixel(&ico, 32, 27, 16), GREEN);
}

#[test]
fn an_unknown_fit_is_refused_by_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    let out = convert(dir.path(), &["wide.png", "--fit", "stretch"]);
    assert_failure_containing(&out, "invalid value");
}

#[test]
fn crop_takes_the_given_part_of_the_image() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    // The right 20x20: all green.
    assert_success(&convert(
        dir.path(),
        &[
            "wide.png",
            "--crop",
            "20,0,20,20",
            "-o",
            "out.ico",
            "--sizes",
            "16",
        ],
    ));
    let ico = dir.path().join("out.ico");
    assert_eq!(icon_pixel(&ico, 16, 2, 8), GREEN);
    assert_eq!(icon_pixel(&ico, 16, 13, 8), GREEN);
    assert_eq!(icon_pixel(&ico, 16, 8, 0)[3], 255, "now square: no bars");
}

#[test]
fn a_bad_crop_is_an_error_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    for bad in ["1,2,3", "a,b,c,d", "0,0,0,5"] {
        let out = convert(dir.path(), &["wide.png", "--crop", bad]);
        assert_failure_containing(&out, "--crop");
    }
    let out = convert(dir.path(), &["wide.png", "--crop", "30,0,20,10"]);
    assert_failure_containing(&out, "reaches outside the image, which is 40x20 pixels");
    assert!(!dir.path().join("wide.ico").exists());
}

#[test]
fn trim_cuts_the_transparent_margin_so_the_artwork_fills_the_icon() {
    let dir = tempfile::tempdir().unwrap();
    write_floating_square(dir.path(), "logo.png");
    // Without --trim the 16x16 square floats in a 64x64 canvas.
    assert_success(&convert(
        dir.path(),
        &["logo.png", "-o", "plain.ico", "--sizes", "64"],
    ));
    assert_eq!(icon_pixel(&dir.path().join("plain.ico"), 64, 2, 2)[3], 0);

    assert_success(&convert(
        dir.path(),
        &["logo.png", "--trim", "-o", "trim.ico", "--sizes", "64"],
    ));
    let ico = dir.path().join("trim.ico");
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63), (32, 32)] {
        assert_eq!(icon_pixel(&ico, 64, x, y), RED, "({x},{y})");
    }
}

#[test]
fn trim_after_a_background_removal_cuts_the_removed_background_too() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "logo.png");
    assert_success(&convert(
        dir.path(),
        &[
            "logo.png", "-c", "00ff00", "--trim", "-o", "out.ico", "--sizes", "16",
        ],
    ));
    let ico = dir.path().join("out.ico");
    assert_eq!(icon_pixel(&ico, 16, 0, 0), RED);
    assert_eq!(icon_pixel(&ico, 16, 15, 15), RED);
}

#[test]
fn trim_without_a_transparent_margin_warns_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    let out = convert(
        dir.path(),
        &["wide.png", "--trim", "-o", "out.ico", "--sizes", "32"],
    );
    assert_success(&out);
    assert!(
        stderr(&out).contains("--trim found no transparent margin"),
        "{}",
        describe(&out)
    );
    assert_eq!(icon_pixel(&dir.path().join("out.ico"), 32, 4, 16), RED);
}

#[test]
fn trim_of_a_completely_transparent_image_warns() {
    let dir = tempfile::tempdir().unwrap();
    write_image(dir.path(), "empty.png", 8, 8, |_, _| [0, 0, 0, 0]);
    let out = convert(dir.path(), &["empty.png", "--trim", "--sizes", "16"]);
    assert_success(&out);
    assert!(
        stderr(&out).contains("nothing to keep"),
        "{}",
        describe(&out)
    );
}

#[test]
fn corner_radius_rounds_the_corners_of_every_size() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "red.png", 128, RED);
    assert_success(&convert(
        dir.path(),
        &[
            "red.png",
            "--corner-radius",
            "25",
            "-o",
            "out.ico",
            "--sizes",
            "16,64,256",
        ],
    ));
    let ico = dir.path().join("out.ico");
    for size in [16, 64, 256] {
        assert_eq!(icon_pixel(&ico, size, 0, 0)[3], 0, "{size}: corner");
        assert_eq!(
            icon_pixel(&ico, size, size - 1, size - 1)[3],
            0,
            "{size}: corner"
        );
        assert_eq!(
            icon_pixel(&ico, size, size / 2, 0)[3],
            255,
            "{size}: edge middle"
        );
        assert_eq!(icon_pixel(&ico, size, size / 2, size / 2), RED, "{size}");
    }
}

#[test]
fn a_corner_radius_beyond_fifty_percent_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "red.png", 16, RED);
    let out = convert(dir.path(), &["red.png", "--corner-radius", "51"]);
    assert_failure_containing(&out, "corner-radius");
}

#[test]
fn corner_radius_and_padding_round_the_image_inside_its_margin() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "red.png", 100, RED);
    assert_success(&convert(
        dir.path(),
        &[
            "red.png",
            "--corner-radius",
            "50",
            "--padding",
            "20",
            "-o",
            "out.ico",
            "--sizes",
            "100",
        ],
    ));
    let ico = dir.path().join("out.ico");
    assert_eq!(icon_pixel(&ico, 100, 50, 50), RED);
    assert_eq!(icon_pixel(&ico, 100, 50, 12)[3], 255, "top of the circle");
    assert_eq!(icon_pixel(&ico, 100, 14, 14)[3], 0, "outside the circle");
    assert_eq!(icon_pixel(&ico, 100, 5, 50)[3], 0, "padding");
}

#[test]
fn sizes_auto_leaves_out_what_the_source_cannot_supply() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "small.png", 100, RED);
    let out = convert(
        dir.path(),
        &["small.png", "--sizes", "auto", "-o", "out.ico"],
    );
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("out.ico")), vec![16, 32, 48, 64]);
    assert!(!stderr(&out).contains("upscaled"), "{}", describe(&out));

    write_solid(dir.path(), "big.png", 512, RED);
    assert_success(&convert(
        dir.path(),
        &["big.png", "--sizes", "AUTO", "-o", "big.ico"],
    ));
    assert_eq!(
        ico_sizes(&dir.path().join("big.ico")),
        vec![16, 32, 48, 64, 128, 256]
    );
}

#[test]
fn sizes_auto_follows_crop_and_trim_and_handles_a_tiny_source() {
    let dir = tempfile::tempdir().unwrap();
    write_floating_square(dir.path(), "logo.png");
    // After --trim the artwork is 16x16: sizes up to 16 only.
    assert_success(&convert(
        dir.path(),
        &["logo.png", "--trim", "--sizes", "auto", "-o", "a.ico"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("a.ico")), vec![16]);

    write_solid(dir.path(), "tiny.png", 10, RED);
    assert_success(&convert(
        dir.path(),
        &["tiny.png", "--sizes", "auto", "-o", "b.ico"],
    ));
    assert_eq!(ico_sizes(&dir.path().join("b.ico")), vec![10]);
}

#[test]
fn the_layout_options_work_for_icns_output_too() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    assert_success(&img2ico(
        dir.path(),
        &[
            "wide.png",
            "--output-format",
            "icns",
            "--fit",
            "cover",
            "--corner-radius",
            "20",
            "-o",
            "a.icns",
        ],
    ));
    assert!(dir.path().join("a.icns").is_file());
}

#[test]
fn the_layout_options_can_come_from_a_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    write_wide(dir.path(), "wide.png");
    std::fs::write(
        dir.path().join("img2ico.toml"),
        "fit = \"cover\"\ncrop = \"0,0,20,20\"\ntrim = true\ncorner-radius = 10\nsizes = \"auto\"\n",
    )
    .unwrap();
    let out = img2ico(
        dir.path(),
        &[
            "wide.png",
            "--output-format",
            "ico",
            "-o",
            "out.ico",
            "--out-toml",
            "saved.toml",
        ],
    );
    assert_success(&out);
    let saved = std::fs::read_to_string(dir.path().join("saved.toml")).unwrap();
    for line in [
        "fit = \"cover\"",
        "crop = \"0,0,20,20\"",
        "trim = true",
        "corner-radius = 10",
        "sizes = \"auto\"",
    ] {
        assert!(saved.contains(line), "{line} missing in:\n{saved}");
    }
    // The command line wins over the file.
    let out = img2ico(
        dir.path(),
        &[
            "wide.png",
            "--output-format",
            "ico",
            "-o",
            "again.ico",
            "--fit",
            "contain",
            "--sizes",
            "16",
            "--force",
        ],
    );
    assert_success(&out);
}

// --- SVG ----------------------------------------------------------------------

/// A 100x100 page with a red 20x20 square at (40, 40).
const SMALL_SQUARE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="40" y="40" width="20" height="20" fill="#ff0000"/></svg>"##;

#[test]
fn trim_fits_the_content_of_an_svg_not_its_page() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.svg"), SMALL_SQUARE_SVG).unwrap();
    assert_success(&convert(
        dir.path(),
        &["logo.svg", "-o", "plain.ico", "--sizes", "32"],
    ));
    assert_eq!(icon_pixel(&dir.path().join("plain.ico"), 32, 1, 1)[3], 0);

    assert_success(&convert(
        dir.path(),
        &["logo.svg", "--trim", "-o", "trim.ico", "--sizes", "32"],
    ));
    let ico = dir.path().join("trim.ico");
    for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
        assert_eq!(icon_pixel(&ico, 32, x, y), RED, "({x},{y})");
    }
}

#[test]
fn an_svg_takes_fit_cover_and_corner_radius_at_every_size() {
    let dir = tempfile::tempdir().unwrap();
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#ff0000"/></svg>"##;
    std::fs::write(dir.path().join("wide.svg"), svg).unwrap();
    assert_success(&convert(
        dir.path(),
        &[
            "wide.svg",
            "--fit",
            "cover",
            "--corner-radius",
            "30",
            "-o",
            "out.ico",
            "--sizes",
            "16,64",
        ],
    ));
    let ico = dir.path().join("out.ico");
    for size in [16, 64] {
        assert_eq!(icon_pixel(&ico, size, 0, 0)[3], 0, "{size}: rounded corner");
        assert_eq!(
            icon_pixel(&ico, size, size / 2, 1)[3],
            255,
            "{size}: covers the top"
        );
        assert_eq!(icon_pixel(&ico, size, size / 2, size / 2), RED, "{size}");
    }
}

#[test]
fn crop_is_refused_for_an_svg_and_sizes_auto_gives_the_default_sizes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.svg"), SMALL_SQUARE_SVG).unwrap();
    let out = convert(dir.path(), &["logo.svg", "--crop", "0,0,10,10"]);
    assert_failure_containing(&out, "--crop does not apply to an SVG");

    assert_success(&convert(
        dir.path(),
        &["logo.svg", "--sizes", "auto", "-o", "out.ico"],
    ));
    assert_eq!(
        ico_sizes(&dir.path().join("out.ico")),
        vec![16, 32, 48, 64, 128, 256]
    );
}

// =============================================================================
// Shell completions
// =============================================================================

#[test]
fn completion_scripts_are_printed_for_every_shell() {
    let dir = tempfile::tempdir().unwrap();
    for (shell, marker) in [
        ("bash", "_img2ico"),
        ("zsh", "#compdef img2ico"),
        ("fish", "complete -c img2ico"),
        ("powershell", "Register-ArgumentCompleter"),
        ("elvish", "edit:completion:arg-completer"),
    ] {
        let out = img2ico(dir.path(), &["--completions", shell]);
        assert_success(&out);
        assert!(stdout(&out).contains(marker), "{shell}: {}", describe(&out));
        assert!(
            stdout(&out).contains("no-config"),
            "{shell} script is missing a flag"
        );
        assert_eq!(stderr(&out), "", "{shell}");
    }
    assert!(names_in(dir.path()).is_empty(), "nothing may be written");
}

#[test]
fn completions_reject_other_arguments_and_unknown_shells() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 16, RED);
    let with_input = img2ico(dir.path(), &["logo.png", "--completions", "bash"]);
    assert_eq!(
        with_input.status.code(),
        Some(2),
        "{}",
        describe(&with_input)
    );
    let unknown = img2ico(dir.path(), &["--completions", "cmd"]);
    assert_eq!(unknown.status.code(), Some(2), "{}", describe(&unknown));
    assert!(
        stderr(&unknown).contains("possible values"),
        "{}",
        describe(&unknown)
    );
}

// =============================================================================
// --verbose
// =============================================================================

#[test]
fn verbose_explains_the_run_on_stderr_and_leaves_stdout_alone() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    let plain = convert(dir.path(), &["logo.png", "--sizes", "16,32", "-o", "a.ico"]);
    let verbose = convert(
        dir.path(),
        &["logo.png", "--sizes", "16,32", "-o", "b.ico", "-v"],
    );
    assert_success(&plain);
    assert_success(&verbose);

    let text = stderr(&verbose);
    for expected in [
        "verbose: effective settings",
        "verbose:   tolerance = 20",
        "verbose: source: logo.png (64x64 pixels), loaded in",
        "verbose: output: b.ico (ico)",
        "verbose: sizes: [16, 32], padding: 0%",
        "verbose: 16x16: ",
        "verbose: 32x32: ",
        "verbose: finished in",
    ] {
        assert!(
            text.contains(expected),
            "missing {expected:?}:\n{}",
            describe(&verbose)
        );
    }
    assert!(!stderr(&plain).contains("verbose:"), "{}", describe(&plain));
    assert!(
        !stdout(&verbose).contains("verbose:"),
        "{}",
        describe(&verbose)
    );
    // Only the file name in the final message differs between the two runs.
    assert_eq!(
        stdout(&verbose).replace("b.ico", "a.ico"),
        stdout(&plain),
        "standard output must be identical"
    );
}

#[test]
fn verbose_shows_which_settings_are_really_in_effect() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_user_config(dir.path(), "padding = 10\noutput-format = \"ico\"\n");
    std::fs::write(dir.path().join("img2ico.toml"), "tolerance = 35\n").unwrap();

    let out = img2ico(
        dir.path(),
        &["logo.png", "--sizes", "16", "-o", "v.ico", "-v"],
    );
    assert_success(&out);
    let text = stderr(&out);
    assert!(
        text.contains("verbose:   padding = 10"),
        "from the user file: {text}"
    );
    assert!(
        text.contains("verbose:   tolerance = 35"),
        "from the project file: {text}"
    );
    assert!(
        text.contains("verbose:   sizes = \"16\""),
        "from the command line: {text}"
    );
}

#[test]
fn verbose_reports_background_removal() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(
        dir.path(),
        &[
            "shot.png", "-c", "00FF00", "--sizes", "32", "-o", "o.ico", "-v",
        ],
    );
    assert_success(&out);
    assert!(
        stderr(&out).contains(
            "verbose: background removal (--chroma-key 00FF00, 0 seed point(s), feather 50%) took"
        ),
        "{}",
        describe(&out)
    );
}

#[test]
fn verbose_works_in_the_other_modes_too() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    let out = img2ico(dir.path(), &["--extract", "b.ico", "-o", "pngs", "-v"]);
    assert_success(&out);
    assert!(
        stderr(&out).contains("verbose: effective settings"),
        "{}",
        describe(&out)
    );
    assert!(dir.path().join("pngs").join("b_32x32.png").is_file());
}

#[test]
fn verbose_and_silent_contradict_each_other() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 16, RED);
    let out = img2ico(dir.path(), &["logo.png", "-v", "--silent"]);
    assert_eq!(out.status.code(), Some(2), "{}", describe(&out));
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        describe(&out)
    );
}

// =============================================================================
// Batch mode: several inputs, or a folder
// =============================================================================

fn write_junk(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), b"this is not an image").unwrap();
}

fn write_jpeg(dir: &Path, name: &str) {
    let img = RgbaImage::from_pixel(32, 32, Rgba([10, 20, 30, 255]));
    image::DynamicImage::ImageRgba8(img)
        .to_rgb8()
        .save(dir.join(name))
        .unwrap();
}

#[test]
fn several_files_are_converted_next_to_their_inputs() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, GREEN);

    let out = convert(dir.path(), &["a.png", "b.png", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(ico_sizes(&dir.path().join("a.ico")), vec![16]);
    assert_eq!(ico_sizes(&dir.path().join("b.ico")), vec![16]);
    let text = stdout(&out);
    assert!(
        text.contains("[1/2] a.png -> a.ico ("),
        "{}",
        describe(&out)
    );
    assert!(
        text.contains("[2/2] b.png -> b.ico ("),
        "{}",
        describe(&out)
    );
    assert!(
        text.contains("Batch finished: 2 converted, 0 skipped, 0 failed (2 file(s) in total)."),
        "{}",
        describe(&out)
    );
}

#[test]
fn a_single_file_does_not_print_a_batch_summary() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "--sizes", "16"]);
    assert_success(&out);
    assert!(
        !stdout(&out).contains("Batch finished"),
        "{}",
        describe(&out)
    );
}

#[test]
fn the_output_folder_is_created_and_receives_one_icon_per_input() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, GREEN);

    assert_success(&convert(
        dir.path(),
        &["a.png", "b.png", "-o", "icons/nested", "--sizes", "16"],
    ));
    assert_eq!(
        names_in(&dir.path().join("icons").join("nested")),
        vec!["a.ico", "b.ico"]
    );
    assert!(
        !dir.path().join("a.ico").exists(),
        "nothing next to the inputs"
    );
}

#[test]
fn a_folder_converts_the_images_directly_inside_it() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(assets.join("sub")).unwrap();
    write_solid(&assets, "a.png", 64, RED);
    write_jpeg(&assets, "b.jpg");
    write_gif(&assets, "d.gif", &[[255, 0, 0]]);
    write_solid(&assets.join("sub"), "deep.png", 64, RED); // subfolders are not searched
    write_junk(&assets, "notes.txt");

    let out = convert(dir.path(), &["assets", "-o", "out", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a.ico", "b.ico", "d.ico"]
    );
    assert!(stdout(&out).contains("3 converted"), "{}", describe(&out));
}

#[test]
fn files_are_converted_in_the_order_given_and_folders_in_name_order() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    write_solid(&assets, "m.png", 64, RED);
    write_solid(&assets, "k.png", 64, RED);
    write_solid(dir.path(), "z.png", 64, RED);

    let out = convert(dir.path(), &["z.png", "assets", "--sizes", "16"]);
    assert_success(&out);
    let text = stdout(&out);
    let z = text.find("z.png -> z.ico").unwrap();
    let k = text.find("k.png -> ").unwrap();
    let m = text.find("m.png -> ").unwrap();
    assert!(z < k && k < m, "{}", describe(&out));
}

#[test]
fn an_empty_folder_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("empty")).unwrap();
    write_junk(&dir.path().join("empty"), "notes.txt");
    assert_failure_containing(&convert(dir.path(), &["empty"]), "No supported images");
}

#[test]
fn the_same_settings_apply_to_every_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, GREEN);
    assert_success(&convert(
        dir.path(),
        &["a.png", "b.png", "--preset", "favicon", "--grayscale"],
    ));
    for name in ["a.ico", "b.ico"] {
        let path = dir.path().join(name);
        assert_eq!(ico_sizes(&path), vec![16, 32, 48], "{name}");
        let pixel = icon_image(&path, 32).get_pixel(16, 16).0;
        assert!(
            pixel[0] == pixel[1] && pixel[1] == pixel[2],
            "{name} must be gray"
        );
    }
}

#[test]
fn the_first_failure_stops_a_batch_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    write_solid(dir.path(), "c.png", 64, RED);

    let out = convert(dir.path(), &["a.png", "bad.png", "c.png", "--sizes", "16"]);
    assert_failure_containing(&out, "bad.png: Could not read input file");
    assert!(
        dir.path().join("a.ico").is_file(),
        "converted before the failure"
    );
    assert!(!dir.path().join("c.ico").exists(), "not reached");
    assert!(
        !stdout(&out).contains("Batch finished"),
        "{}",
        describe(&out)
    );
}

#[test]
fn keep_going_converts_the_rest_and_reports_every_failure() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    write_solid(dir.path(), "c.png", 64, RED);

    let out = convert(
        dir.path(),
        &["a.png", "bad.png", "c.png", "--sizes", "16", "--keep-going"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", describe(&out));
    assert!(dir.path().join("a.ico").is_file());
    assert!(
        dir.path().join("c.ico").is_file(),
        "the file after the failure is converted"
    );
    let errors = stderr(&out);
    assert!(
        errors.contains("Error: bad.png: Could not read input file"),
        "{errors}"
    );
    assert!(errors.contains("1 of 3 file(s) failed."), "{errors}");
    assert!(
        stdout(&out)
            .contains("Batch finished: 2 converted, 0 skipped, 1 failed (3 file(s) in total)."),
        "{}",
        describe(&out)
    );
}

#[test]
fn keep_going_can_come_from_a_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    write_solid(dir.path(), "c.png", 64, RED);
    std::fs::write(dir.path().join("img2ico.toml"), "keep-going = true\n").unwrap();

    let out = convert(dir.path(), &["a.png", "bad.png", "c.png", "--sizes", "16"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(dir.path().join("c.ico").is_file(), "{}", describe(&out));
}

#[test]
fn a_missing_file_in_a_batch_is_named() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "nope.png", "--sizes", "16"]);
    assert_failure_containing(&out, "Could not read 'nope.png'");
    assert!(
        !stderr(&out).contains("nope.png: Could not read 'nope.png'"),
        "the name must not be repeated: {}",
        describe(&out)
    );
}

#[test]
fn skip_existing_makes_a_batch_repeatable() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, GREEN);
    std::fs::write(dir.path().join("a.ico"), b"precious").unwrap();

    let out = convert(
        dir.path(),
        &["a.png", "b.png", "--sizes", "16", "--skip-existing"],
    );
    assert_success(&out);
    assert_eq!(
        std::fs::read(dir.path().join("a.ico")).unwrap(),
        b"precious"
    );
    assert_eq!(ico_sizes(&dir.path().join("b.ico")), vec![16]);
    assert!(
        stderr(&out).contains("Skipping 'a.png': 'a.ico' already exists (--skip-existing)."),
        "{}",
        describe(&out)
    );
    assert!(
        stdout(&out).contains("Batch finished: 1 converted, 1 skipped, 0 failed"),
        "{}",
        describe(&out)
    );

    // Running it again finds nothing left to do.
    let again = convert(
        dir.path(),
        &["a.png", "b.png", "--sizes", "16", "--skip-existing"],
    );
    assert_success(&again);
    assert!(
        stdout(&again).contains("0 converted, 2 skipped"),
        "{}",
        describe(&again)
    );
}

#[test]
fn skip_existing_notices_can_be_silenced() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    std::fs::write(dir.path().join("a.ico"), b"precious").unwrap();
    let out = convert(
        dir.path(),
        &["a.png", "--sizes", "16", "--skip-existing", "--silent"],
    );
    assert_success(&out);
    assert_eq!(stderr(&out), "");
}

#[test]
fn skip_existing_also_works_for_a_single_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    std::fs::write(dir.path().join("a.ico"), b"precious").unwrap();

    let out = convert(dir.path(), &["a.png", "--sizes", "16", "--skip-existing"]);
    assert_success(&out);
    assert_eq!(
        std::fs::read(dir.path().join("a.ico")).unwrap(),
        b"precious"
    );
    assert!(
        stderr(&out).contains("Skipping 'a.png'"),
        "{}",
        describe(&out)
    );
    assert!(!stdout(&out).contains("Batch finished"));
}

#[test]
fn skip_existing_and_force_contradict_each_other() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);

    let on_the_command_line = convert(dir.path(), &["a.png", "--skip-existing", "--force"]);
    assert_eq!(
        on_the_command_line.status.code(),
        Some(2),
        "{}",
        describe(&on_the_command_line)
    );

    std::fs::write(dir.path().join("img2ico.toml"), "force = true\n").unwrap();
    let across_files = convert(dir.path(), &["a.png", "--skip-existing"]);
    assert_failure_containing(
        &across_files,
        "--force and --skip-existing contradict each other",
    );
}

#[test]
fn outputs_that_would_collide_are_refused_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_jpeg(dir.path(), "logo.jpg");
    write_solid(dir.path(), "other.png", 64, RED);

    let out = convert(dir.path(), &["other.png", "logo.png", "logo.jpg"]);
    assert_failure_containing(&out, "would both be written to");
    assert!(stderr(&out).contains("logo.png") && stderr(&out).contains("logo.jpg"));
    assert!(
        !dir.path().join("other.ico").exists(),
        "no file may be written when the plan is invalid"
    );
}

#[test]
fn in_a_batch_output_names_a_folder_not_a_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    std::fs::write(dir.path().join("taken"), b"a file").unwrap();

    let looks_like_a_file = convert(dir.path(), &["a.png", "b.png", "-o", "out.ico"]);
    assert_failure_containing(&looks_like_a_file, "looks like a file name");
    let is_a_file = convert(dir.path(), &["a.png", "b.png", "-o", "taken"]);
    assert_failure_containing(&is_a_file, "is a file");
    assert!(!dir.path().join("out.ico").exists());
}

#[test]
fn a_find_preview_is_not_available_for_a_batch_but_auto_apply_is() {
    let dir = tempfile::tempdir().unwrap();
    write_enclosed_patch(dir.path(), "p1.png");
    write_enclosed_patch(dir.path(), "p2.png");

    let preview = convert(dir.path(), &["p1.png", "p2.png", "--find", "00FF00"]);
    assert_failure_containing(&preview, "--find without --auto-apply");

    let applied = convert(
        dir.path(),
        &[
            "p1.png",
            "p2.png",
            "--find",
            "00FF00",
            "--auto-apply",
            "--sizes",
            "30",
            "-o",
            "out",
        ],
    );
    assert_success(&applied);
    let text = stdout(&applied);
    assert!(
        text.contains("p1.png: Found 1 additional region(s)"),
        "{}",
        describe(&applied)
    );
    assert!(
        text.contains("p2.png: Found 1 additional region(s)"),
        "{}",
        describe(&applied)
    );
    for name in ["p1.ico", "p2.ico"] {
        let icon = icon_image(&dir.path().join("out").join(name), 30);
        assert_eq!(
            icon.get_pixel(12, 12)[3],
            0,
            "{name}: the enclosed patch is removed"
        );
    }
}

#[test]
fn warnings_name_the_file_in_a_batch_but_not_for_a_single_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "tiny.png", 16, RED);
    write_solid(dir.path(), "also-tiny.png", 16, RED);

    let single = convert(
        dir.path(),
        &["tiny.png", "--sizes", "16,256", "-o", "single.ico"],
    );
    assert_success(&single);
    assert!(
        stderr(&single).contains("Warning: the source image is 16x16"),
        "{}",
        describe(&single)
    );

    let batch = convert(
        dir.path(),
        &[
            "tiny.png",
            "also-tiny.png",
            "--sizes",
            "16,256",
            "-o",
            "out",
        ],
    );
    assert_success(&batch);
    let text = stderr(&batch);
    assert!(
        text.contains("Warning: tiny.png: the source image is 16x16"),
        "{text}"
    );
    assert!(
        text.contains("Warning: also-tiny.png: the source image is 16x16"),
        "{text}"
    );
}

#[test]
fn delete_source_removes_the_inputs_of_a_fully_successful_batch() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    let out = convert(
        dir.path(),
        &["a.png", "b.png", "--sizes", "16", "--delete-source"],
    );
    assert_success(&out);
    assert_eq!(names_in(dir.path()), vec!["a.ico", "b.ico"]);
}

#[test]
fn delete_source_deletes_nothing_if_any_file_of_the_batch_failed() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "bad.png",
            "--sizes",
            "16",
            "--keep-going",
            "--delete-source",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(dir.path().join("a.png").is_file(), "{}", describe(&out));
    assert!(dir.path().join("bad.png").is_file());
    assert!(
        dir.path().join("a.ico").is_file(),
        "the successful output stays"
    );
}

#[test]
fn skipped_inputs_are_never_deleted() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    std::fs::write(dir.path().join("a.ico"), b"precious").unwrap();
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "--sizes",
            "16",
            "--skip-existing",
            "--delete-source",
        ],
    );
    assert_success(&out);
    assert!(
        dir.path().join("a.png").is_file(),
        "skipped: its source must stay"
    );
    assert!(
        !dir.path().join("b.png").exists(),
        "converted: its source goes"
    );
}

#[test]
fn out_toml_is_written_once_for_a_successful_batch_only() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    write_solid(dir.path(), "c.png", 64, RED);

    let failing = convert(
        dir.path(),
        &[
            "a.png",
            "bad.png",
            "c.png",
            "--sizes",
            "16",
            "--keep-going",
            "--out-toml",
            "snap.toml",
        ],
    );
    assert_eq!(failing.status.code(), Some(1));
    assert!(
        !dir.path().join("snap.toml").exists(),
        "no snapshot after a failed batch"
    );

    let ok = convert(
        dir.path(),
        &[
            "a.png",
            "c.png",
            "--sizes",
            "16",
            "--force",
            "--out-toml",
            "snap.toml",
        ],
    );
    assert_success(&ok);
    let snapshot = std::fs::read_to_string(dir.path().join("snap.toml")).unwrap();
    assert!(snapshot.contains("sizes = \"16\""), "{snapshot}");
    assert_eq!(
        stdout(&ok).matches("Settings written to").count(),
        1,
        "{}",
        describe(&ok)
    );
}

#[test]
fn a_batch_can_write_icns_files() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    assert_success(&img2ico(
        dir.path(),
        &["a.png", "b.png", "--output-format", "icns", "-o", "mac"],
    ));
    assert_eq!(names_in(&dir.path().join("mac")), vec!["a.icns", "b.icns"]);
    let bytes = std::fs::read(dir.path().join("mac").join("a.icns")).unwrap();
    assert_eq!(&bytes[0..4], b"icns");
}

#[test]
fn verbose_names_the_file_for_each_line_in_a_batch() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "b.png", "--sizes", "16", "-v"]);
    assert_success(&out);
    let text = stderr(&out);
    assert!(text.contains("verbose: batch: 2 file(s)"), "{text}");
    assert!(text.contains("verbose: a.png: 16x16: "), "{text}");
    assert!(text.contains("verbose: b.png: 16x16: "), "{text}");
    assert!(text.contains("verbose: a.png: finished in"), "{text}");
}

#[test]
fn the_other_modes_still_insist_on_their_input_rules() {
    let dir = tempfile::tempdir().unwrap();
    make_icos(dir.path());
    assert_failure_containing(
        &img2ico(dir.path(), &["--select", "a.ico", "b.ico"]),
        "exactly one input",
    );
    assert_failure_containing(
        &img2ico(dir.path(), &["--extract", "a.ico", "b.ico"]),
        "exactly one input",
    );
    assert_success(&img2ico(
        dir.path(),
        &["--merge", "a.ico", "b.ico", "-o", "m.ico"],
    ));
}

// =============================================================================
// Automatic background detection, --feather and removal warnings
// =============================================================================

#[test]
fn chroma_key_auto_detects_removes_and_names_the_background_color() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(
        dir.path(),
        &["shot.png", "-c", "auto", "--sizes", "32", "-o", "o.ico"],
    );
    assert_success(&out);
    assert!(
        stdout(&out).contains("Detected background color #00FF00"),
        "{}",
        describe(&out)
    );
    assert!(
        stdout(&out).contains("--chroma-key 00FF00"),
        "the message shows how to take the color over: {}",
        describe(&out)
    );
    let icon = icon_image(&dir.path().join("o.ico"), 32);
    assert_eq!(icon.get_pixel(0, 0)[3], 0, "the background is gone");
    assert_eq!(icon.get_pixel(16, 16)[3], 255, "the square stays");
}

#[test]
fn chroma_key_auto_is_case_insensitive_and_silent_hides_the_message() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(
        dir.path(),
        &["shot.png", "-c", "AUTO", "--sizes", "32", "--silent"],
    );
    assert_success(&out);
    assert!(!stdout(&out).contains("Detected"), "{}", describe(&out));
    assert_eq!(
        icon_image(&dir.path().join("shot.ico"), 32).get_pixel(0, 0)[3],
        0
    );
}

#[test]
fn chroma_key_auto_on_a_gradient_fails_clearly_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut img = RgbaImage::new(64, 64);
    for (x, _, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 4) as u8, 0, 255 - (x * 4) as u8, 255]);
    }
    img.save(dir.path().join("sky.png")).unwrap();
    let out = convert(dir.path(), &["sky.png", "-c", "auto", "-t", "5"]);
    assert_failure_containing(&out, "Could not detect a single background color");
    assert!(
        stderr(&out).contains("--chroma-key #RRGGBB"),
        "{}",
        describe(&out)
    );
    assert!(!dir.path().join("sky.ico").exists());
}

#[test]
fn every_file_of_a_batch_detects_its_own_background() {
    let dir = tempfile::tempdir().unwrap();
    for (name, bg) in [("a.png", [0, 255, 0, 255]), ("b.png", [0, 0, 255, 255])] {
        let mut img = RgbaImage::from_pixel(32, 32, Rgba(bg));
        for y in 12..20 {
            for x in 12..20 {
                img.put_pixel(x, y, Rgba(RED));
            }
        }
        img.save(dir.path().join(name)).unwrap();
    }
    let out = convert(
        dir.path(),
        &["a.png", "b.png", "-c", "auto", "--sizes", "32"],
    );
    assert_success(&out);
    let text = stdout(&out);
    assert!(
        text.contains("a.png: Detected background color #00FF00"),
        "{}",
        describe(&out)
    );
    assert!(
        text.contains("b.png: Detected background color #0000FF"),
        "{}",
        describe(&out)
    );
    for name in ["a.ico", "b.ico"] {
        assert_eq!(
            icon_image(&dir.path().join(name), 32).get_pixel(0, 0)[3],
            0,
            "{name}"
        );
    }
}

#[test]
fn find_auto_uses_the_detected_color_and_auto_can_come_from_a_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    // Green background, a red ring, and green again inside the ring.
    write_image(dir.path(), "patch.png", 30, 30, |x, y| {
        let in_ring_box = (8..22).contains(&x) && (8..22).contains(&y);
        let inside = (10..20).contains(&x) && (10..20).contains(&y);
        if in_ring_box && !inside { RED } else { GREEN }
    });
    let out = convert(dir.path(), &["patch.png", "--find", "auto"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("Detected background color"),
        "{}",
        describe(&out)
    );
    assert!(stdout(&out).contains("--seed"), "{}", describe(&out));

    write_green_background_red_square(dir.path(), "shot.png");
    std::fs::write(dir.path().join("auto.toml"), "chroma-key = \"auto\"\n").unwrap();
    let out = convert(
        dir.path(),
        &["shot.png", "--sizes", "32", "--config", "auto.toml"],
    );
    assert_success(&out);
    assert_eq!(
        icon_image(&dir.path().join("shot.ico"), 32).get_pixel(0, 0)[3],
        0
    );
}

#[test]
fn out_toml_records_auto_as_the_setting_and_the_feather() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    assert_success(&convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "auto",
            "--feather",
            "20",
            "--sizes",
            "32",
            "--out-toml",
            "s.toml",
        ],
    ));
    let text = std::fs::read_to_string(dir.path().join("s.toml")).unwrap();
    assert!(text.contains("chroma-key = \"auto\""), "{text}");
    assert!(text.contains("feather = 20"), "{text}");
}

#[test]
fn a_chroma_key_that_removes_nothing_warns_but_still_converts() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(dir.path(), &["shot.png", "-c", "FF00FF", "--sizes", "32"]);
    assert_success(&out);
    assert!(
        stderr(&out).contains("nothing was removed"),
        "{}",
        describe(&out)
    );
    assert!(
        stderr(&out).contains("--chroma-key auto"),
        "{}",
        describe(&out)
    );
    assert!(dir.path().join("shot.ico").is_file());

    let quiet = convert(
        dir.path(),
        &[
            "shot.png", "-c", "FF00FF", "--sizes", "32", "--force", "--silent",
        ],
    );
    assert_success(&quiet);
    assert_eq!(stderr(&quiet), "", "{}", describe(&quiet));
}

#[test]
fn a_chroma_key_that_removes_almost_everything_warns() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "all.png", 64, GREEN);
    let out = convert(dir.path(), &["all.png", "-c", "00FF00", "--sizes", "16"]);
    assert_success(&out);
    assert!(
        stderr(&out).contains("almost everything was removed"),
        "{}",
        describe(&out)
    );
}

#[test]
fn an_ordinary_removal_does_not_warn() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(dir.path(), &["shot.png", "-c", "00FF00", "--sizes", "32"]);
    assert_success(&out);
    assert_eq!(stderr(&out), "", "{}", describe(&out));
}

#[test]
fn the_removal_warnings_are_counted_in_a_batch_summary() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "a.png");
    write_green_background_red_square(dir.path(), "b.png");
    let out = convert(
        dir.path(),
        &["a.png", "b.png", "-c", "FF00FF", "--sizes", "32"],
    );
    assert_success(&out);
    assert!(stdout(&out).contains("2 warning(s)"), "{}", describe(&out));
    assert!(
        stderr(&out).contains("a.png: nothing was removed"),
        "{}",
        describe(&out)
    );
}

#[test]
fn feather_is_accepted_and_shown_in_the_effective_settings() {
    let dir = tempfile::tempdir().unwrap();
    write_green_background_red_square(dir.path(), "shot.png");
    let out = convert(
        dir.path(),
        &[
            "shot.png",
            "-c",
            "00FF00",
            "--feather",
            "10",
            "--sizes",
            "32",
            "-v",
        ],
    );
    assert_success(&out);
    assert!(stderr(&out).contains("feather = 10"), "{}", describe(&out));
    assert!(stderr(&out).contains("feather 10%"), "{}", describe(&out));
}

// =============================================================================
// Batch mode: folders, filters and names
// =============================================================================

/// assets/{a.png, b.png}, assets/sub/{c.png, c_old.png}, assets/sub/deep/d.png
fn write_asset_tree(dir: &Path) -> PathBuf {
    let assets = dir.join("assets");
    std::fs::create_dir_all(assets.join("sub").join("deep")).unwrap();
    write_solid(&assets, "a.png", 64, RED);
    write_solid(&assets, "b.png", 64, GREEN);
    write_solid(&assets.join("sub"), "c.png", 64, RED);
    write_solid(&assets.join("sub"), "c_old.png", 64, RED);
    write_solid(&assets.join("sub").join("deep"), "d.png", 64, GREEN);
    assets
}

#[test]
fn recursive_converts_the_images_of_all_subfolders() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());

    let out = convert(dir.path(), &["assets", "-r", "-o", "out", "--sizes", "16"]);
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a.ico", "b.ico", "c.ico", "c_old.ico", "d.ico"]
    );
    assert!(stdout(&out).contains("5 converted"), "{}", describe(&out));
}

#[test]
fn without_recursive_the_subfolders_stay_untouched() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    assert_success(&convert(
        dir.path(),
        &["assets", "-o", "out", "--sizes", "16"],
    ));
    assert_eq!(names_in(&dir.path().join("out")), vec!["a.ico", "b.ico"]);
}

#[test]
fn recursive_without_a_folder_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    for flags in [&["-r"][..], &["--include", "*.png"], &["--exclude", "x"]] {
        let mut args = vec!["a.png"];
        args.extend_from_slice(flags);
        assert_failure_containing(
            &convert(dir.path(), &args),
            "only make sense when an input is a folder",
        );
    }
    assert!(!dir.path().join("a.ico").exists());
}

#[test]
fn a_folder_with_only_subfolders_suggests_recursive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("assets").join("sub")).unwrap();
    write_solid(&dir.path().join("assets").join("sub"), "x.png", 64, RED);
    let out = convert(dir.path(), &["assets"]);
    assert_failure_containing(&out, "add --recursive");
    assert_success(&convert(dir.path(), &["assets", "-r", "--sizes", "16"]));
    assert!(
        dir.path()
            .join("assets")
            .join("sub")
            .join("x.ico")
            .is_file()
    );
}

#[test]
fn include_and_exclude_narrow_down_what_a_folder_contributes() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());

    let out = convert(
        dir.path(),
        &[
            "assets",
            "-r",
            "--include",
            "*.png",
            "--exclude",
            "*_old*",
            "--exclude",
            "b.*",
            "-o",
            "out",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a.ico", "c.ico", "d.ico"]
    );
}

#[test]
fn a_pattern_with_a_slash_matches_the_path_below_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    let out = convert(
        dir.path(),
        &[
            "assets",
            "-r",
            "--exclude",
            "sub/**",
            "-o",
            "out",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert_eq!(names_in(&dir.path().join("out")), vec!["a.ico", "b.ico"]);
}

#[test]
fn filters_do_not_touch_files_named_directly() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    write_solid(dir.path(), "own.png", 64, RED);
    let out = convert(
        dir.path(),
        &[
            "own.png",
            "assets",
            "--exclude",
            "*",
            "-o",
            "out",
            "--sizes",
            "16",
        ],
    );
    // The folder is filtered down to nothing - which is reported - ...
    assert_failure_containing(&out, "match the --include/--exclude");
    // ... but with an include that keeps something, the named file is still there.
    let out = convert(
        dir.path(),
        &[
            "own.png",
            "assets",
            "--include",
            "b.*",
            "-o",
            "out",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert_eq!(names_in(&dir.path().join("out")), vec!["b.ico", "own.ico"]);
}

#[test]
fn a_broken_filter_pattern_is_an_error_before_any_work() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    let out = convert(dir.path(), &["assets", "--include", "[abc", "-o", "out"]);
    assert_failure_containing(&out, "Invalid --include pattern");
    assert!(!dir.path().join("out").exists());
}

#[test]
fn keep_structure_rebuilds_the_subfolders_below_the_output_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    let out = convert(
        dir.path(),
        &[
            "assets",
            "-r",
            "-o",
            "out",
            "--keep-structure",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    let out_dir = dir.path().join("out");
    assert_eq!(names_in(&out_dir), vec!["a.ico", "b.ico", "sub"]);
    assert_eq!(
        names_in(&out_dir.join("sub")),
        vec!["c.ico", "c_old.ico", "deep"]
    );
    assert_eq!(names_in(&out_dir.join("sub").join("deep")), vec!["d.ico"]);
}

#[test]
fn files_of_the_same_name_in_different_folders_clash_without_keep_structure() {
    let dir = tempfile::tempdir().unwrap();
    let assets = dir.path().join("assets");
    for sub in ["one", "two"] {
        std::fs::create_dir_all(assets.join(sub)).unwrap();
        write_solid(&assets.join(sub), "logo.png", 64, RED);
    }

    let out = convert(dir.path(), &["assets", "-r", "-o", "out", "--sizes", "16"]);
    assert_failure_containing(&out, "would both be written to");
    assert!(
        stderr(&out).contains("--keep-structure"),
        "{}",
        describe(&out)
    );
    assert!(!dir.path().join("out").exists(), "nothing was written");

    let out = convert(
        dir.path(),
        &[
            "assets",
            "-r",
            "-o",
            "out",
            "--keep-structure",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert!(
        dir.path()
            .join("out")
            .join("one")
            .join("logo.ico")
            .is_file()
    );
    assert!(
        dir.path()
            .join("out")
            .join("two")
            .join("logo.ico")
            .is_file()
    );
}

#[test]
fn without_an_output_folder_the_icons_land_next_to_their_inputs_in_every_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    assert_success(&convert(dir.path(), &["assets", "-r", "--sizes", "16"]));
    let assets = dir.path().join("assets");
    assert!(assets.join("a.ico").is_file());
    assert!(assets.join("sub").join("c.ico").is_file());
    assert!(assets.join("sub").join("deep").join("d.ico").is_file());
}

#[test]
fn keep_structure_needs_an_output_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    let out = convert(dir.path(), &["assets", "--keep-structure"]);
    assert_failure_containing(&out, "--keep-structure needs -o");
}

#[test]
fn a_name_pattern_names_every_icon() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, GREEN);
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "--name",
            "{stem}-app",
            "-o",
            "out",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a-app.ico", "b-app.ico"]
    );
}

#[test]
fn a_name_pattern_with_the_extension_keeps_same_named_files_apart() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_jpeg(dir.path(), "logo.jpg");

    let clash = convert(dir.path(), &["logo.png", "logo.jpg", "--sizes", "16"]);
    assert_failure_containing(&clash, "would both be written to");

    let out = convert(
        dir.path(),
        &[
            "logo.png",
            "logo.jpg",
            "--name",
            "{stem}-{ext}",
            "--sizes",
            "16",
        ],
    );
    assert_success(&out);
    assert!(dir.path().join("logo-png.ico").is_file());
    assert!(dir.path().join("logo-jpg.ico").is_file());
}

#[test]
fn a_name_pattern_also_works_for_a_single_file_and_for_icns() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_success(&convert(
        dir.path(),
        &["logo.png", "--name", "{stem}_{format}", "--sizes", "16"],
    ));
    assert!(dir.path().join("logo_ico.ico").is_file());
}

#[test]
fn a_bad_name_pattern_is_refused_before_any_work() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "--name", "{nope}"]),
        "Unknown variable '{nope}'",
    );
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "--name", "a/{stem}"]),
        "not a path",
    );
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "--name", "{stem}", "-o", "x.ico"]),
        "--name chooses the icon's file name",
    );
    assert_eq!(
        names_in(dir.path()),
        vec!["logo.png"],
        "nothing was written"
    );
}

#[test]
fn the_new_batch_options_are_refused_in_the_modes_that_do_not_convert() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    assert_success(&convert(dir.path(), &["logo.png", "--sizes", "16"]));
    for flags in [
        &["--inspect", "--recursive"][..],
        &["--extract", "--name", "x"],
        &["--select", "--what-if"],
        &["--merge", "--report", "r.csv"],
    ] {
        let mut args = vec!["logo.ico"];
        args.extend_from_slice(flags);
        let out = img2ico(dir.path(), &args);
        assert_failure_containing(&out, "only apply when converting images");
    }
}

#[test]
fn a_settings_file_can_hold_recursive_include_and_exclude() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    std::fs::write(
        dir.path().join("folder.toml"),
        "recursive = true
include = [\"*.png\"]
exclude = [\"*_old*\", \"sub/deep/**\"]
",
    )
    .unwrap();

    let out = convert(
        dir.path(),
        &[
            "assets",
            "-o",
            "out",
            "--sizes",
            "16",
            "--config",
            "folder.toml",
        ],
    );
    assert_success(&out);
    assert_eq!(
        names_in(&dir.path().join("out")),
        vec!["a.ico", "b.ico", "c.ico"]
    );
}

#[test]
fn a_list_on_the_command_line_replaces_the_files_list() {
    let dir = tempfile::tempdir().unwrap();
    write_asset_tree(dir.path());
    std::fs::write(
        dir.path().join("folder.toml"),
        "recursive = true
include = [\"*.png\"]
",
    )
    .unwrap();
    // The file would take every PNG; the command line narrows it to d.png.
    let out = convert(
        dir.path(),
        &[
            "assets",
            "-o",
            "out",
            "--sizes",
            "16",
            "--config",
            "folder.toml",
            "--include",
            "d.*",
        ],
    );
    assert_success(&out);
    assert_eq!(names_in(&dir.path().join("out")), vec!["d.ico"]);
}

#[test]
fn folder_settings_in_a_file_are_ignored_for_single_files() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    std::fs::write(
        dir.path().join("folder.toml"),
        "recursive = true
include = [\"*.jpg\"]
exclude = [\"a*\"]
",
    )
    .unwrap();
    // Typed on the command line this would be an error; from a file it is a
    // standing default that simply has nothing to do here.
    let out = convert(
        dir.path(),
        &["a.png", "--sizes", "16", "--config", "folder.toml"],
    );
    assert_success(&out);
    assert!(dir.path().join("a.ico").is_file());
}

// =============================================================================
// Batch mode: progress, summary, what-if, report
// =============================================================================

#[test]
fn every_file_of_a_batch_gets_a_numbered_progress_line() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.png", "b.png", "c.png"] {
        write_solid(dir.path(), name, 64, RED);
    }
    let out = convert(
        dir.path(),
        &["a.png", "b.png", "c.png", "-o", "out", "--sizes", "16"],
    );
    assert_success(&out);
    let text = stdout(&out);
    for (n, name) in ["a", "b", "c"].iter().enumerate() {
        let expected = format!("[{}/3] {name}.png -> ", n + 1);
        assert!(text.contains(&expected), "{expected}\n{}", describe(&out));
    }
    // The file size is shown, in a human unit.
    assert!(
        text.contains(" B)") || text.contains(" KB)"),
        "{}",
        describe(&out)
    );
    assert!(text.contains("Wrote 3 icon file(s)"), "{}", describe(&out));
}

#[test]
fn a_single_file_has_no_progress_counter() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "--sizes", "16"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("Done: 'a.ico' created"),
        "{}",
        describe(&out)
    );
    assert!(!stdout(&out).contains("[1/1]"), "{}", describe(&out));
}

#[test]
fn skipped_and_failed_files_are_numbered_too() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "b.png");
    write_solid(dir.path(), "c.png", 64, RED);
    assert_success(&convert(dir.path(), &["a.png", "--sizes", "16"]));

    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "c.png",
            "--skip-existing",
            "--keep-going",
            "--sizes",
            "16",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", describe(&out));
    let errors = stderr(&out);
    assert!(
        errors.contains("[1/3] Skipping 'a.png'"),
        "{}",
        describe(&out)
    );
    assert!(errors.contains("[2/3] Error: b.png:"), "{}", describe(&out));
    assert!(
        stdout(&out).contains("[3/3] c.png -> c.ico"),
        "{}",
        describe(&out)
    );
}

#[test]
fn the_summary_counts_warnings() {
    let dir = tempfile::tempdir().unwrap();
    // 8x8 sources are too small for a 16 px icon: one upscaling warning each.
    write_solid(dir.path(), "a.png", 8, RED);
    write_solid(dir.path(), "b.png", 8, RED);
    let out = convert(dir.path(), &["a.png", "b.png", "--sizes", "16"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains(
            "Batch finished: 2 converted, 0 skipped, 0 failed, 2 warning(s) (2 file(s) in total)."
        ),
        "{}",
        describe(&out)
    );
}

#[test]
fn silent_hides_progress_and_a_clean_summary_but_not_a_failure_summary() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    let clean = convert(dir.path(), &["a.png", "b.png", "--silent", "--sizes", "16"]);
    assert_success(&clean);
    assert_eq!(stdout(&clean), "", "{}", describe(&clean));
    assert!(dir.path().join("a.ico").is_file());

    write_junk(dir.path(), "bad.png");
    let failing = convert(
        dir.path(),
        &[
            "a.png",
            "bad.png",
            "--silent",
            "--keep-going",
            "--force",
            "--sizes",
            "16",
        ],
    );
    assert_eq!(failing.status.code(), Some(1), "{}", describe(&failing));
    assert!(
        stdout(&failing).contains("Batch finished: 1 converted, 0 skipped, 1 failed"),
        "{}",
        describe(&failing)
    );
    assert!(
        stderr(&failing).contains("Error: bad.png"),
        "{}",
        describe(&failing)
    );
}

#[test]
fn what_if_shows_what_would_happen_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "-o",
            "out",
            "--sizes",
            "16,32",
            "--what-if",
        ],
    );
    assert_success(&out);
    let text = stdout(&out);
    assert!(text.contains("[1/2] a.png -> "), "{}", describe(&out));
    assert!(
        text.contains("would convert, sizes [16, 32]"),
        "{}",
        describe(&out)
    );
    assert!(
        text.contains(
            "What if: 2 would be converted, 0 skipped, 0 would fail. Nothing was written."
        ),
        "{}",
        describe(&out)
    );
    assert!(!dir.path().join("out").exists(), "no output folder");
    assert_eq!(names_in(dir.path()), vec!["a.png", "b.png"]);
}

#[test]
fn what_if_predicts_what_existing_outputs_mean() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_solid(dir.path(), "b.png", 64, RED);
    assert_success(&convert(dir.path(), &["a.png", "--sizes", "16"]));

    let refused = convert(dir.path(), &["a.png", "b.png", "--what-if"]);
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
    assert!(
        stdout(&refused).contains("would fail: the output already exists"),
        "{}",
        describe(&refused)
    );
    assert!(
        stdout(&refused).contains("1 would fail"),
        "{}",
        describe(&refused)
    );

    let skip = convert(
        dir.path(),
        &["a.png", "b.png", "--what-if", "--skip-existing"],
    );
    assert_success(&skip);
    assert!(stdout(&skip).contains("would skip"), "{}", describe(&skip));

    let force = convert(dir.path(), &["a.png", "b.png", "--what-if", "--force"]);
    assert_success(&force);
    assert!(
        stdout(&force).contains("replacing the existing output"),
        "{}",
        describe(&force)
    );
    assert!(!dir.path().join("b.ico").exists());
}

#[test]
fn what_if_catches_name_collisions_and_missing_inputs() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "logo.png", 64, RED);
    write_jpeg(dir.path(), "logo.jpg");
    assert_failure_containing(
        &convert(dir.path(), &["logo.png", "logo.jpg", "--what-if"]),
        "would both be written to",
    );

    let missing = convert(dir.path(), &["logo.png", "nope.png", "--what-if"]);
    assert_eq!(missing.status.code(), Some(1), "{}", describe(&missing));
    assert!(
        stdout(&missing).contains("input file not found"),
        "{}",
        describe(&missing)
    );
}

#[test]
fn what_if_works_for_a_single_file_and_ignores_find_previews() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "--preset", "favicon", "--what-if"]);
    assert_success(&out);
    assert!(
        stdout(&out).contains("a.png -> a.ico (would convert, sizes [16, 32, 48])"),
        "{}",
        describe(&out)
    );
    assert!(!dir.path().join("a.ico").exists());
}

#[test]
fn what_if_does_not_write_the_settings_snapshot_or_delete_sources() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "--what-if",
            "--delete-source",
            "--out-toml",
            "snap.toml",
        ],
    );
    assert_success(&out);
    assert!(dir.path().join("a.png").is_file());
    assert!(!dir.path().join("snap.toml").exists());
}

#[test]
fn what_if_and_report_cannot_be_combined() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "--what-if", "--report", "r.csv"]);
    assert_eq!(out.status.code(), Some(2), "{}", describe(&out));
}

#[test]
fn a_csv_report_lists_every_file_even_after_failures() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    write_solid(dir.path(), "c.png", 64, RED);
    assert_success(&convert(dir.path(), &["c.png", "--sizes", "16"]));

    let out = convert(
        dir.path(),
        &[
            "a.png",
            "bad.png",
            "c.png",
            "--keep-going",
            "--skip-existing",
            "--sizes",
            "16,32",
            "--report",
            "report.csv",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", describe(&out));
    assert!(
        stdout(&out).contains("Report written to 'report.csv'."),
        "{}",
        describe(&out)
    );

    let csv = std::fs::read_to_string(dir.path().join("report.csv")).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "input,output,status,size_bytes,sizes,warnings,duration_ms,message"
    );
    assert_eq!(lines.len(), 4, "{csv}");
    assert!(lines[1].starts_with("a.png,a.ico,converted,"), "{csv}");
    assert!(lines[1].contains(",16 32,"), "{csv}");
    assert!(lines[2].starts_with("bad.png,bad.ico,failed,"), "{csv}");
    assert!(
        lines[2].contains("bad.png: Could not read input file"),
        "{csv}"
    );
    assert!(lines[3].starts_with("c.png,c.ico,skipped,"), "{csv}");
    assert!(lines[3].contains("--skip-existing"), "{csv}");
}

#[test]
fn a_json_report_has_a_summary_and_per_file_details() {
    let dir = tempfile::tempdir().unwrap();
    // Too small for 16 px: one warning per file - counted even with --silent.
    write_solid(dir.path(), "a.png", 8, RED);
    write_solid(dir.path(), "b.png", 8, RED);
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "--sizes",
            "16",
            "--silent",
            "--report",
            "report.json",
        ],
    );
    assert_success(&out);
    assert_eq!(stdout(&out), "", "--silent: {}", describe(&out));

    let text = std::fs::read_to_string(dir.path().join("report.json")).unwrap();
    let report: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["summary"]["total"], 2);
    assert_eq!(report["summary"]["converted"], 2);
    assert_eq!(report["summary"]["failed"], 0);
    assert_eq!(report["summary"]["warnings"], 2);
    assert!(report["summary"]["output_bytes"].as_u64().unwrap() > 0);
    assert_eq!(report["files"][0]["input"], "a.png");
    assert_eq!(report["files"][0]["status"], "converted");
    assert_eq!(report["files"][0]["sizes"], serde_json::json!([16]));
    assert_eq!(report["files"][0]["warnings"], 1);
    assert!(report["files"][0]["size_bytes"].as_u64().unwrap() > 0);
    assert!(report["files"][0]["message"].is_null());
}

#[test]
fn a_report_is_written_when_the_first_failure_stops_the_run() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "b.png");
    write_solid(dir.path(), "c.png", 64, RED);
    let out = convert(
        dir.path(),
        &[
            "a.png",
            "b.png",
            "c.png",
            "--sizes",
            "16",
            "--report",
            "report.json",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", describe(&out));
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("report.json")).unwrap())
            .unwrap();
    // The run stopped at b.png: c.png was never attempted and is not listed.
    assert_eq!(report["summary"]["total"], 2);
    assert_eq!(report["files"][1]["status"], "failed");
    assert!(!dir.path().join("c.ico").exists());
}

#[test]
fn a_single_file_can_have_a_report_too() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    assert_success(&convert(
        dir.path(),
        &["a.png", "--sizes", "16", "--report", "r.csv"],
    ));
    let csv = std::fs::read_to_string(dir.path().join("r.csv")).unwrap();
    assert_eq!(csv.lines().count(), 2, "{csv}");
}

#[test]
fn an_existing_report_is_replaced_and_other_extensions_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    std::fs::write(dir.path().join("r.csv"), "old").unwrap();
    assert_success(&convert(
        dir.path(),
        &["a.png", "--sizes", "16", "--report", "r.csv"],
    ));
    assert!(
        std::fs::read_to_string(dir.path().join("r.csv"))
            .unwrap()
            .starts_with("input,")
    );

    let out = convert(dir.path(), &["a.png", "--force", "--report", "a.png"]);
    assert_failure_containing(&out, "must end in .csv or .json");
    assert!(
        image::open(dir.path().join("a.png")).is_ok(),
        "the image is untouched"
    );
}

// =============================================================================
// --inspect --json
// =============================================================================

#[test]
fn inspect_json_describes_an_ico_file() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    assert_success(&convert(dir.path(), &["a.png", "--sizes", "16,32"]));

    let out = img2ico(dir.path(), &["--inspect", "a.ico", "--json"]);
    assert_success(&out);
    let report: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let file = &report[0];
    assert_eq!(report.as_array().unwrap().len(), 1);
    assert_eq!(file["path"], "a.ico");
    assert_eq!(file["kind"], "ico");
    assert_eq!(file["entries"][0]["width"], 16);
    assert_eq!(file["entries"][1]["height"], 32);
    assert_eq!(file["entries"][0]["format"], "png");
    assert!(file["entries"][0]["bits_per_pixel"].as_u64().unwrap() >= 24);
    assert_eq!(file["entries"][0]["index"], 0);
    assert!(file["entries"][0]["bytes"].as_u64().unwrap() > 0);
    assert!(
        file["missing_windows_sizes"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(256))
    );
    assert_eq!(file["warnings"], serde_json::json!([]));
}

#[test]
fn inspect_json_describes_source_images_and_several_files_at_once() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_gif(dir.path(), "g.gif", &[[255, 0, 0], [0, 255, 0]]);
    let out = img2ico(dir.path(), &["--inspect", "a.png", "g.gif", "--json"]);
    assert_success(&out);
    let report: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(report.as_array().unwrap().len(), 2);

    assert_eq!(report[0]["kind"], "image");
    assert_eq!(report[0]["width"], 64);
    assert!(report[0]["frames"].is_null());
    assert_eq!(
        report[0]["windows"]["native"],
        serde_json::json!([16, 20, 24, 32, 40, 48, 64])
    );
    assert_eq!(
        report[0]["windows"]["upscaled"],
        serde_json::json!([96, 128, 256])
    );
    assert!(
        !report[0]["macos"]["upscaled"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert_eq!(report[1]["frames"], 2);
}

#[test]
fn inspect_json_prints_nothing_but_an_error_when_a_file_is_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    write_junk(dir.path(), "bad.png");
    let out = img2ico(dir.path(), &["--inspect", "a.png", "bad.png", "--json"]);
    assert_failure_containing(&out, "neither a readable .ico file");
    assert_eq!(stdout(&out), "", "{}", describe(&out));
}

#[test]
fn json_without_inspect_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    write_solid(dir.path(), "a.png", 64, RED);
    let out = convert(dir.path(), &["a.png", "--json"]);
    assert_eq!(out.status.code(), Some(2), "{}", describe(&out));
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
