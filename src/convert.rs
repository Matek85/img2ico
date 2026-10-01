// The normal mode: turning source images into .ico (or .icns) files.
//
// With ONE input file that is a single conversion; with several inputs, or
// a folder, it is a batch of them ("Batch" below). Either way every file
// goes through the same `convert_one`, which reads top to bottom as the
// pipeline itself - load the image, optionally --find regions, recolor,
// warn, write - with every step a small function of its own. What differs
// between the two modes is only the planning before it and the bookkeeping
// after it.

use crate::chroma_key::{FoundRegion, apply_chroma_key, find_isolated_regions, parse_hex_color};
use crate::cli::{Args, OutputFormat};
use crate::gif::{extract_gif_frame, is_gif};
use crate::icns::{icns_sizes, write_icns};
use crate::resize::{
    apply_grayscale, has_transparency, make_square_icon, warn_about_thin_content,
    warn_about_upscaling,
};
use crate::settings::{
    ResolvedSettings, delete_sources_if_requested, finish_run, maybe_write_out_toml,
};
use crate::util::{check_overwrite, enter_file_context, file_prefix, parse_seed};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The sizes used when neither --sizes, a config file nor --preset says
/// otherwise.
const DEFAULT_SIZES: &str = "16,32,48,64,128,256";

/// ICO files officially only support edge lengths up to 256px.
const MAX_ICO_SIZE: u32 = 256;

/// The file extensions (compared case-insensitively) that a folder given as
/// input contributes - the formats the normal conversion accepts.
const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];

/// One conversion: which image to read and where the icon goes.
struct Job {
    input: PathBuf,
    output: PathBuf,
}

/// What became of one input.
enum Outcome {
    /// The icon was written.
    Converted,
    /// The output already existed and --skip-existing said to leave it.
    Skipped,
    /// A --find preview: the report was printed, nothing was written.
    Previewed,
}

/// What the command line asks for: one file, or a batch of them.
enum Plan<'a> {
    Single(&'a Path),
    Batch(Vec<PathBuf>),
}

/// Runs the complete image-to-icon conversion for the parsed `args` and
/// the `resolved` settings (command line + config file + defaults).
pub fn run(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    let plan = plan_inputs(&args.input)?;

    check_options(resolved)?;
    let replacement = resolved
        .replace_color
        .map(|hex| parse_hex_color(hex).map_err(|e| format!("Invalid --replace-color value: {e}")))
        .transpose()?;

    match plan {
        Plan::Single(input) => run_single(args, resolved, replacement, input),
        Plan::Batch(files) => run_batch(args, resolved, replacement, files),
    }
}

/// Converts one file: `-o` (if given) is the output FILE.
fn run_single(
    args: &Args,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    input: &Path,
) -> Result<(), String> {
    let use_icns = wants_icns(resolved.output_format);
    let job = Job {
        input: input.to_path_buf(),
        output: output_path(args.output.as_deref(), input, use_icns),
    };

    match convert_one(&job, resolved, replacement)? {
        Outcome::Converted => finish_run(
            args,
            resolved,
            std::slice::from_ref(&job.input),
            Some(&job.output),
        ),
        // Nothing was written, so there is nothing to clean up and no
        // settings snapshot worth saving.
        Outcome::Skipped | Outcome::Previewed => Ok(()),
    }
}

