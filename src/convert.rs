// The normal mode: turning ONE source image into an .ico (or .icns) file.
// `run` reads top to bottom as the pipeline itself - validate the options,
// load the image, optionally --find regions, choose the output, recolor,
// warn, write, clean up - with every step a small function of its own.

use crate::chroma_key::{FoundRegion, apply_chroma_key, find_isolated_regions, parse_hex_color};
use crate::cli::{Args, OutputFormat};
use crate::gif::{extract_gif_frame, is_gif};
use crate::icns::{icns_sizes, write_icns};
use crate::resize::{
    apply_grayscale, has_transparency, make_square_icon, warn_about_thin_content,
    warn_about_upscaling,
};
use crate::settings::{ResolvedSettings, finish_run};
use crate::util::{check_overwrite, parse_seed};
use image::RgbaImage;
use std::path::{Path, PathBuf};

/// The sizes used when neither --sizes, a config file nor --preset says
/// otherwise.
const DEFAULT_SIZES: &str = "16,32,48,64,128,256";

/// ICO files officially only support edge lengths up to 256px.
const MAX_ICO_SIZE: u32 = 256;

/// Runs the complete image-to-icon conversion for the parsed `args` and
/// the `resolved` settings (command line + config file + defaults).
pub fn run(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    // Normal (non-merge) mode expects exactly one input image.
    let [input_path] = args.input.as_slice() else {
        return Err(format!(
            "Expected exactly one input image, got {} (use --merge to combine multiple existing .ico files instead).",
            args.input.len()
        ));
    };

    check_color_options(resolved)?;
    let replacement = resolved
        .replace_color
        .map(|hex| parse_hex_color(hex).map_err(|e| format!("Invalid --replace-color value: {e}")))
        .transpose()?;

    // Load the input image EARLY - before the output path/--sizes are even
    // looked at - specifically so that a plain "--find" preview (see
    // below) can print its report and exit without needing any of that:
    // it doesn't write a file, so it shouldn't need to know or care where
    // one WOULD have gone.
    let mut source = load_source_image(input_path, resolved)?;

    // --find: look for regions matching this color that the border-based
    // flood fill in apply_chroma_key can't reach on its own (the same
    // situation --seed manually solves, just discovered automatically).
    // Without --auto-apply, this is the ENTIRE effect of --find: print
    // the suggestions and stop, without converting anything (and without
    // --out-toml writing anything either - nothing was actually decided
    // about the FINAL settings yet in that case).
    let mut discovered_seeds: Vec<(u32, u32)> = Vec::new();
    if let Some(find_hex) = resolved.find {
        let regions = find_regions(&source, find_hex, resolved)?;
        print_found_regions(find_hex, &regions);

        if !resolved.auto_apply {
            if !regions.is_empty() {
                println!(
                    "Re-run with these as --seed values, or add --auto-apply to use them automatically."
                );
            }
            return Ok(());
        }

        discovered_seeds = regions.into_iter().map(|r| r.seed).collect();
    }

    let use_icns = wants_icns(resolved.output_format);
    let output_path = output_path(args.output.as_deref(), input_path, use_icns);

    // Fail fast, before doing any actual work (resizing the image), if
    // the resolved output already exists and --force wasn't given. This
    // one check covers both the normal ICO path and --icns, since they
    // share this same output_path.
    check_overwrite(&output_path, resolved.force)?;

    // Only relevant for the normal ICO path - icns output uses its own
    // fixed set of sizes instead (see icns::ICNS_SIZES) - but resolving it
    // here anyway is cheap and harmless even when it ends up unused.
    let sizes = resolve_sizes(resolved)?;
    if !use_icns && sizes.is_empty() {
        return Err("At least one size must be given.".to_string());
    }

    remove_background(&mut source, resolved, replacement, discovered_seeds)?;

    // --grayscale runs LAST, after any --chroma-key/--replace-color
    // processing above - so it uniformly affects the final colors,
    // including a --replace-color color if both were combined, rather
    // than leaving a confusing "everything except the replaced background
    // is grayscale" exception.
    if resolved.grayscale {
        apply_grayscale(&mut source);
    }

    // Computed ONCE here and passed down to every make_square_icon call
    // (for every single icon size) instead of re-checking per size - see
    // has_transparency()'s doc comment for why this matters.
    let has_alpha = has_transparency(&source);

    warn_about_small_source(&source, use_icns, &sizes, resolved);

    // .icns branches off here (whether from an explicit --output-format
    // icns or from the platform-based default): it has its own container
    // format (see write_icns) and doesn't use the ICO-specific --sizes
    // list at all.
    if use_icns {
        write_icns(&source, resolved.padding, has_alpha, &output_path)?;
    } else {
        write_ico(&source, &sizes, has_alpha, resolved, &output_path)?;
    }

    finish_run(
        args,
        resolved,
        std::slice::from_ref(input_path),
        Some(&output_path),
    )
}

