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

use crate::chroma_key::{FoundRegion, parse_hex_color};
use crate::cli::{Args, OutputFormat};
use crate::gif::{extract_gif_frame_limited, is_gif, is_gif_bytes};
use crate::icns::icns_sizes;
use crate::layout::{Layout, parse_crop};
use crate::plan::{Job, Naming, check_batch_options, check_folder_options, plan_jobs, single_job};
use crate::report::{
    FileRecord, ReportFormat, Status, Summary, human_duration, human_size, write_report,
};
use crate::select::{Filter, NamePattern, Source, expand_inputs};
use crate::settings::{
    ResolvedSettings, delete_sources_if_requested, finish_run, maybe_write_out_toml,
};
use crate::source::{Artwork, decode_source_bytes, open_source, peek_pixels};
use crate::util::{
    Budget, available_threads, check_overwrite, enter_file_context, file_prefix, for_each_ordered,
    parse_seed, same_file, warn, warnings_so_far,
};
use crate::vector::VectorImage;
use image::RgbaImage;
use img2ico_core::convert::{
    Background, BackgroundMode, Format, MAX_ICO_SIZE, Notes, Options, Sizes, analyze_raster,
    convert_raster, convert_vector as convert_drawing, vector_refusal,
};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The sizes used when neither --sizes, a config file nor --preset says
/// otherwise.
const DEFAULT_SIZES: &str = "16,32,48,64,128,256";

/// How many threads this run works with. An explicit --jobs (or `jobs` in a
/// settings file) is taken as given, 0 meaning one per processor. Without
/// one, it is one per processor - except for a run whose output only makes
/// sense in sequence: --verbose (its trace of one file after the other) and a
/// --find preview (it prints a multi-line report per file).
pub fn worker_count(resolved: &ResolvedSettings) -> usize {
    match resolved.jobs {
        Some(0) | None => {
            let find_preview = resolved.find.is_some() && !resolved.auto_apply;
            if resolved.verbose || find_preview {
                1
            } else {
                available_threads()
            }
        }
        Some(jobs) => jobs,
    }
}

/// How many pixels of source images may be held decoded by all workers
/// together: 150 million, which is some 1.2 GB with the working copies. A
/// batch of large photos then runs a few at a time instead of all at once.
const DECODED_PIXELS_IN_FLIGHT: u64 = 150_000_000;

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
    check_stdio(args, resolved)?;
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

/// Whether `path` is the single dash, which stands for standard input (as an
/// input) or standard output (as `-o`). A file really called "-" is written
/// `./-`, which is a different path.
fn is_stdio(path: &Path) -> bool {
    path == Path::new("-")
}

