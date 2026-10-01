// The normal mode: turning source images into .ico (or .icns) files.
//
// With ONE input file that is a single conversion; with several inputs, or
// a folder, it is a batch of them ("Batch" below). Either way every file
// goes through the same `convert_one`, which reads top to bottom as the
// pipeline itself - load the image, optionally --find regions, recolor,
// warn, write - with every step a small function of its own. What differs
// between the two modes is only the planning before it (plan.rs, select.rs)
// and the bookkeeping after it (the progress lines, the summary, the
// --report file).

use crate::chroma_key::{
    FoundRegion, apply_chroma_key_feathered, detect_background_color, find_isolated_regions,
    format_hex, parse_hex_color, warn_about_removal_extent,
};
use crate::cli::{Args, OutputFormat};
use crate::gif::{extract_gif_frame, is_gif};
use crate::icns::{icns_sizes, write_icns};
use crate::plan::{Job, Naming, check_batch_options, check_folder_options, plan_jobs, single_job};
use crate::report::{
    FileRecord, ReportFormat, Status, Summary, human_duration, human_size, write_report,
};
use crate::resize::{
    apply_grayscale, has_transparency, make_square_icon, warn_about_thin_content,
    warn_about_upscaling,
};
use crate::select::{Filter, NamePattern, Source, expand_inputs};
use crate::settings::{
    ResolvedSettings, delete_sources_if_requested, finish_run, maybe_write_out_toml,
};
use crate::util::{
    check_overwrite, enter_file_context, file_prefix, parse_seed, warn, warnings_so_far,
};
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The sizes used when neither --sizes, a config file nor --preset says
/// otherwise.
const DEFAULT_SIZES: &str = "16,32,48,64,128,256";

/// ICO files officially only support edge lengths up to 256px.
const MAX_ICO_SIZE: u32 = 256;

/// What became of one input.
enum Outcome {
    /// The icon was written: its size in bytes and the icon sizes inside.
    Converted { bytes: u64, sizes: Vec<u32> },
    /// The output already existed and --skip-existing said to leave it.
    Skipped,
    /// A --find preview: the report was printed, nothing was written.
    Previewed,
}

/// What the command line asks for: one file, or a batch of them.
enum Plan<'a> {
    Single(&'a Path),
    Batch(Vec<Source>),
}

/// Runs the complete image-to-icon conversion for the parsed `args` and
/// the `resolved` settings (command line + config file + defaults).
pub fn run(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    // Everything that can be checked without touching an image comes first,
    // so a mistake in the options costs no work.
    let pattern = args.name.as_deref().map(NamePattern::parse).transpose()?;
    let filter = Filter::new(resolved.include, resolved.exclude)?;
    if let Some(path) = args.report.as_deref() {
        ReportFormat::of(path)?;
    }
    check_folder_options(args)?;
    let plan = plan_inputs(&args.input, resolved.recursive, &filter)?;

    check_options(resolved)?;
    let replacement = resolved
        .replace_color
        .map(|hex| parse_hex_color(hex).map_err(|e| format!("Invalid --replace-color value: {e}")))
        .transpose()?;

    let use_icns = wants_icns(resolved.output_format);
    match plan {
        Plan::Single(input) => {
            let job = single_job(args.output.as_deref(), input, pattern.as_ref(), use_icns)?;
            if args.what_if {
                return what_if(&[job], resolved);
            }
            run_single(args, resolved, replacement, job)
        }
        Plan::Batch(sources) => {
            check_batch_options(args.output.as_deref(), resolved)?;
            let naming = Naming {
                folder: args.output.as_deref(),
                keep_structure: args.keep_structure,
                pattern: pattern.as_ref(),
                use_icns,
            };
            let jobs = plan_jobs(sources, &naming)?;
            if args.what_if {
                return what_if(&jobs, resolved);
            }
            run_batch(args, resolved, replacement, jobs)
        }
    }
}