/// Rejects option combinations that contradict each other or make no
/// sense on their own.
fn check_color_options(resolved: &ResolvedSettings) -> Result<(), String> {
    // --chroma-key and --find both ultimately specify "the background
    // color" - only one is needed at a time, and allowing both would just
    // invite the confusing case of them disagreeing.
    if resolved.chroma_key.is_some() && resolved.find.is_some() {
        return Err(
            "--chroma-key and --find are mutually exclusive - use --find for automatic seed discovery, or --chroma-key with manual --seed values."
                .to_string(),
        );
    }
    if resolved.auto_apply && resolved.find.is_none() {
        return Err("--auto-apply has no effect without --find.".to_string());
    }
    if resolved.replace_color.is_some() && resolved.chroma_key.is_none() && resolved.find.is_none()
    {
        return Err(
            "--replace-color requires --chroma-key or --find (there's no background color to replace otherwise)."
                .to_string(),
        );
    }
    Ok(())
}

/// Loads the source image as RGBA.
///
/// image::open() alone - used for every other format - only ever returns
/// a GIF's FIRST frame, with no way to pick a different one. So: for a
/// GIF specifically, --gif-frame picks which frame becomes the source
/// image (extract_gif_frame() already gives back an RgbaImage directly);
/// for everything else, --gif-frame has no meaning and image::open()
/// (which detects the format from the file header, not just its
/// extension) is used exactly as before.
fn load_source_image(input_path: &Path, resolved: &ResolvedSettings) -> Result<RgbaImage, String> {
    if is_gif(input_path)? {
        return extract_gif_frame(input_path, resolved.gif_frame);
    }

    if resolved.gif_frame != 1 && !resolved.silent {
        eprintln!("Warning: --gif-frame only applies to GIF input and is ignored for this file.");
    }
    // IMPORTANT (optimization): to_rgba8() converts/copies the entire
    // image. This intentionally happens only ONCE here, regardless of
    // whether --chroma-key/--find is set or not - and NOT (as in an
    // earlier version) again for every single icon size. For a large
    // source image and the 6 default sizes, that would otherwise have
    // been 6 full copies of the source image, even though the source
    // image doesn't change between sizes.
    Ok(image::open(input_path)
        .map_err(|e| format!("Could not read input file: {e}"))?
        .to_rgba8())
}

/// Runs --find's region discovery on the source image.
fn find_regions(
    source: &RgbaImage,
    find_hex: &str,
    resolved: &ResolvedSettings,
) -> Result<Vec<FoundRegion>, String> {
    let target = parse_hex_color(find_hex).map_err(|e| format!("Invalid --find value: {e}"))?;
    Ok(find_isolated_regions(
        source,
        target,
        resolved.tolerance,
        resolved.find_min_size,
        resolved.silent,
    ))
}

/// Prints --find's report: how many regions were found and, for each, a
/// ready-to-use --seed value.
fn print_found_regions(find_hex: &str, regions: &[FoundRegion]) {
    if regions.is_empty() {
        println!(
            "No additional regions matching {find_hex} found - the border-based flood fill should already reach everything."
        );
        return;
    }

    println!(
        "Found {} additional region(s) matching {find_hex} that the border-based flood fill can't reach on its own:",
        regions.len()
    );
    for region in regions {
        let (x, y) = region.seed;
        println!(
            "  ~{} pixel(s) near ({x}, {y}) -> --seed {x},{y}",
            region.pixel_count
        );
    }
}

/// Whether to produce .icns or .ico. If the user gave --output-format
/// explicitly (or a config file set it), that wins outright. Otherwise,
/// fall back to a platform-based default: .icns on macOS, .ico
/// everywhere else - this is what most people building on a given
/// platform actually want, without having to remember to pass
/// --output-format every time on a Mac.
///
/// cfg!(target_os = "macos") is a compile-time check (baked into the
/// binary depending on what platform it was BUILT for), not a runtime
/// one - which is exactly what we want here: a binary built natively
/// on macOS should default to .icns, regardless of where it's later
/// copied to and run from.
fn wants_icns(output_format: Option<OutputFormat>) -> bool {
    match output_format {
        Some(OutputFormat::Icns) => true,
        Some(OutputFormat::Ico) => false,
        None => cfg!(target_os = "macos"),
    }
}