/// Converts several files in a row with the same settings: `-o` (if given)
/// is the FOLDER the icons go into.
///
/// By default the first failure stops the run (the error names the file);
/// with --keep-going the rest is still converted, every failure is
/// reported, and the run fails at the end. Either way, source files are
/// deleted (--delete-source) and the settings snapshot is written
/// (--out-toml) only if the whole batch succeeded - a half-finished run
/// must never take the originals with it.
fn run_batch(
    args: &Args,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    files: Vec<PathBuf>,
) -> Result<(), String> {
    check_batch_options(args.output.as_deref(), resolved)?;

    let use_icns = wants_icns(resolved.output_format);
    let jobs = plan_jobs(files, args.output.as_deref(), use_icns)?;
    resolved.note(format_args!("batch: {} file(s)", jobs.len()));

    if let Some(folder) = args.output.as_deref() {
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("Could not create output folder '{}': {e}", folder.display()))?;
    }

    let mut converted: Vec<&Job> = Vec::new();
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for job in &jobs {
        // Lets warnings raised deep inside the conversion name this file.
        let _context = enter_file_context(&job.input);
        match convert_one(job, resolved, replacement) {
            Ok(Outcome::Converted) => converted.push(job),
            Ok(Outcome::Skipped | Outcome::Previewed) => skipped += 1,
            Err(message) => {
                // Name the file, unless the message already does.
                let name = job.input.display().to_string();
                let message = if message.contains(&name) {
                    message
                } else {
                    format!("{name}: {message}")
                };
                if !resolved.keep_going {
                    return Err(message);
                }
                eprintln!("Error: {message}");
                failed += 1;
            }
        }
    }

    println!(
        "Batch finished: {} converted, {skipped} skipped, {failed} failed ({} file(s) in total).",
        converted.len(),
        jobs.len()
    );
    if failed > 0 {
        return Err(format!("{failed} of {} file(s) failed.", jobs.len()));
    }

    for job in converted {
        delete_sources_if_requested(
            resolved,
            std::slice::from_ref(&job.input),
            Some(&job.output),
        );
    }
    maybe_write_out_toml(args.out_toml.as_deref(), resolved)
}

/// The conversion of one image into one icon file - everything that is the
/// same for a single file and for each file of a batch.
fn convert_one(
    job: &Job,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
) -> Result<Outcome, String> {
    let started = Instant::now();
    let input_path = job.input.as_path();
    let output_path = job.output.as_path();
    let use_icns = wants_icns(resolved.output_format);

    // --skip-existing: with the output already there there is nothing to do
    // - and nothing worth loading, so this comes before anything else. (A
    // --find preview writes no file, so it is never skipped.)
    let find_preview = resolved.find.is_some() && !resolved.auto_apply;
    if resolved.skip_existing && !find_preview && output_path.exists() {
        if !resolved.silent {
            eprintln!(
                "Skipping '{}': '{}' already exists (--skip-existing).",
                input_path.display(),
                output_path.display()
            );
        }
        return Ok(Outcome::Skipped);
    }

    // Load the input image EARLY - before the output path/--sizes are even
    // looked at - specifically so that a plain "--find" preview (see
    // below) can print its report and exit without needing any of that:
    // it doesn't write a file, so it shouldn't need to know or care where
    // one WOULD have gone.
    let load_started = Instant::now();
    let mut source = load_source_image(input_path, resolved)?;
    let (width, height) = source.dimensions();
    resolved.note(format_args!(
        "source: {} ({width}x{height} pixels), loaded in {:.1?}",
        input_path.display(),
        load_started.elapsed()
    ));

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
            return Ok(Outcome::Previewed);
        }

        discovered_seeds = regions.into_iter().map(|r| r.seed).collect();
    }

    resolved.note(format_args!(
        "output: {} ({})",
        output_path.display(),
        if use_icns { "icns" } else { "ico" }
    ));

    // Fail fast, before doing any actual work (resizing the image), if
    // the resolved output already exists and --force wasn't given. This
    // one check covers both the normal ICO path and --icns, since they
    // share this same output_path.
    check_overwrite(output_path, resolved.force)?;

    // Only relevant for the normal ICO path - icns output uses its own
    // fixed set of sizes instead (see icns::ICNS_SIZES) - but resolving it
    // here anyway is cheap and harmless even when it ends up unused.
    let sizes = resolve_sizes(resolved)?;
    if !use_icns && sizes.is_empty() {
        return Err("At least one size must be given.".to_string());
    }
    if !use_icns {
        resolved.note(format_args!(
            "sizes: {sizes:?}, padding: {}%",
            resolved.padding
        ));
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
        write_icns(&source, resolved.padding, has_alpha, output_path)?;
    } else {
        write_ico(&source, &sizes, has_alpha, resolved, output_path)?;
    }

    resolved.note(format_args!(
        "{}finished in {:.1?}",
        file_prefix(),
        started.elapsed()
    ));
    Ok(Outcome::Converted)
}

/// Decides whether the inputs are one file or a batch. One input that is
/// not a folder is a single conversion - including a path that doesn't
/// exist, which `convert_one` then reports as unreadable, as ever. Anything
/// else (several inputs, or a folder) is a batch of the files they stand
/// for.
fn plan_inputs(inputs: &[PathBuf]) -> Result<Plan<'_>, String> {
    if let [only] = inputs
        && !only.is_dir()
    {
        return Ok(Plan::Single(only));
    }
    expand_inputs(inputs).map(Plan::Batch)
}