/// Decides whether the inputs are one file or a batch. One input that is
/// not a folder is a single conversion - including a path that doesn't
/// exist, which `convert_one` then reports as unreadable, as ever. Anything
/// else (several inputs, or a folder) is a batch of the files they stand
/// for.
fn plan_inputs<'a>(
    inputs: &'a [PathBuf],
    recursive: bool,
    filter: &Filter,
) -> Result<Plan<'a>, String> {
    if let [only] = inputs
        && !only.is_dir()
    {
        return Ok(Plan::Single(only));
    }
    expand_inputs(inputs, recursive, filter).map(Plan::Batch)
}

/// Converts one file: `-o` (if given) is the output FILE.
fn run_single(
    args: &Args,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    job: Job,
) -> Result<(), String> {
    let started = Instant::now();
    let (result, record) = run_job(&job, resolved, replacement);

    let ran = match result {
        Ok(Outcome::Converted { sizes, .. }) => {
            println!(
                "{}",
                done_line(&job.output, &sizes, wants_icns(resolved.output_format))
            );
            finish_run(
                args,
                resolved,
                std::slice::from_ref(&job.input),
                Some(&job.output),
            )
        }
        // Nothing was written, so there is nothing to clean up and no
        // settings snapshot worth saving.
        Ok(Outcome::Skipped) => {
            if !resolved.silent {
                eprintln!("{}", skip_line(&job));
            }
            Ok(())
        }
        Ok(Outcome::Previewed) => Ok(()),
        Err(message) => Err(message),
    };
    finish_report(args, &[record], started.elapsed(), ran)
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
    jobs: Vec<Job>,
) -> Result<(), String> {
    resolved.note(format_args!("batch: {} file(s)", jobs.len()));

    if let Some(folder) = args.output.as_deref() {
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("Could not create output folder '{}': {e}", folder.display()))?;
    }

    let started = Instant::now();
    let total = jobs.len();
    let use_icns = wants_icns(resolved.output_format);
    let mut records: Vec<FileRecord> = Vec::with_capacity(total);
    let mut converted: Vec<&Job> = Vec::new();
    let mut stopped_by: Option<String> = None;

    for (index, job) in jobs.iter().enumerate() {
        // Lets warnings raised deep inside the conversion name this file.
        let _context = enter_file_context(&job.input);
        let (result, mut record) = run_job(job, resolved, replacement);
        let count = progress_prefix(index + 1, total);

        match result {
            Ok(Outcome::Converted { bytes, sizes }) => {
                if !resolved.silent {
                    println!(
                        "{count}{}",
                        converted_line(job, bytes, &sizes, total, use_icns)
                    );
                }
                converted.push(job);
            }
            Ok(Outcome::Skipped | Outcome::Previewed) => {
                if !resolved.silent {
                    eprintln!("{count}{}", skip_line(job));
                }
            }
            Err(message) => {
                // Name the file, unless the message already does.
                let name = job.input.display().to_string();
                let message = if message.contains(&name) {
                    message
                } else {
                    format!("{name}: {message}")
                };
                record.message = Some(message.clone());
                if !resolved.keep_going {
                    stopped_by = Some(message);
                } else {
                    eprintln!("{count}Error: {message}");
                }
            }
        }
        records.push(record);
        if stopped_by.is_some() {
            break;
        }
    }

    let summary = Summary::of(&records, started.elapsed());
    let ran = match stopped_by {
        Some(message) => Err(message),
        None => {
            // --silent hides the summary of a clean run - but a failure is
            // never something to be quiet about.
            if !resolved.silent || summary.failed > 0 {
                print_summary(&summary);
            }
            if summary.failed > 0 {
                Err(format!("{} of {total} file(s) failed.", summary.failed))
            } else {
                for job in converted {
                    delete_sources_if_requested(
                        resolved,
                        std::slice::from_ref(&job.input),
                        Some(&job.output),
                    );
                }
                maybe_write_out_toml(args.out_toml.as_deref(), resolved)
            }
        }
    };
    finish_report(args, &records, started.elapsed(), ran)
}