/// The path to write to: the explicit -o/--output if given (it stays
/// command-line-only, not config-file-eligible - see cli.rs's doc comment
/// on --config for why), otherwise the input's path with its extension
/// swapped for ".ico" (or ".icns").
fn output_path(explicit: Option<&Path>, input_path: &Path, use_icns: bool) -> PathBuf {
    match explicit {
        Some(path) => path.to_path_buf(),
        None => input_path.with_extension(if use_icns { "icns" } else { "ico" }),
    }
}

/// The icon sizes to generate for the ICO output.
///
/// --preset short-circuits --sizes: it replaces whatever --sizes says with
/// a predefined list outright, so --sizes is simply never parsed/looked at
/// in that case. If neither --sizes, a config file, nor --preset set
/// anything, img2ico's own built-in default is used.
fn resolve_sizes(resolved: &ResolvedSettings) -> Result<Vec<u32>, String> {
    match resolved.preset {
        Some(preset) => Ok(preset.sizes().to_vec()),
        None => parse_size_list(resolved.sizes.unwrap_or(DEFAULT_SIZES)),
    }
}

/// Turns a sizes string ("16,32,48") into a list of numbers. Each
/// individual size can fail (not a valid number) - so the map() closure
/// returns a Result instead of a plain u32. collect() into a
/// `Result<Vec<u32>, String>` then gathers everything into ONE Result: as
/// soon as any size is invalid, it immediately returns that one Err (and
/// stops), instead of still processing the rest of the list.
fn parse_size_list(text: &str) -> Result<Vec<u32>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<u32>()
                .map_err(|_| format!("Invalid size: '{s}' (must be a positive number)"))
        })
        .collect()
}

/// Parses every --seed value ("x,y") into a coordinate pair.
fn parse_seeds(seeds: &[String]) -> Result<Vec<(u32, u32)>, String> {
    seeds
        .iter()
        .map(|s| parse_seed(s).map_err(|e| format!("Invalid --seed value: {e}")))
        .collect()
}

/// Applies the actual chroma-key removal/replacement, if requested either
/// way: --chroma-key (with whatever manual --seed values were given), or
/// --find --auto-apply (the manual --seed values PLUS the automatically
/// `discovered` regions from the --find preview, combined). Does nothing
/// if neither was given. `replacement` (from --replace-color) is passed
/// through to both.
fn remove_background(
    img: &mut RgbaImage,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    discovered: Vec<(u32, u32)>,
) -> Result<(), String> {
    // The two are mutually exclusive (see check_color_options), so at most
    // one of them names the background color. The --find case only gets
    // here with --auto-apply - the plain preview already returned.
    let (flag, hex) = match (resolved.chroma_key, resolved.find) {
        (Some(hex), _) => ("--chroma-key", hex),
        (None, Some(hex)) => ("--find", hex),
        (None, None) => return Ok(()),
    };

    let target = parse_hex_color(hex).map_err(|e| format!("Invalid {flag} value: {e}"))?;
    let mut seeds = parse_seeds(resolved.seeds)?;
    seeds.extend(discovered);
    apply_chroma_key(
        img,
        target,
        resolved.tolerance,
        &seeds,
        replacement,
        resolved.silent,
    );
    Ok(())
}

/// A heads-up (not an error - upscaling still produces a valid icon, just
/// a softer one) if any requested size exceeds what the source image
/// actually has to offer, or would leave only a sliver of content.
/// Resizing up can't invent detail that isn't there; Lanczos3 (what
/// make_square_icon uses) makes that smooth rather than blocky, but it's
/// still fundamentally a guess, not real detail.
fn warn_about_small_source(
    source: &RgbaImage,
    use_icns: bool,
    ico_sizes: &[u32],
    resolved: &ResolvedSettings,
) {
    let (width, height) = source.dimensions();
    let icns_list;
    let sizes = if use_icns {
        icns_list = icns_sizes();
        &icns_list
    } else {
        ico_sizes
    };
    warn_about_upscaling(width, height, sizes, resolved.silent);
    warn_about_thin_content(width, height, resolved.padding, sizes, resolved.silent);
}