/// Whether `path` has one of the extensions in `IMAGE_EXTENSIONS`.
fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

/// Turns the inputs into the list of image files to convert: a file stands
/// for itself, a folder for the supported image files directly inside it
/// (subfolders are not searched), in name order so a run is reproducible.
/// A folder without any such file is an error rather than a silent no-op.
fn expand_inputs(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        if !input.is_dir() {
            files.push(input.clone());
            continue;
        }

        let entries = std::fs::read_dir(input)
            .map_err(|e| format!("Could not read folder '{}': {e}", input.display()))?;
        let mut found: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && is_supported_image(path))
            .collect();
        if found.is_empty() {
            return Err(format!(
                "No supported images (PNG, JPG, BMP, GIF) found in folder '{}'.",
                input.display()
            ));
        }
        found.sort();
        files.extend(found);
    }
    Ok(files)
}

/// Where the icon for `input` goes in a batch: `<stem>.<extension>` inside
/// `folder`, or - without one - right next to the input.
fn batch_output_path(folder: Option<&Path>, input: &Path, extension: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("icon");
    let name = format!("{stem}.{extension}");
    match folder {
        Some(folder) => folder.join(name),
        None => input.with_file_name(name),
    }
}

/// A form of `path` in which two paths naming the same file compare equal:
/// lowercased where the file system ignores case (Windows, macOS).
fn collision_key(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(any(windows, target_os = "macos")) {
        text.to_lowercase()
    } else {
        text
    }
}

/// Pairs every input with its output path - and refuses the whole batch,
/// before anything is written, if two inputs would end up as the same
/// output file (say `logo.png` and `logo.jpg`, both becoming `logo.ico`):
/// one would silently overwrite the other.
fn plan_jobs(
    files: Vec<PathBuf>,
    folder: Option<&Path>,
    use_icns: bool,
) -> Result<Vec<Job>, String> {
    let extension = if use_icns { "icns" } else { "ico" };
    let mut claimed: HashMap<String, PathBuf> = HashMap::new();
    let mut jobs = Vec::with_capacity(files.len());
    for input in files {
        let output = batch_output_path(folder, &input, extension);
        if let Some(first) = claimed.get(&collision_key(&output)) {
            return Err(format!(
                "'{}' and '{}' would both be written to '{}'. Rename one of them, or convert them separately.",
                first.display(),
                input.display(),
                output.display()
            ));
        }
        claimed.insert(collision_key(&output), input.clone());
        jobs.push(Job { input, output });
    }
    Ok(jobs)
}

/// Rejects options that make no sense for a batch.
fn check_batch_options(output: Option<&Path>, resolved: &ResolvedSettings) -> Result<(), String> {
    // A --find preview prints a report for ONE image. Several of them
    // would be an undifferentiated pile of reports, and the preview isn't
    // a conversion, so it doesn't belong in one.
    if resolved.find.is_some() && !resolved.auto_apply {
        return Err(
            "--find without --auto-apply prints a report for one image and can't be used with several inputs or a folder. Run it for each file separately, or add --auto-apply."
                .to_string(),
        );
    }

    // In a batch -o names a FOLDER. Catch the two easy mix-ups instead of
    // quietly creating a folder called "icon.ico" or failing obscurely.
    if let Some(output) = output {
        if output.exists() && !output.is_dir() {
            return Err(format!(
                "With several inputs (or a folder) -o names an output folder, but '{}' is a file.",
                output.display()
            ));
        }
        let looks_like_icon_file = output.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ico") || extension.eq_ignore_ascii_case("icns")
        });
        if !output.exists() && looks_like_icon_file {
            return Err(format!(
                "With several inputs (or a folder) -o names an output folder, but '{}' looks like a file name. To convert one file to that name, give just that one input.",
                output.display()
            ));
        }
    }
    Ok(())
}

/// Rejects option combinations that contradict each other or make no
/// sense on their own.
fn check_options(resolved: &ResolvedSettings) -> Result<(), String> {
    check_color_options(resolved)?;
    // --force overwrites an existing output; --skip-existing leaves it
    // alone. They can't both decide what happens to the same file. (On the
    // command line clap already refuses the pair; this also catches one of
    // them coming from a settings file.)
    if resolved.force && resolved.skip_existing {
        return Err(
            "--force and --skip-existing contradict each other: one overwrites an existing output, the other leaves it alone."
                .to_string(),
        );
    }
    Ok(())
}