/// --what-if: says, for every job, what a real run would do - the output it
/// would write, the icon sizes, and what an output that already exists
/// means (skipped, replaced or refused) - without loading an image or
/// writing a byte. Fails if any job would.
fn what_if(jobs: &[Job], resolved: &ResolvedSettings) -> Result<(), String> {
    let sizes = if wants_icns(resolved.output_format) {
        icns_sizes()
    } else {
        let sizes: Vec<u32> = resolve_sizes(resolved)?
            .into_iter()
            .filter(|&size| (1..=MAX_ICO_SIZE).contains(&size))
            .collect();
        if sizes.is_empty() {
            return Err("At least one size must be given.".to_string());
        }
        sizes
    };

    let total = jobs.len();
    let (mut convert, mut skip, mut fail) = (0usize, 0usize, 0usize);
    for (index, job) in jobs.iter().enumerate() {
        let verdict = if !job.input.is_file() {
            fail += 1;
            "would fail: input file not found".to_string()
        } else if job.output.exists() {
            if resolved.skip_existing {
                skip += 1;
                "would skip: the output already exists".to_string()
            } else if resolved.force {
                convert += 1;
                format!("would convert, replacing the existing output, sizes {sizes:?}")
            } else {
                fail += 1;
                "would fail: the output already exists (--force replaces it, --skip-existing leaves it)"
                    .to_string()
            }
        } else {
            convert += 1;
            format!("would convert, sizes {sizes:?}")
        };
        println!(
            "{}{} -> {} ({verdict})",
            progress_prefix(index + 1, total),
            job.input.display(),
            job.output.display()
        );
    }
    println!(
        "What if: {convert} would be converted, {skip} skipped, {fail} would fail. Nothing was written."
    );
    if fail > 0 {
        return Err(format!("{fail} of {total} file(s) would fail."));
    }
    Ok(())
}

/// "[3/20] " while working through several files, nothing for one.
fn progress_prefix(position: usize, total: usize) -> String {
    if total > 1 {
        format!("[{position}/{total}] ")
    } else {
        String::new()
    }
}

/// The line after a single file was converted.
fn done_line(output: &Path, sizes: &[u32], use_icns: bool) -> String {
    if use_icns {
        format!(
            "Done: '{}' created with {} icon size(s) (icns format).",
            output.display(),
            sizes.len()
        )
    } else {
        format!("Done: '{}' created with sizes {sizes:?}.", output.display())
    }
}

/// The progress line after a file of a batch was converted:
/// "[3/20] assets/logo.png -> icons/logo.ico (42 KB)". (A batch of just one
/// file reads like a single conversion.)
fn converted_line(job: &Job, bytes: u64, sizes: &[u32], total: usize, use_icns: bool) -> String {
    if total <= 1 {
        return done_line(&job.output, sizes, use_icns);
    }
    format!(
        "{} -> {} ({})",
        job.input.display(),
        job.output.display(),
        human_size(bytes)
    )
}

/// What is said about an input left alone because of --skip-existing.
fn skip_line(job: &Job) -> String {
    format!(
        "Skipping '{}': '{}' already exists (--skip-existing).",
        job.input.display(),
        job.output.display()
    )
}

/// The totals line(s) at the end of a batch.
fn print_summary(summary: &Summary) {
    let warnings = if summary.warnings > 0 {
        format!(", {} warning(s)", summary.warnings)
    } else {
        String::new()
    };
    println!(
        "Batch finished: {} converted, {} skipped, {} failed{warnings} ({} file(s) in total).",
        summary.converted, summary.skipped, summary.failed, summary.total
    );
    if summary.converted > 0 {
        println!(
            "Wrote {} icon file(s), {} in all, in {}.",
            summary.converted,
            human_size(summary.output_bytes),
            human_duration(summary.duration_ms)
        );
    }
}