/// Builds the .ico: one square icon per requested size, all PNG-encoded,
/// written together into `output_path`.
fn write_ico(
    source: &RgbaImage,
    sizes: &[u32],
    has_alpha: bool,
    resolved: &ResolvedSettings,
    output_path: &Path,
) -> Result<(), String> {
    // An IconDir collects all the resolutions that will be written
    // together into ONE .ico file at the end.
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &size in sizes {
        if size == 0 || size > MAX_ICO_SIZE {
            if !resolved.silent {
                eprintln!("Skipping size {size} (valid range: 1-{MAX_ICO_SIZE}).");
            }
            continue;
        }

        let square = make_square_icon(source, size, resolved.padding, has_alpha);
        let (w, h) = square.dimensions();

        // into_raw() gives us the raw pixel bytes in RGBA order (red,
        // green, blue, alpha, red, green, blue, alpha, ...) - exactly the
        // format the ico crate expects as input.
        let icon_image = ico::IconImage::from_rgba_data(w, h, square.into_raw());

        // IMPORTANT: we deliberately force PNG encoding for EVERY size,
        // instead of trusting the default "IconDirEntry::encode()" method.
        // Reason: ico::encode() internally decides via a heuristic between
        // PNG and the old BMP format (for compatibility with very old
        // Windows versions). For small and/or fully opaque images it
        // chooses BMP - and that means:
        //   - only a 1-bit transparency mask (a pixel is either fully
        //     visible or fully invisible, no more soft edges)
        //   - often only 8-bit color depth (256-color palette instead of
        //     true color), which causes visible color banding/"pixelation"
        // PNG, on the other hand, keeps full color depth and, whenever the
        // image has any transparency, a clean alpha channel - exactly what
        // was required for "transparency as a feature". PNG-in-ICO has been
        // supported by Windows since Vista (2007), so it's safe for
        // practically any use case.
        let entry = ico::IconDirEntry::encode_as_png(&icon_image)
            .map_err(|e| format!("Could not encode size {size}: {e}"))?;
        icon_dir.add_entry(entry);
    }

    // Create the target file and write all the collected resolutions into it.
    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Could not create output file: {e}"))?;
    icon_dir
        .write(file)
        .map_err(|e| format!("Error writing ICO file: {e}"))?;

    println!(
        "Done: '{}' created with sizes {:?}.",
        output_path.display(),
        sizes
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use clap::Parser;

    fn parsed(extra: &[&str]) -> Args {
        let mut full = vec!["img2ico", "in.png"];
        full.extend_from_slice(extra);
        Args::parse_from(full)
    }

    /// Runs `check` against the settings resolved from the given flags.
    fn with_resolved<T>(flags: &[&str], check: impl FnOnce(&ResolvedSettings) -> T) -> T {
        let (cli, file) = (parsed(flags), Settings::default());
        check(&ResolvedSettings::resolve(&cli, &file))
    }

    // --- parse_size_list / resolve_sizes -------------------------------------------

    #[test]
    fn size_lists_are_parsed_in_order() {
        assert_eq!(parse_size_list("16,32,48"), Ok(vec![16, 32, 48]));
        assert_eq!(parse_size_list("256"), Ok(vec![256]));
    }

    #[test]
    fn size_lists_tolerate_whitespace_and_empty_items() {
        assert_eq!(parse_size_list(" 16 , 32 ,"), Ok(vec![16, 32]));
        assert_eq!(parse_size_list(",,"), Ok(vec![]));
        assert_eq!(parse_size_list(""), Ok(vec![]));
    }

    #[test]
    fn an_invalid_size_is_an_error_naming_it() {
        let err = parse_size_list("16,abc,32").unwrap_err();
        assert!(err.contains("abc"), "unexpected message: {err}");
        assert!(parse_size_list("-16").is_err());
        assert!(parse_size_list("1.5").is_err());
    }

    #[test]
    fn sizes_default_when_nothing_is_set() {
        let sizes = with_resolved(&[], resolve_sizes).unwrap();
        assert_eq!(sizes, vec![16, 32, 48, 64, 128, 256]);
    }

    #[test]
    fn explicit_sizes_are_used() {
        let sizes = with_resolved(&["--sizes", "20,40"], resolve_sizes).unwrap();
        assert_eq!(sizes, vec![20, 40]);
    }

    #[test]
    fn a_preset_replaces_sizes_even_invalid_ones() {
        let sizes = with_resolved(
            &["--sizes", "nonsense", "--preset", "minimal"],
            resolve_sizes,
        );
        assert_eq!(sizes, Ok(vec![16, 32]));
    }

    // --- output path / format ---------------------------------------------------------

    #[test]
    fn default_output_path_swaps_the_extension() {
        let input = Path::new("art/logo.png");
        assert_eq!(output_path(None, input, false), Path::new("art/logo.ico"));
        assert_eq!(output_path(None, input, true), Path::new("art/logo.icns"));
    }

    #[test]
    fn default_output_path_handles_inputs_without_an_extension() {
        assert_eq!(
            output_path(None, Path::new("logo"), false),
            Path::new("logo.ico")
        );
    }

    #[test]
    fn an_explicit_output_path_is_used_unchanged() {
        let explicit = Path::new("out/custom.name");
        assert_eq!(
            output_path(Some(explicit), Path::new("in.png"), true),
            explicit
        );
    }

    #[test]
    fn explicit_output_formats_beat_the_platform_default() {
        assert!(wants_icns(Some(OutputFormat::Icns)));
        assert!(!wants_icns(Some(OutputFormat::Ico)));
        assert_eq!(wants_icns(None), cfg!(target_os = "macos"));
    }

    // --- option validation ----------------------------------------------------------------

    #[test]
    fn valid_color_option_combinations_pass() {
        for flags in [
            &[][..],
            &["--chroma-key", "00FF00"],
            &["--find", "00FF00"],
            &["--find", "00FF00", "--auto-apply"],
            &["--chroma-key", "00FF00", "--replace-color", "000000"],
            &["--find", "00FF00", "--replace-color", "000000"],
        ] {
            assert_eq!(
                with_resolved(flags, check_color_options),
                Ok(()),
                "{flags:?}"
            );
        }
    }

    #[test]
    fn contradicting_or_dangling_color_options_fail() {
        let exclusive = with_resolved(
            &["--chroma-key", "00FF00", "--find", "00FF00"],
            check_color_options,
        );
        assert!(exclusive.unwrap_err().contains("mutually exclusive"));

        let dangling_auto_apply = with_resolved(&["--auto-apply"], check_color_options);
        assert!(dangling_auto_apply.unwrap_err().contains("--auto-apply"));

        let dangling_replace = with_resolved(&["--replace-color", "000000"], check_color_options);
        assert!(dangling_replace.unwrap_err().contains("--replace-color"));
    }

    // --- seeds and background removal -------------------------------------------------------

    #[test]
    fn seeds_are_parsed_and_bad_ones_name_the_flag() {
        let ok = vec!["1,2".to_string(), " 3 , 4 ".to_string()];
        assert_eq!(parse_seeds(&ok), Ok(vec![(1, 2), (3, 4)]));
        let bad = vec!["1,2".to_string(), "oops".to_string()];
        assert!(
            parse_seeds(&bad)
                .unwrap_err()
                .contains("Invalid --seed value")
        );
    }

    #[test]
    fn remove_background_does_nothing_without_a_background_color() {
        let mut img = RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        let before = img.clone();
        with_resolved(&[], |resolved| {
            remove_background(&mut img, resolved, None, vec![]).unwrap();
        });
        assert_eq!(img, before);
    }

    #[test]
    fn remove_background_uses_chroma_key_and_reports_a_bad_color_by_flag() {
        let mut img = RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        with_resolved(&["--chroma-key", "00FF00"], |resolved| {
            remove_background(&mut img, resolved, None, vec![]).unwrap();
        });
        assert!(img.pixels().all(|p| p[3] == 0));

        let err = with_resolved(&["--chroma-key", "nope"], |resolved| {
            remove_background(&mut img, resolved, None, vec![]).unwrap_err()
        });
        assert!(err.contains("Invalid --chroma-key value"), "{err}");

        let err = with_resolved(&["--find", "nope", "--auto-apply"], |resolved| {
            remove_background(&mut img, resolved, None, vec![]).unwrap_err()
        });
        assert!(err.contains("Invalid --find value"), "{err}");
    }

    #[test]
    fn remove_background_adds_discovered_seeds_to_the_manual_ones() {
        // A red ring around a green patch: only a seed can reach the patch.
        let mut img = RgbaImage::from_pixel(9, 9, image::Rgba([0, 255, 0, 255]));
        for y in 2..=6 {
            for x in 2..=6 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        img.put_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        with_resolved(&["--find", "00FF00", "--auto-apply"], |resolved| {
            remove_background(&mut img, resolved, None, vec![(4, 4)]).unwrap();
        });
        assert_eq!(img.get_pixel(4, 4)[3], 0, "the discovered seed was used");
        assert_eq!(img.get_pixel(0, 0)[3], 0, "the border was cleared too");
    }
}