/// Rejects background-color options that contradict each other or make
/// no sense on their own.
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
        eprintln!(
            "Warning: {}--gif-frame only applies to GIF input and is ignored for this file.",
            file_prefix()
        );
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
    let prefix = file_prefix();
    if regions.is_empty() {
        println!(
            "{prefix}No additional regions matching {find_hex} found - the border-based flood fill should already reach everything."
        );
        return;
    }

    println!(
        "{prefix}Found {} additional region(s) matching {find_hex} that the border-based flood fill can't reach on its own:",
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
    let started = Instant::now();
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
    resolved.note(format_args!(
        "background removal ({flag} {hex}, {} seed point(s)) took {:.1?}",
        seeds.len(),
        started.elapsed()
    ));
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
                eprintln!(
                    "{}Skipping size {size} (valid range: 1-{MAX_ICO_SIZE}).",
                    file_prefix()
                );
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
        resolved.note(format_args!(
            "{}{size}x{size}: {} bytes",
            file_prefix(),
            entry.data().len()
        ));
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

    // --- Planning a batch ----------------------------------------------------------------

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"x").unwrap();
        path
    }

    fn names(files: &[PathBuf]) -> Vec<String> {
        files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn supported_images_are_recognized_by_extension_ignoring_case() {
        for yes in [
            "a.png",
            "a.PNG",
            "a.jpg",
            "a.JPEG",
            "a.bmp",
            "a.gif",
            "dir/a.Png",
        ] {
            assert!(is_supported_image(Path::new(yes)), "{yes}");
        }
        for no in ["a.ico", "a.txt", "a.pngx", "png", "a", "a."] {
            assert!(!is_supported_image(Path::new(no)), "{no}");
        }
    }

    #[test]
    fn one_file_is_a_single_conversion_even_if_it_does_not_exist() {
        let inputs = vec![PathBuf::from("missing.png")];
        assert!(
            matches!(plan_inputs(&inputs), Ok(Plan::Single(path)) if path == Path::new("missing.png"))
        );
    }

    #[test]
    fn several_files_or_a_folder_are_a_batch() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "a.png");

        let two = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        assert!(matches!(plan_inputs(&two), Ok(Plan::Batch(files)) if files.len() == 2));

        let folder = vec![dir.path().to_path_buf()];
        assert!(matches!(plan_inputs(&folder), Ok(Plan::Batch(files)) if files.len() == 1));
    }

    #[test]
    fn a_folder_contributes_its_images_in_name_order() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "c.png",
            "a.PNG",
            "b.jpg",
            "notes.txt",
            "icon.ico",
            "d.gif",
            "e.bmp",
            "f.jpeg",
        ] {
            touch(dir.path(), name);
        }
        std::fs::create_dir(dir.path().join("sub.png")).unwrap(); // a folder, not an image
        touch(&dir.path().join("sub.png"), "inner.png"); // subfolders are not searched

        let files = expand_inputs(&[dir.path().to_path_buf()]).unwrap();
        assert_eq!(
            names(&files),
            ["a.PNG", "b.jpg", "c.png", "d.gif", "e.bmp", "f.jpeg"]
        );
    }

    #[test]
    fn files_and_folders_can_be_mixed_and_keep_the_given_order() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "x.png");
        touch(dir.path(), "y.png");
        let inputs = vec![
            PathBuf::from("first.png"),
            dir.path().to_path_buf(),
            PathBuf::from("last.png"),
        ];
        let files = expand_inputs(&inputs).unwrap();
        assert_eq!(names(&files), ["first.png", "x.png", "y.png", "last.png"]);
    }

    #[test]
    fn a_folder_without_images_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "notes.txt");
        let err = expand_inputs(&[dir.path().to_path_buf()]).unwrap_err();
        assert!(err.contains("No supported images"), "{err}");
    }

    #[test]
    fn batch_outputs_go_next_to_the_input_or_into_the_folder() {
        let input = Path::new("art").join("logo.png");
        assert_eq!(
            batch_output_path(None, &input, "ico"),
            Path::new("art").join("logo.ico")
        );
        assert_eq!(
            batch_output_path(Some(Path::new("out")), &input, "icns"),
            Path::new("out").join("logo.icns")
        );
        assert_eq!(
            batch_output_path(None, Path::new("logo.png"), "ico"),
            Path::new("logo.ico")
        );
    }

    #[test]
    fn dots_in_the_file_name_are_kept() {
        let out = batch_output_path(
            Some(Path::new("out")),
            Path::new("logo.v2.final.png"),
            "ico",
        );
        assert_eq!(out, Path::new("out").join("logo.v2.final.ico"));
    }

    #[test]
    fn jobs_pair_every_input_with_its_output() {
        let files = vec![PathBuf::from("a.png"), PathBuf::from("b.jpg")];
        let jobs = plan_jobs(files, Some(Path::new("out")), false).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].output, Path::new("out").join("a.ico"));
        assert_eq!(jobs[1].output, Path::new("out").join("b.ico"));
    }

    #[test]
    fn two_inputs_that_would_share_an_output_are_refused() {
        let files = vec![PathBuf::from("logo.png"), PathBuf::from("logo.jpg")];
        let err = plan_jobs(files, None, false).map(|_| ()).unwrap_err();
        assert!(
            err.contains("logo.png") && err.contains("logo.jpg"),
            "{err}"
        );
        assert!(err.contains("would both be written to"), "{err}");

        let same_twice = vec![PathBuf::from("a.png"), PathBuf::from("a.png")];
        assert!(plan_jobs(same_twice, None, false).is_err());
    }

    #[test]
    fn the_same_name_in_different_places_is_not_a_collision() {
        let files = vec![
            Path::new("one").join("logo.png"),
            Path::new("two").join("logo.png"),
        ];
        assert!(
            plan_jobs(files, None, false).is_ok(),
            "next to their inputs they differ"
        );

        let files = vec![
            Path::new("one").join("logo.png"),
            Path::new("two").join("logo.png"),
        ];
        assert!(
            plan_jobs(files, Some(Path::new("out")), false).is_err(),
            "in one folder they clash"
        );
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn where_the_file_system_ignores_case_differing_case_is_a_collision() {
        let files = vec![PathBuf::from("Logo.png"), PathBuf::from("logo.png")];
        assert!(plan_jobs(files, Some(Path::new("out")), false).is_err());
    }

    #[test]
    fn the_extension_follows_the_output_format() {
        let jobs = plan_jobs(vec![PathBuf::from("a.png")], None, true).unwrap();
        assert_eq!(jobs[0].output, Path::new("a.icns"));
    }

    #[test]
    fn a_find_preview_cannot_run_as_a_batch() {
        let err =
            with_resolved(&["--find", "00FF00"], |r| check_batch_options(None, r)).unwrap_err();
        assert!(err.contains("--find without --auto-apply"), "{err}");
        assert!(
            with_resolved(&["--find", "00FF00", "--auto-apply"], |r| {
                check_batch_options(None, r)
            })
            .is_ok()
        );
    }

    #[test]
    fn in_a_batch_output_must_be_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = touch(dir.path(), "existing.ico");
        let check = |out: &Path| with_resolved(&[], |r| check_batch_options(Some(out), r));

        assert!(check(dir.path()).is_ok(), "an existing folder");
        assert!(
            check(&dir.path().join("new-folder")).is_ok(),
            "a folder to be created"
        );
        assert!(check(&file).unwrap_err().contains("is a file"));
        let looks_like_file = dir.path().join("out.ico");
        assert!(
            check(&looks_like_file)
                .unwrap_err()
                .contains("looks like a file name")
        );
        assert!(
            check(&dir.path().join("OUT.ICNS"))
                .unwrap_err()
                .contains("looks like a file name")
        );
    }

    #[test]
    fn force_and_skip_existing_contradict_each_other_when_both_are_set() {
        let force = Settings {
            force: true,
            skip_existing: true,
            ..Settings::default()
        };
        let cli = parsed(&[]);
        let resolved = ResolvedSettings::resolve(&cli, &force);
        let err = check_options(&resolved).unwrap_err();
        assert!(err.contains("--force and --skip-existing"), "{err}");
        assert!(with_resolved(&["--skip-existing"], check_options).is_ok());
        assert!(with_resolved(&["--force"], check_options).is_ok());
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