/// Runs one job, timing it and counting the warnings it raises, and returns
/// its outcome together with the report line describing it. Prints nothing
/// itself: what to say about the outcome depends on whether it is part of a
/// batch.
fn run_job(
    job: &Job,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
) -> (Result<Outcome, String>, FileRecord) {
    let started = Instant::now();
    let warnings_before = warnings_so_far();
    let result = convert_one(job, resolved, replacement);

    let mut record = FileRecord {
        input: job.input.display().to_string(),
        output: job.output.display().to_string(),
        status: Status::Failed,
        size_bytes: None,
        sizes: Vec::new(),
        warnings: warnings_so_far() - warnings_before,
        duration_ms: started.elapsed().as_millis() as u64,
        message: None,
    };
    match &result {
        Ok(Outcome::Converted { bytes, sizes }) => {
            record.status = Status::Converted;
            record.size_bytes = Some(*bytes);
            record.sizes = sizes.clone();
        }
        Ok(Outcome::Skipped) => {
            record.status = Status::Skipped;
            record.message = Some("the output already exists (--skip-existing)".to_string());
        }
        Ok(Outcome::Previewed) => record.status = Status::Previewed,
        Err(message) => record.message = Some(message.clone()),
    }
    (result, record)
}

/// Writes the --report file, if one was asked for, as the last step of a
/// run - whether it went well or not, since a report of a failed run is
/// the one most worth having. If the run itself failed, that failure is
/// what comes back; a problem with the report is then only mentioned.
fn finish_report(
    args: &Args,
    records: &[FileRecord],
    elapsed: Duration,
    ran: Result<(), String>,
) -> Result<(), String> {
    let Some(path) = args.report.as_deref() else {
        return ran;
    };
    let summary = Summary::of(records, elapsed);
    match (write_report(path, records, &summary), ran) {
        (Ok(()), ran) => {
            if !args.silent {
                println!("Report written to '{}'.", path.display());
            }
            ran
        }
        (Err(report_error), Ok(())) => Err(report_error),
        (Err(report_error), Err(run_error)) => {
            eprintln!("{report_error}");
            Err(run_error)
        }
    }
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

    // The background color, if the options name one: given as a hex code, or
    // detected from the image border with "auto".
    let background = resolve_background(&source, resolved)?;

    // --find: look for regions matching this color that the border-based
    // flood fill in apply_chroma_key can't reach on its own (the same
    // situation --seed manually solves, just discovered automatically).
    // Without --auto-apply, this is the ENTIRE effect of --find: print
    // the suggestions and stop, without converting anything (and without
    // --out-toml writing anything either - nothing was actually decided
    // about the FINAL settings yet in that case).
    let mut discovered_seeds: Vec<(u32, u32)> = Vec::new();
    if let Some(target) = background.as_ref().filter(|target| target.flag == "--find") {
        let regions = find_regions(&source, target, resolved);
        print_found_regions(&target.label, &regions);

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
    if job.make_dirs
        && let Some(folder) = output_path.parent().filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("Could not create output folder '{}': {e}", folder.display()))?;
    }

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

    remove_background(
        &mut source,
        resolved,
        replacement,
        discovered_seeds,
        background.as_ref(),
    )?;

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
    let written_sizes = if use_icns {
        write_icns(&source, resolved.padding, has_alpha, output_path)?;
        icns_sizes()
    } else {
        write_ico(&source, &sizes, has_alpha, resolved, output_path)?
    };
    let bytes = std::fs::metadata(output_path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    resolved.note(format_args!(
        "{}finished in {:.1?}",
        file_prefix(),
        started.elapsed()
    ));
    Ok(Outcome::Converted {
        bytes,
        sizes: written_sizes,
    })
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

    if resolved.gif_frame != 1 {
        warn(
            resolved.silent,
            format_args!(
                "Warning: {}--gif-frame only applies to GIF input and is ignored for this file.",
                file_prefix()
            ),
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

/// The background color of a run and how to refer to it.
#[derive(Debug)]
struct BackgroundTarget {
    /// The option that named it: `--chroma-key` or `--find`.
    flag: &'static str,
    color: [u8; 3],
    /// What to call the color in messages: the hex code as the user wrote
    /// it, or `#RRGGBB` for a detected one.
    label: String,
}

/// Works out the background color of this run, if --chroma-key or --find
/// names one: a hex code is parsed, and "auto" is detected from the border
/// of `source` (and announced, so it can be passed explicitly next time).
/// In a batch every file detects its own.
fn resolve_background(
    source: &RgbaImage,
    resolved: &ResolvedSettings,
) -> Result<Option<BackgroundTarget>, String> {
    // The two are mutually exclusive (see check_color_options), so at most
    // one of them names the background color.
    let (flag, spec) = match (resolved.chroma_key, resolved.find) {
        (Some(spec), _) => ("--chroma-key", spec.trim()),
        (None, Some(spec)) => ("--find", spec.trim()),
        (None, None) => return Ok(None),
    };

    if !spec.eq_ignore_ascii_case("auto") {
        let color = parse_hex_color(spec).map_err(|e| format!("Invalid {flag} value: {e}"))?;
        return Ok(Some(BackgroundTarget {
            flag,
            color,
            label: spec.to_string(),
        }));
    }

    let started = Instant::now();
    let found = detect_background_color(source, resolved.tolerance)?;
    let label = format_hex(found.color);
    if !resolved.silent {
        println!(
            "{}Detected background color {label} ({:.0}% of the image border). To use it explicitly: {flag} {}",
            file_prefix(),
            found.coverage * 100.0,
            label.trim_start_matches('#')
        );
    }
    resolved.note(format_args!(
        "{}background detection took {:.1?}",
        file_prefix(),
        started.elapsed()
    ));
    Ok(Some(BackgroundTarget {
        flag,
        color: found.color,
        label,
    }))
}

/// Runs --find's region discovery on the source image.
fn find_regions(
    source: &RgbaImage,
    target: &BackgroundTarget,
    resolved: &ResolvedSettings,
) -> Vec<FoundRegion> {
    find_isolated_regions(
        source,
        target.color,
        resolved.tolerance,
        resolved.find_min_size,
        resolved.silent,
    )
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

/// Applies the actual chroma-key removal/replacement, if a `background`
/// color was named: --chroma-key (with whatever manual --seed values were
/// given), or --find --auto-apply (the manual --seed values PLUS the
/// automatically `discovered` regions from the --find preview, combined).
/// Does nothing without one. `replacement` (from --replace-color) is passed
/// through to both. Warns if almost nothing, or almost everything, matched.
fn remove_background(
    img: &mut RgbaImage,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    discovered: Vec<(u32, u32)>,
    background: Option<&BackgroundTarget>,
) -> Result<(), String> {
    // The --find case only gets here with --auto-apply - the plain preview
    // already returned.
    let Some(target) = background else {
        return Ok(());
    };

    let started = Instant::now();
    let mut seeds = parse_seeds(resolved.seeds)?;
    seeds.extend(discovered);
    let affected = apply_chroma_key_feathered(
        img,
        target.color,
        resolved.tolerance,
        resolved.feather,
        &seeds,
        replacement,
        resolved.silent,
    );
    warn_about_removal_extent(
        affected,
        img.width() as usize * img.height() as usize,
        &target.label,
        resolved.silent,
    );
    resolved.note(format_args!(
        "background removal ({} {}, {} seed point(s), feather {}%) took {:.1?}",
        target.flag,
        target.label,
        seeds.len(),
        resolved.feather,
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
) -> Result<Vec<u32>, String> {
    // An IconDir collects all the resolutions that will be written
    // together into ONE .ico file at the end.
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    let mut written = Vec::with_capacity(sizes.len());

    for &size in sizes {
        if size == 0 || size > MAX_ICO_SIZE {
            warn(
                resolved.silent,
                format_args!(
                    "{}Skipping size {size} (valid range: 1-{MAX_ICO_SIZE}).",
                    file_prefix()
                ),
            );
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
        written.push(size);
    }

    // Create the target file and write all the collected resolutions into it.
    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Could not create output file: {e}"))?;
    icon_dir
        .write(file)
        .map_err(|e| format!("Error writing ICO file: {e}"))?;

    Ok(written)
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

    // --- background color ------------------------------------------------------------

    /// What a conversion does for the background: name the color, then remove it.
    fn remove(
        img: &mut RgbaImage,
        resolved: &ResolvedSettings,
        replacement: Option<[u8; 3]>,
        discovered: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let background = resolve_background(img, resolved)?;
        remove_background(img, resolved, replacement, discovered, background.as_ref())
    }

    #[test]
    fn auto_detects_the_border_color_and_labels_it_in_hex() {
        let mut img = RgbaImage::from_pixel(8, 8, image::Rgba([0, 255, 0, 255]));
        img.put_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        let target = with_resolved(&["--chroma-key", "auto"], |resolved| {
            resolve_background(&img, resolved).unwrap().unwrap()
        });
        assert_eq!(target.color, [0, 255, 0]);
        assert_eq!(target.label, "#00FF00");
        assert_eq!(target.flag, "--chroma-key");
    }

    #[test]
    fn auto_is_case_insensitive_and_works_for_find_too() {
        let img = RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]));
        for flags in [&["-c", "AUTO"][..], &["--find", "Auto"]] {
            let target = with_resolved(flags, |resolved| {
                resolve_background(&img, resolved).unwrap().unwrap()
            });
            assert_eq!(target.color, [10, 20, 30], "{flags:?}");
        }
        let find = with_resolved(&["--find", "auto"], |resolved| {
            resolve_background(&img, resolved).unwrap().unwrap()
        });
        assert_eq!(find.flag, "--find");
    }

    #[test]
    fn a_fixed_color_is_taken_as_written() {
        let img = RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]));
        let target = with_resolved(&["-c", " #00ff00 "], |resolved| {
            resolve_background(&img, resolved).unwrap().unwrap()
        });
        assert_eq!(target.color, [0, 255, 0]);
        assert_eq!(target.label, "#00ff00");
        assert!(
            with_resolved(&[], |resolved| resolve_background(&img, resolved))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn auto_on_a_gradient_gives_a_clear_message_naming_the_best_guess() {
        let mut img = RgbaImage::new(64, 64);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x * 4) as u8, 0, 255 - (x * 4) as u8, 255]);
        }
        let err = with_resolved(&["-c", "auto", "-t", "5"], |resolved| {
            resolve_background(&img, resolved).unwrap_err()
        });
        assert!(
            err.contains("Could not detect a single background color"),
            "{err}"
        );
        assert!(
            err.contains('#') && err.contains("--chroma-key #RRGGBB"),
            "{err}"
        );
    }

    #[test]
    fn auto_then_removal_clears_the_detected_background() {
        let mut img = RgbaImage::from_pixel(9, 9, image::Rgba([0, 255, 0, 255]));
        for y in 3..6 {
            for x in 3..6 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        with_resolved(&["-c", "auto", "--silent"], |resolved| {
            remove(&mut img, resolved, None, vec![]).unwrap();
        });
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(4, 4)[3], 255, "the artwork stays");
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

    // --- output format ---------------------------------------------------------------

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
            remove(&mut img, resolved, None, vec![]).unwrap();
        });
        assert_eq!(img, before);
    }

    #[test]
    fn remove_background_uses_chroma_key_and_reports_a_bad_color_by_flag() {
        let mut img = RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        with_resolved(&["--chroma-key", "00FF00"], |resolved| {
            remove(&mut img, resolved, None, vec![]).unwrap();
        });
        assert!(img.pixels().all(|p| p[3] == 0));

        let err = with_resolved(&["--chroma-key", "nope"], |resolved| {
            remove(&mut img, resolved, None, vec![]).unwrap_err()
        });
        assert!(err.contains("Invalid --chroma-key value"), "{err}");

        let err = with_resolved(&["--find", "nope", "--auto-apply"], |resolved| {
            remove(&mut img, resolved, None, vec![]).unwrap_err()
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
            remove(&mut img, resolved, None, vec![(4, 4)]).unwrap();
        });
        assert_eq!(img.get_pixel(4, 4)[3], 0, "the discovered seed was used");
        assert_eq!(img.get_pixel(0, 0)[3], 0, "the border was cleared too");
    }
}