/// Checks the use of standard input (`-` as the input) and standard output
/// (`-o -`), which are for converting ONE image into ONE icon:
///   - standard input is the single input, so it can't be mixed with others;
///   - it has no file name to derive an output name from, so -o is needed -
///     except for a --find preview, which writes no icon at all;
///   - --delete-source has nothing to delete for a stream, and cannot know
///     where an icon sent to standard output ended up;
///   - standard output takes one icon, not a folder's worth;
///   - a --find preview writes no icon, so there is nothing to send.
fn check_stdio(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    let from_stdin = args.input.iter().any(|input| is_stdio(input));
    let to_stdout = args.output.as_deref().is_some_and(is_stdio);
    let find_preview = resolved.find.is_some() && !resolved.auto_apply;

    if from_stdin {
        if args.input.len() != 1 {
            return Err(
                "'-' (standard input) is one image and can't be combined with other inputs. A file really called '-' can be written as './-'."
                    .to_string(),
            );
        }
        if resolved.delete_source {
            return Err(
                "--delete-source can't be used with standard input: there is no file to delete."
                    .to_string(),
            );
        }
        if args.output.is_none() && !find_preview {
            return Err(
                "Reading from standard input needs -o: there is no input file name to derive the output name from. (Use -o - to write to standard output.)"
                    .to_string(),
            );
        }
    }
    if to_stdout {
        if args.input.len() != 1 || args.input.iter().any(|input| input.is_dir()) {
            return Err(
                "'-o -' (standard output) takes one icon, so it works for one input image - not for several inputs or a folder."
                    .to_string(),
            );
        }
        if resolved.delete_source {
            return Err(
                "--delete-source can't be used with '-o -': it can't be known where the icon ended up."
                    .to_string(),
            );
        }
        if find_preview {
            return Err(
                "--find without --auto-apply writes no icon, so there is nothing to send to standard output. Add --auto-apply, or leave out '-o -'."
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// Added to a "can't decode" error for an image from standard input on
/// Windows, where the likeliest cause is Windows PowerShell 5.1: it turns
/// what it pipes into a native program into text, which damages binary data.
#[cfg(windows)]
const STDIN_DECODE_HINT: &str = "
Hint: if this was piped in from Windows PowerShell, it converts binary data on the way. Let cmd do the redirect instead (cmd /c \"img2ico - -o icon.ico < logo.png\"), or give the file as the input.";
#[cfg(not(windows))]
const STDIN_DECODE_HINT: &str = "";

/// Reads the whole of standard input - the image to convert.
fn read_stdin() -> Result<Vec<u8>, String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Err(
            "'-' reads an image from standard input, but standard input is a terminal. Pipe or redirect an image in, for example: cat logo.png | img2ico - -o icon.ico"
                .to_string(),
        );
    }
    let mut bytes = Vec::new();
    stdin
        .lock()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not read standard input: {e}"))?;
    if bytes.is_empty() {
        return Err("Standard input is empty - there is no image to convert.".to_string());
    }
    Ok(bytes)
}

/// Writes the finished icon: to the file `output`, or - for `-o -` - to
/// standard output. (The messages for a file keep their long-standing
/// wording.)
fn write_output(output: &Path, bytes: &[u8], use_icns: bool) -> Result<(), String> {
    if is_stdio(output) {
        let mut stdout = std::io::stdout();
        if stdout.is_terminal() {
            return Err(
                "Refusing to write binary icon data to a terminal. Redirect or pipe the output, for example: img2ico logo.png -o - > icon.ico"
                    .to_string(),
            );
        }
        // A reader that stops early (`... | head -c 10`) is an ordinary,
        // quiet end - not an error of ours.
        return match stdout.write_all(bytes).and_then(|()| stdout.flush()) {
            Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => {
                Err(format!("Could not write to standard output: {e}"))
            }
            _ => Ok(()),
        };
    }

    // Written atomically: the output is either complete or not there (or
    // still the old file), never cut off by a crash or a Ctrl+C.
    crate::util::write_atomic(output, bytes).map_err(|e| {
        format!(
            "Could not write {} file '{}': {e}",
            if use_icns { "ICNS" } else { "ICO" },
            output.display()
        )
    })
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
    let (result, record) = run_job(&job, resolved, replacement, worker_count(resolved));

    let ran = match result {
        Ok(Outcome::Converted { sizes, .. }) => {
            say!(
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

    // The files are converted on up to `workers` threads, but reported
    // strictly in input order, as they come up in line. With several
    // workers each file's sizes are done one after the other - the cores are
    // busy with other files already; with one file or one worker the sizes
    // get all the threads instead.
    let threads = worker_count(resolved);
    let workers = threads.min(total);
    let size_threads = if workers > 1 { 1 } else { threads };
    resolved.note(format_args!(
        "batch: {workers} worker(s), {size_threads} thread(s) per file"
    ));

    let memory = Budget::new(DECODED_PIXELS_IN_FLIGHT);
    for_each_ordered(
        &jobs,
        workers,
        |_, job| {
            // Lets warnings raised deep inside the conversion name this file.
            let _context = enter_file_context(&job.input);
            // Wait for a share of the memory budget before decoding: a few
            // huge images take turns, many small ones run side by side.
            let _share = (workers > 1 && !is_stdio(&job.input))
                .then(|| memory.take(peek_pixels(&job.input)));
            run_job(job, resolved, replacement, size_threads)
        },
        |index, (result, mut record)| {
            let job = &jobs[index];
            let count = progress_prefix(index + 1, total);

            match result {
                Ok(Outcome::Converted { bytes, sizes }) => {
                    if !resolved.silent {
                        say!(
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
                        // The first failure is the one that stops the run;
                        // files that were already under way are finished
                        // and reported, but no new one is started.
                        stopped_by.get_or_insert(message);
                    } else {
                        eprintln!("{count}Error: {message}");
                    }
                }
            }
            records.push(record);
            stopped_by.is_none()
        },
    );

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
        let verdict = if !is_stdio(&job.input) && !job.input.is_file() {
            fail += 1;
            "would fail: input file not found".to_string()
        } else if !is_stdio(&job.output) && job.output.exists() {
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
    if is_stdio(output) {
        format!("Done: icon written to standard output, with sizes {sizes:?}.")
    } else if use_icns {
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
    say!(
        "Batch finished: {} converted, {} skipped, {} failed{warnings} ({} file(s) in total).",
        summary.converted,
        summary.skipped,
        summary.failed,
        summary.total
    );
    if summary.converted > 0 {
        say!(
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
    threads: usize,
) -> (Result<Outcome, String>, FileRecord) {
    let started = Instant::now();
    let warnings_before = warnings_so_far();
    let result = convert_one(job, resolved, replacement, threads);

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
                say!("Report written to '{}'.", path.display());
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
    threads: usize,
) -> Result<Outcome, String> {
    let started = Instant::now();
    let input_path = job.input.as_path();
    let output_path = job.output.as_path();
    let use_icns = wants_icns(resolved.output_format);
    // Standard output is never "already there" - and `-` is not a file name.
    let to_stdout = is_stdio(output_path);

    // Never let the icon replace the image it is made from - with an .icns
    // source and --output-format icns (the default on macOS) the default
    // output name is the input's own, and --force would then destroy the
    // source.
    if !to_stdout && !is_stdio(input_path) && same_file(input_path, output_path) {
        return Err(format!(
            "The output '{}' is the input file itself - converting would overwrite the source image. Choose another file with -o, or another --output-format.",
            output_path.display()
        ));
    }

    // --skip-existing: with the output already there there is nothing to do
    // - and nothing worth loading, so this comes before anything else. (A
    // --find preview writes no file, so it is never skipped.)
    let find_preview = resolved.find.is_some() && !resolved.auto_apply;
    if resolved.skip_existing && !find_preview && !to_stdout && output_path.exists() {
        return Ok(Outcome::Skipped);
    }

    // Load the input image EARLY - before the output path/--sizes are even
    // looked at - specifically so that a plain "--find" preview (see
    // below) can print its report and exit without needing any of that:
    // it doesn't write a file, so it shouldn't need to know or care where
    // one WOULD have gone.
    let load_started = Instant::now();
    let source = match load_source_image(input_path, resolved)? {
        Loaded::Raster(image) => image,
        Loaded::Vector(drawing) => {
            return convert_vector(
                job,
                resolved,
                replacement,
                &drawing,
                (started, load_started),
                threads,
            );
        }
    };
    let (width, height) = source.dimensions();
    resolved.note(format_args!(
        "source: {} ({width}x{height} pixels), loaded in {:.1?}",
        input_path.display(),
        load_started.elapsed()
    ));

    // What the conversion needs to know, from the settings. The sizes and the
    // seed points are filled in below, once it is clear that there is
    // something to convert (a --find preview needs neither).
    let notes = CliNotes(resolved);
    let mut options = options_of(resolved, replacement, use_icns, threads)?;

    // The background color, if the options name one: given as a hex code, or
    // detected from the image border with "auto". With --find, also the
    // regions of that color the border-based flood fill can't reach on its
    // own (the same situation --seed manually solves, just discovered
    // automatically).
    let analysis = analyze_raster(&source, &options, &notes)?;

    // Without --auto-apply, printing the suggestions is the ENTIRE effect of
    // --find: the run stops there, without converting anything (and without
    // --out-toml writing anything either - nothing was actually decided
    // about the FINAL settings yet in that case).
    if let Some(target) = analysis
        .target
        .as_ref()
        .filter(|target| target.flag == "--find")
    {
        print_found_regions(&target.label, &analysis.regions, !resolved.auto_apply);

        if !resolved.auto_apply {
            if !analysis.regions.is_empty() {
                println!(
                    "Re-run with these as --seed values, or add --auto-apply to use them automatically."
                );
            }
            return Ok(Outcome::Previewed);
        }
    }

    let sizes = prepare_output(job, resolved, use_icns, to_stdout)?;
    options.sizes = sizes_of(resolved, sizes);
    if let Some(background) = options.background.as_mut() {
        background.seeds = parse_seeds(resolved.seeds)?;
    }

    let converted = convert_raster(source, &options, &analysis, &notes)?;
    write_output(output_path, &converted.bytes, use_icns)?;
    let bytes = converted.bytes.len() as u64;

    resolved.note(format_args!(
        "{}finished in {:.1?}",
        file_prefix(),
        started.elapsed()
    ));
    Ok(Outcome::Converted {
        bytes,
        sizes: converted.sizes,
    })
}

/// Where the commentary of a conversion goes: the run's own lines to its
/// output (see `say!`), the verbose ones to standard error.
struct CliNotes<'a, 'b>(&'a ResolvedSettings<'b>);

impl Notes for CliNotes<'_, '_> {
    fn say(&self, line: &str) {
        say!("{line}");
    }

    fn verbose(&self, line: &str) {
        self.0.note(line);
    }

    fn verbose_enabled(&self) -> bool {
        self.0.verbose
    }
}

/// The conversion's options from the settings. The sizes start empty and the
/// seed points unparsed (see `convert_one`); `replacement` is the color from
/// --replace-color, already parsed.
fn options_of(
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    use_icns: bool,
    threads: usize,
) -> Result<Options, String> {
    let background = match (resolved.chroma_key, resolved.find) {
        (Some(spec), _) => Some((BackgroundMode::ChromaKey, spec)),
        (None, Some(spec)) => Some((
            BackgroundMode::Find {
                min_size: resolved.find_min_size,
            },
            spec,
        )),
        (None, None) => None,
    };
    Ok(Options {
        format: if use_icns { Format::Icns } else { Format::Ico },
        sizes: Sizes::Fixed(Vec::new()),
        layout: layout_of(resolved),
        background: background.map(|(mode, spec)| Background {
            mode,
            spec: spec.trim().to_string(),
            tolerance: resolved.tolerance,
            feather: resolved.feather,
            seeds: Vec::new(),
            replacement,
        }),
        grayscale: resolved.grayscale,
        crop: resolved.crop.map(parse_crop).transpose()?,
        trim: resolved.trim,
        threads,
        silent: resolved.silent,
    })
}

/// The sizes from `prepare_output`, as the conversion wants them: fixed, or
/// `--sizes auto` - settled once the source's size is known.
fn sizes_of(resolved: &ResolvedSettings, sizes: Vec<u32>) -> Sizes {
    if wants_auto_sizes(resolved) {
        Sizes::Auto(sizes)
    } else {
        Sizes::Fixed(sizes)
    }
}

/// How the picture is laid onto the icon canvas, from the settings.
fn layout_of(resolved: &ResolvedSettings) -> Layout {
    Layout {
        fit: resolved.fit,
        padding: resolved.padding,
        corner_radius: resolved.corner_radius,
    }
}

/// Whether `--sizes auto` is in effect (a preset, which names its sizes
/// outright, takes precedence over --sizes as always).
fn wants_auto_sizes(resolved: &ResolvedSettings) -> bool {
    resolved.preset.is_none()
        && resolved
            .sizes
            .is_some_and(|sizes| sizes.trim().eq_ignore_ascii_case("auto"))
}

/// The output side of a conversion, shared by raster and vector sources:
/// refuses to overwrite an existing output (before any real work is done),
/// creates the output folder if the job asks for it, and works out the sizes
/// of an .ico. Returns them (unused, and harmless, for .icns).
fn prepare_output(
    job: &Job,
    resolved: &ResolvedSettings,
    use_icns: bool,
    to_stdout: bool,
) -> Result<Vec<u32>, String> {
    let output_path = job.output.as_path();
    resolved.note(format_args!(
        "output: {} ({})",
        output_path.display(),
        if use_icns { "icns" } else { "ico" }
    ));

    // Fail fast, before doing any actual work (resizing the image), if
    // the resolved output already exists and --force wasn't given. This
    // one check covers both the normal ICO path and --icns, since they
    // share this same output_path.
    if !to_stdout {
        check_overwrite(output_path, resolved.force)?;
    }
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
    Ok(sizes)
}

/// Converts an SVG: instead of loading one picture and scaling it down, the
/// drawing is rendered anew at every size of the icon. A background color can
/// still be removed (from every rendered size); the options that name pixel
/// positions are refused rather than silently ignored.
fn convert_vector(
    job: &Job,
    resolved: &ResolvedSettings,
    replacement: Option<[u8; 3]>,
    drawing: &VectorImage,
    (started, load_started): (Instant, Instant),
    threads: usize,
) -> Result<Outcome, String> {
    let output_path = job.output.as_path();
    let use_icns = wants_icns(resolved.output_format);
    let to_stdout = is_stdio(output_path);
    let (width, height) = drawing.size();
    resolved.note(format_args!(
        "source: {} (SVG, {width}x{height} units - rendered at every size), loaded in {:.1?}",
        job.input.display(),
        load_started.elapsed()
    ));

    // --seed and --find work with pixel positions, which mean something
    // different at every size - they are for raster images.
    let unsupported = if resolved.find.is_some() {
        Some("--find")
    } else if !resolved.seeds.is_empty() {
        Some("--seed")
    } else if resolved.crop.is_some() {
        Some("--crop")
    } else {
        None
    };
    if let Some(option) = unsupported {
        return Err(vector_refusal(option));
    }

    let sizes = prepare_output(job, resolved, use_icns, to_stdout)?;
    // --sizes auto: a drawing has no resolution to stay below, so it gets
    // all the default sizes (what prepare_output returned for "auto").
    let mut options = options_of(resolved, replacement, use_icns, threads)?;
    options.sizes = Sizes::Fixed(sizes);
    let converted = convert_drawing(drawing, &options, &CliNotes(resolved))?;
    write_output(output_path, &converted.bytes, use_icns)?;

    resolved.note(format_args!(
        "{}finished in {:.1?}",
        file_prefix(),
        started.elapsed()
    ));
    Ok(Outcome::Converted {
        bytes: converted.bytes.len() as u64,
        sizes: converted.sizes,
    })
}

/// Rejects option combinations that contradict each other or make no
/// sense on their own.
fn check_options(resolved: &ResolvedSettings) -> Result<(), String> {
    check_color_options(resolved)?;
    // The syntax of --crop is checked before any file is touched; that the
    // part lies inside the image can only be told per image.
    if let Some(text) = resolved.crop {
        parse_crop(text)?;
    }
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
fn load_source_image(input_path: &Path, resolved: &ResolvedSettings) -> Result<Loaded, String> {
    // Standard input arrives as bytes; a file is read by the decoders
    // themselves. Everything below is the same either way.
    let stdin_bytes = if is_stdio(input_path) {
        Some(read_stdin()?)
    } else {
        None
    };

    let is_gif = match &stdin_bytes {
        Some(bytes) => is_gif_bytes(bytes),
        None => is_gif(input_path)?,
    };
    if is_gif {
        return match &stdin_bytes {
            Some(bytes) => img2ico_core::source::extract_gif_frame_from_bytes(
                bytes,
                "standard input",
                resolved.gif_frame,
                resolved.max_pixels,
            ),
            None => extract_gif_frame_limited(input_path, resolved.gif_frame, resolved.max_pixels),
        }
        .map(Loaded::Raster);
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
    //
    // The format is recognized from the content, for a file as for standard
    // input - image::open() would go by the file extension, and fail on a
    // file without one or with a wrong one. (See source.rs.)
    let decoded = match &stdin_bytes {
        Some(bytes) => decode_source_bytes(bytes, resolved.max_pixels),
        None => open_source(input_path, resolved.max_pixels),
    };
    let source = decoded.map_err(|e| {
        let hint = if stdin_bytes.is_some() {
            STDIN_DECODE_HINT
        } else {
            ""
        };
        format!("Could not read input file: {e}{hint}")
    })?;
    Ok(match source {
        Artwork::Raster(image) => Loaded::Raster(image.into_rgba8()),
        Artwork::Vector(drawing) => Loaded::Vector(drawing),
    })
}

/// What was loaded as the source of a conversion.
enum Loaded {
    Raster(RgbaImage),
    Vector(VectorImage),
}

/// Prints --find's report: how many regions were found and, for each, a
/// ready-to-use --seed value.
///
/// The report is the output of a plain preview, so it goes to standard output
/// then. With --auto-apply the run converts as well, and the report is only
/// commentary on it (`say!`): --quiet hides it, and with `-o -` it keeps
/// off the standard output the icon goes to.
fn print_found_regions(find_hex: &str, regions: &[FoundRegion], is_preview: bool) {
    let say = |line: String| {
        if is_preview {
            println!("{line}");
        } else {
            say!("{line}");
        }
    };
    let prefix = file_prefix();
    if regions.is_empty() {
        say(format!(
            "{prefix}No additional regions matching {find_hex} found - the border-based flood fill should already reach everything."
        ));
        return;
    }

    say(format!(
        "{prefix}Found {} additional region(s) matching {find_hex} that the border-based flood fill can't reach on its own:",
        regions.len()
    ));
    for region in regions {
        let (x, y) = region.seed;
        say(format!(
            "  ~{} pixel(s) near ({x}, {y}) -> --seed {x},{y}",
            region.pixel_count
        ));
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
        // "auto" starts from the default sizes; what the source can supply
        // is decided once its size is known (see wants_auto_sizes).
        None if wants_auto_sizes(resolved) => parse_size_list(DEFAULT_SIZES),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use clap::Parser;
    use img2ico_core::convert::{BackgroundTarget, remove_background};

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

    /// The background color the settings name, found as a conversion finds it.
    fn resolve_background(
        img: &RgbaImage,
        resolved: &ResolvedSettings,
    ) -> Result<Option<BackgroundTarget>, String> {
        let options = options_of(resolved, None, false, 1)?;
        Ok(analyze_raster(img, &options, &CliNotes(resolved))?.target)
    }

    /// What a conversion does for the background: name the color, then remove it.
    /// `discovered` are extra seed points, as --find would have found them.
    fn remove(
        img: &mut RgbaImage,
        resolved: &ResolvedSettings,
        replacement: Option<[u8; 3]>,
        discovered: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let notes = CliNotes(resolved);
        let mut options = options_of(resolved, replacement, false, 1)?;
        if let Some(background) = options.background.as_mut() {
            background.seeds = parse_seeds(resolved.seeds)?;
        }
        let mut analysis = analyze_raster(img, &options, &notes)?;
        analysis.regions = discovered
            .into_iter()
            .map(|seed| FoundRegion {
                pixel_count: 1,
                seed,
            })
            .collect();
        remove_background(img, &options, &analysis, &notes)
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
