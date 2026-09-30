// ============================================================================
// img2ico - converts any image format into a Windows ICO file (or a macOS
// ICNS file) with support for transparency (alpha channel).
//
// This file only contains the top-level orchestration (main() and run()).
// Everything else lives in its own module, grouped by feature:
//   - cli.rs         the Args struct and the two --preset/--output-format
//                     value enums (everything clap needs)
//   - config.rs       the optional --config/--out-toml TOML settings file
//   - chroma_key.rs   --chroma-key/--find hex parsing and the flood-fill
//                     and region-discovery algorithms
//   - resize.rs       alpha-aware resizing and square-icon construction
//   - icns.rs         the macOS .icns container format
//   - ico_ops.rs      --merge, --inspect, --extract, --select (everything
//                     that reads/writes EXISTING .ico files)
//   - util.rs         small general-purpose helpers (--seed parsing,
//                     overwrite protection, --delete-source)
// ============================================================================

mod chroma_key;
mod cli;
mod config;
mod gif;
mod icns;
mod ico_ops;
mod resize;
mod util;

use chroma_key::{apply_chroma_key, find_isolated_regions, parse_hex_color, DEFAULT_FIND_MIN_SIZE};
use clap::Parser;
use cli::{Args, OutputFormat};
use config::{load_config, write_config, Settings};
use gif::{extract_gif_frame, is_gif};
use icns::{write_icns, ICNS_SIZES};
use ico_ops::{extract_icons, inspect_icons, merge_icons, parse_indices, select_icons};
use resize::{
    apply_grayscale, has_transparency, make_square_icon, warn_about_thin_content,
    warn_about_upscaling,
};
use util::{check_overwrite, delete_source_files, parse_seed};

fn main() {
    install_broken_pipe_panic_hook();

    // The actual work lives in run(). Reason for this split: run() returns
    // a Result<(), String>, which lets us use the "?" operator everywhere
    // inside it (see explanation below) - this saves us from repeating the
    // same "print error + exit" block at every single call site. main()
    // itself then only has to handle a potential error ONCE, centrally in
    // one place, printing it and exiting the program with exit code 1.
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

/// Fixes a real bug: by default, Rust's `println!`/`eprintln!` PANIC if
/// the write itself fails - and one very ordinary, non-error way for that
/// to happen is piping this program's output into something that closes
/// the pipe early, e.g. `img2ico --inspect big.ico | head -3`. `head`
/// reads the first 3 lines and exits, closing its end of the pipe; the
/// next `println!` we attempt after that fails with a "Broken pipe"
/// error, and the standard library turns THAT into a panic - a scary
/// stack trace and exit code 101, for something that isn't a bug in this
/// program at all and that virtually every other command-line tool
/// handles by just quietly stopping.
///
/// The fix: install a custom panic hook that runs BEFORE the normal one.
/// If the panic is specifically this "failed printing to stdout/stderr:
/// Broken pipe" case, we exit cleanly (code 0, no scary output) instead -
/// exactly what a well-behaved CLI tool does when its output pipe closes
/// early. Any OTHER panic (an actual bug) falls through to the normal
/// panic hook completely unchanged, so a genuine crash is never hidden.
///
/// Caveat worth knowing: this recognizes the broken-pipe case by checking
/// whether the panic message contains the text "Broken pipe", since
/// std's panicking print functions don't hand us the underlying
/// `std::io::Error` in a more structured way. That text comes from the
/// operating system and could in principle read differently on a
/// non-English-locale system - if that ever happens, the worst case is
/// simply that this fix doesn't kick in and behavior falls back to
/// exactly what it was before (a panic), never anything worse.
fn install_broken_pipe_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic_info.payload().downcast_ref::<&str>().copied());

        if let Some(message) = message {
            if message.contains("Broken pipe") {
                std::process::exit(0);
            }
        }

        default_hook(panic_info);
    }));
}

/// Everything --config/a discovered "img2ico.toml" resolved into, plus
/// the settings actually used for THIS run - bundled together so it can
/// be passed around easily and, at the end, optionally written back out
/// via --out-toml. Every field here mirrors one from `Settings`
/// (config.rs) - see that struct's doc comment for which settings are
/// config-file-eligible in the first place (and why some, like the mode
/// or the input file, deliberately aren't).
///
/// Resolution order for every field is the same: an explicit command-line
/// flag wins outright; otherwise the config file's value is used (if any);
/// otherwise img2ico's own built-in default applies. For plain on/off
/// flags (no explicit "off" is possible on the command line, only
/// "given" or "not given"), this is implemented as a simple OR: the
/// setting ends up on if EITHER the command line or the config file turns
/// it on.
struct ResolvedSettings {
    preset: Option<cli::SizePreset>,
    chroma_key: Option<String>,
    tolerance: u8,
    seeds: Vec<String>,
    find: Option<String>,
    find_min_size: usize,
    auto_apply: bool,
    replace_color: Option<String>,
    grayscale: bool,
    padding: u8,
    gif_frame: usize,
    silent: bool,
    output_format: Option<OutputFormat>,
    delete_source: bool,
    force: bool,
    combine: bool,
    index: Option<String>,
    sizes: Option<String>,
}

impl ResolvedSettings {
    /// Merges the command-line `args` with a loaded (or default, if no
    /// config file applied) `settings`, following the "CLI wins, then
    /// config file, then built-in default" priority described on the
    /// struct itself.
    fn resolve(args: &Args, settings: &Settings) -> ResolvedSettings {
        ResolvedSettings {
            preset: args.preset.or(settings.preset),
            chroma_key: args.chroma_key.clone().or_else(|| settings.chroma_key.clone()),
            tolerance: args.tolerance.or(settings.tolerance).unwrap_or(20),
            seeds: if !args.seeds.is_empty() {
                args.seeds.clone()
            } else {
                settings.seeds.clone()
            },
            find: args.find.clone().or_else(|| settings.find.clone()),
            find_min_size: args
                .find_min_size
                .or(settings.find_min_size)
                .unwrap_or(DEFAULT_FIND_MIN_SIZE),
            auto_apply: args.auto_apply || settings.auto_apply,
            replace_color: args
                .replace_color
                .clone()
                .or_else(|| settings.replace_color.clone()),
            grayscale: args.grayscale || settings.grayscale,
            padding: args.padding.or(settings.padding).unwrap_or(0),
            gif_frame: args.gif_frame.or(settings.gif_frame).unwrap_or(1),
            silent: args.silent || settings.silent,
            output_format: args.output_format.or(settings.output_format),
            delete_source: args.delete_source || settings.delete_source,
            force: args.force || settings.force,
            combine: args.combine || settings.combine,
            index: args.index.clone().or_else(|| settings.index.clone()),
            sizes: args.sizes.clone().or_else(|| settings.sizes.clone()),
        }
    }

    /// Turns the resolved settings back into a `Settings` value, for
    /// --out-toml to write out as a snapshot of what was actually used
    /// for this run.
    fn to_settings(&self) -> Settings {
        Settings {
            sizes: self.sizes.clone(),
            preset: self.preset,
            chroma_key: self.chroma_key.clone(),
            tolerance: Some(self.tolerance),
            seeds: self.seeds.clone(),
            find: self.find.clone(),
            find_min_size: Some(self.find_min_size),
            auto_apply: self.auto_apply,
            replace_color: self.replace_color.clone(),
            grayscale: self.grayscale,
            padding: Some(self.padding),
            gif_frame: Some(self.gif_frame),
            output_format: self.output_format,
            delete_source: self.delete_source,
            force: self.force,
            combine: self.combine,
            index: self.index.clone(),
            silent: self.silent,
        }
    }
}

/// Writes `resolved` out to `path` as TOML, if `--out-toml` was given -
/// shared by every mode that supports it (merge/extract/select/normal
/// conversion). A no-op if `out_toml` is `None`. Called right before each
/// mode's own final `Ok(())`, so it only runs after everything else about
/// the run already succeeded.
fn maybe_write_out_toml(out_toml: &Option<std::path::PathBuf>, resolved: &ResolvedSettings) -> Result<(), String> {
    if let Some(path) = out_toml {
        write_config(&resolved.to_settings(), path)?;
        if !resolved.silent {
            println!("Settings written to '{}'.", path.display());
        }
    }
    Ok(())
}

/// Contains the complete program logic.
///
/// Rust note on the "?" operator: when a "?" follows an expression that
/// returns a Result, the following happens automatically:
///   - Ok(value) -> gets unwrapped to "value", the program continues normally
///   - Err(e)    -> the function is exited IMMEDIATELY, returning Err(e)
///
/// This is shorthand for exactly the pattern we previously rebuilt by hand
/// everywhere with unwrap_or_else(|e| { eprintln!(...); exit(1); }) -
/// except the error is now "passed through" up to main(), instead of
/// ending the whole program immediately at every single call site. This
/// requires the function itself to return a Result (hence
/// "-> Result<(), String>" right below).
fn run() -> Result<(), String> {
    // Args::parse() reads argv, validates it against the struct in cli.rs,
    // and automatically exits the program with an error message if, say,
    // the input file is missing.
    let args = Args::parse();

    // --merge, --inspect, --extract and --select are mutually exclusive -
    // each one replaces the normal image-conversion pipeline with
    // something else entirely. --output-format is different: it's still
    // part of the normal conversion pipeline (just picking a different
    // output container at the end), so they only conflict with the other
    // four modes, not with "no flag at all". Note this check deliberately
    // looks at the RAW args.output_format (whether the user typed
    // --output-format at all), not any config-file/platform-based default
    // - a Mac user (or someone with --output-format in their config file)
    // running --merge without ever explicitly typing --output-format
    // shouldn't trip this just because icns happens to be the default.
    let mode_count = [args.merge, args.inspect, args.extract, args.select]
        .iter()
        .filter(|&&on| on)
        .count();
    if mode_count > 1 || (mode_count == 1 && args.output_format.is_some()) {
        return Err(
            "--merge, --inspect, --extract, --select and --output-format are mutually exclusive - please use only one at a time."
                .to_string(),
        );
    }

    if args.inspect {
        // --inspect never writes anything to disk (that's the whole
        // point of it), so it deliberately doesn't participate in
        // --config/--out-toml at all - loading tuning settings that
        // inspect wouldn't use anyway, or letting --out-toml add a
        // surprise file-write side effect, would both work against that.
        return inspect_icons(&args.input);
    }

    // Resolve the config file (--config, or an auto-discovered
    // "img2ico.toml" in the current directory) ONCE here, before any of
    // the remaining modes - all of them can use --force/--delete-source
    // (and --select additionally --combine/--index), so all of them need
    // the resolved settings, not just the normal conversion path further
    // down.
    let (settings, config_path) = match load_config(args.config.as_deref(), args.silent)? {
        Some((settings, path)) => (settings, Some(path)),
        None => (Settings::default(), None),
    };
    if let Some(path) = &config_path {
        if !args.silent {
            println!("Using settings from '{}'.", path.display());
        }
    }
    let resolved = ResolvedSettings::resolve(&args, &settings);

    if args.extract {
        let [input_path] = args.input.as_slice() else {
            return Err(format!(
                "--extract expects exactly one input .ico file, got {}.",
                args.input.len()
            ));
        };
        extract_icons(input_path, args.output.as_deref(), resolved.force)?;
        if resolved.delete_source {
            // The output here is a directory, not a file, so it can never
            // collide with the (file) source path - no output_path needed
            // for the same-file safety check.
            delete_source_files(std::slice::from_ref(input_path), None, resolved.silent);
        }
        maybe_write_out_toml(&args.out_toml, &resolved)?;
        return Ok(());
    }

    if args.select {
        let [input_path] = args.input.as_slice() else {
            return Err(format!(
                "--select expects exactly one input .ico file, got {}.",
                args.input.len()
            ));
        };
        let indices = parse_indices(&resolved.index)?;
        let written_file = select_icons(
            input_path,
            &indices,
            resolved.combine,
            args.output.as_deref(),
            resolved.force,
        )?;
        if resolved.delete_source {
            // written_file is Some(path) when a single combined file was
            // written (protect against deleting it if it happens to equal
            // the source), or None when a whole directory of separate
            // files was written instead (can't collide with a file path).
            delete_source_files(std::slice::from_ref(input_path), written_file.as_deref(), resolved.silent);
        }
        maybe_write_out_toml(&args.out_toml, &resolved)?;
        return Ok(());
    }

    // Merge mode branches off immediately: it has its own, much simpler
    // pipeline (no resizing, no chroma-key) and doesn't touch any of the
    // image-conversion logic below at all.
    if args.merge {
        // Merging several files into one has no natural "obvious" output
        // name the way single-image conversion does (input name + .ico),
        // so we require an explicit -o here instead of guessing.
        let output_path = args.output.ok_or_else(|| {
            "Merge mode requires an explicit output path (-o/--output).".to_string()
        })?;
        merge_icons(&args.input, &output_path, resolved.force, resolved.silent)?;
        if resolved.delete_source {
            delete_source_files(&args.input, Some(&output_path), resolved.silent);
        }
        maybe_write_out_toml(&args.out_toml, &resolved)?;
        return Ok(());
    }

    // Normal (non-merge) mode expects exactly one input image.
    let [input_path] = args.input.as_slice() else {
        return Err(format!(
            "Expected exactly one input image, got {} (use --merge to combine multiple existing .ico files instead).",
            args.input.len()
        ));
    };

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
    let replacement: Option<[u8; 3]> = match &resolved.replace_color {
        Some(hex) => {
            Some(parse_hex_color(hex).map_err(|e| format!("Invalid --replace-color value: {e}"))?)
        }
        None => None,
    };

    // Load the input image EARLY - before output_path/--sizes are even
    // looked at - specifically so that a plain "--find" preview (see
    // below) can print its report and exit without needing any of that:
    // it doesn't write a file, so it shouldn't need to know or care where
    // one WOULD have gone.
    //
    // image::open() alone - used for every other format - only ever
    // returns a GIF's FIRST frame, with no way to pick a different one.
    // So: for a GIF specifically, --gif-frame picks which frame becomes
    // the source image (extract_gif_frame() already gives back an
    // RgbaImage directly); for everything else, --gif-frame has no
    // meaning and image::open() (which detects the format from the file
    // header, not just its extension) is used exactly as before.
    let mut rgba_source = if is_gif(input_path)? {
        extract_gif_frame(input_path, resolved.gif_frame)?
    } else {
        if resolved.gif_frame != 1 && !resolved.silent {
            eprintln!(
                "Warning: --gif-frame only applies to GIF input and is ignored for this file."
            );
        }
        // IMPORTANT (optimization): to_rgba8() converts/copies the entire
        // image. This intentionally happens only ONCE here, regardless of
        // whether --chroma-key/--find is set or not - and NOT (as in an
        // earlier version) again for every single icon size. For a large
        // source image and the 6 default sizes, that would otherwise have
        // been 6 full copies of the source image, even though the source
        // image doesn't change between sizes.
        image::open(input_path)
            .map_err(|e| format!("Could not read input file: {e}"))?
            .to_rgba8()
    };

    // --find: look for regions matching this color that the border-based
    // flood fill in apply_chroma_key can't reach on its own (the same
    // situation --seed manually solves, just discovered automatically).
    // Without --auto-apply, this is the ENTIRE effect of --find: print
    // the suggestions and stop, without converting anything (and without
    // --out-toml writing anything either - nothing was actually decided
    // about the FINAL settings yet in that case).
    let mut auto_found_seeds: Vec<(u32, u32)> = Vec::new();
    if let Some(find_hex) = &resolved.find {
        let target =
            parse_hex_color(find_hex).map_err(|e| format!("Invalid --find value: {e}"))?;
        let regions = find_isolated_regions(&rgba_source, target, resolved.tolerance, resolved.find_min_size, resolved.silent);

        if regions.is_empty() {
            println!(
                "No additional regions matching {find_hex} found - the border-based flood fill should already reach everything."
            );
        } else {
            println!(
                "Found {} additional region(s) matching {find_hex} that the border-based flood fill can't reach on its own:",
                regions.len()
            );
            for region in &regions {
                let (x, y) = region.seed;
                println!(
                    "  ~{} pixel(s) near ({x}, {y}) -> --seed {x},{y}",
                    region.pixel_count
                );
            }
        }

        if !resolved.auto_apply {
            if !regions.is_empty() {
                println!(
                    "Re-run with these as --seed values, or add --auto-apply to use them automatically."
                );
            }
            return Ok(());
        }

        auto_found_seeds = regions.into_iter().map(|r| r.seed).collect();
    }

    // Whether to produce .icns or .ico. If the user gave --output-format
    // explicitly (or a config file set it), that wins outright. Otherwise,
    // fall back to a platform-based default: .icns on macOS, .ico
    // everywhere else - this is what most people building on a given
    // platform actually want, without having to remember to pass
    // --output-format every time on a Mac.
    //
    // cfg!(target_os = "macos") is a compile-time check (baked into the
    // binary depending on what platform it was BUILT for), not a runtime
    // one - which is exactly what we want here: a binary built natively
    // on macOS should default to .icns, regardless of where it's later
    // copied to and run from.
    let use_icns = match resolved.output_format {
        Some(OutputFormat::Icns) => true,
        Some(OutputFormat::Ico) => false,
        None => cfg!(target_os = "macos"),
    };

    // Determine the target path: either given explicitly, or the input
    // name with a ".ico" (or ".icns", depending on use_icns) extension.
    // Not an error case, but a default value - hence still
    // unwrap_or_else() instead of "?". -o/--output stays command-line-only
    // (not config-file-eligible) - see cli.rs's doc comment on --config
    // for why.
    let output_path = args.output.unwrap_or_else(|| {
        let mut p = input_path.clone();
        p.set_extension(if use_icns { "icns" } else { "ico" });
        p
    });

    // Fail fast, before doing any actual work (resizing the image), if
    // the resolved output already exists and --force wasn't given. This
    // one check covers both the normal ICO path and --icns, since they
    // share this same output_path.
    check_overwrite(&output_path, resolved.force)?;

    // Turn the sizes string ("16,32,48") into a list of numbers. Each
    // individual size can fail (not a valid number) - so the map() closure
    // here returns a Result instead of a plain u32.
    // collect::<Result<Vec<u32>, String>>() then gathers everything into
    // ONE Result: as soon as any size is invalid, collect() immediately
    // returns that one Err (and stops), instead of still processing the
    // rest of the list.
    //
    // Only relevant for the normal ICO path - icns output uses its own fixed
    // set of sizes instead (see ICNS_SIZES) - but parsing it here anyway
    // is cheap and harmless even when it ends up unused.
    //
    // --preset short-circuits all of this: it replaces whatever --sizes
    // says with a predefined list outright, so --sizes is simply never
    // parsed/looked at in that case. If neither --sizes, a config file,
    // nor --preset set anything, img2ico's own built-in default
    // ("16,32,48,64,128,256") is used, matching the previous behavior
    // from before --sizes could be left unset.
    let sizes: Vec<u32> = if let Some(preset) = resolved.preset {
        preset.sizes().to_vec()
    } else {
        resolved
            .sizes
            .as_deref()
            .unwrap_or("16,32,48,64,128,256")
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u32>()
                    .map_err(|_| format!("Invalid size: '{s}' (must be a positive number)"))
            })
            .collect::<Result<Vec<u32>, String>>()?
    };

    if !use_icns && sizes.is_empty() {
        return Err("At least one size must be given.".to_string());
    }

    // Apply the actual chroma-key removal/replacement, if requested
    // either way: --chroma-key (with whatever manual --seed values were
    // given), or --find --auto-apply (manual --seed values PLUS the
    // automatically discovered regions from above, combined). --replace-color
    // was already parsed into `replacement` above and gets passed through
    // to both.
    if let Some(hex) = &resolved.chroma_key {
        let target = parse_hex_color(hex).map_err(|e| format!("Invalid --chroma-key value: {e}"))?;
        let seeds: Vec<(u32, u32)> = resolved
            .seeds
            .iter()
            .map(|s| parse_seed(s).map_err(|e| format!("Invalid --seed value: {e}")))
            .collect::<Result<Vec<(u32, u32)>, String>>()?;
        apply_chroma_key(&mut rgba_source, target, resolved.tolerance, &seeds, replacement, resolved.silent);
    } else if let Some(hex) = &resolved.find {
        // Only reachable when --auto-apply was given too - the plain
        // "--find" preview case already returned Ok(()) further up.
        let target = parse_hex_color(hex).map_err(|e| format!("Invalid --find value: {e}"))?;
        let mut seeds: Vec<(u32, u32)> = resolved
            .seeds
            .iter()
            .map(|s| parse_seed(s).map_err(|e| format!("Invalid --seed value: {e}")))
            .collect::<Result<Vec<(u32, u32)>, String>>()?;
        seeds.extend(auto_found_seeds);
        apply_chroma_key(&mut rgba_source, target, resolved.tolerance, &seeds, replacement, resolved.silent);
    }

    // --grayscale runs LAST, after any --chroma-key/--replace-color
    // processing above - so it uniformly affects the final colors,
    // including a --replace-color color if both were combined, rather
    // than leaving a confusing "everything except the replaced background
    // is grayscale" exception.
    if resolved.grayscale {
        apply_grayscale(&mut rgba_source);
    }

    // Computed ONCE here and passed down to every make_square_icon call
    // (for every single icon size) instead of re-checking per size - see
    // has_transparency()'s doc comment for why this matters.
    let has_alpha = has_transparency(&rgba_source);

    // A heads-up (not an error - upscaling still produces a valid icon,
    // just a softer one) if any requested size exceeds what the source
    // image actually has to offer. Resizing up can't invent detail that
    // isn't there; Lanczos3 (what make_square_icon uses) makes that
    // smooth rather than blocky, but it's still fundamentally a guess,
    // not real detail.
    let (source_w, source_h) = rgba_source.dimensions();
    if use_icns {
        let icns_sizes: Vec<u32> = ICNS_SIZES.iter().map(|&(size, _)| size).collect();
        warn_about_upscaling(source_w, source_h, &icns_sizes, resolved.silent);
        warn_about_thin_content(source_w, source_h, resolved.padding, &icns_sizes, resolved.silent);
    } else {
        warn_about_upscaling(source_w, source_h, &sizes, resolved.silent);
        warn_about_thin_content(source_w, source_h, resolved.padding, &sizes, resolved.silent);
    }

    // .icns branches off here (whether from an explicit --output-format icns or from
    // the platform-based default computed above): it has its own
    // container format (see write_icns) and doesn't use the ICO-specific
    // --sizes list at all.
    if use_icns {
        write_icns(&rgba_source, resolved.padding, has_alpha, &output_path)?;
        if resolved.delete_source {
            delete_source_files(std::slice::from_ref(input_path), Some(&output_path), resolved.silent);
        }
        maybe_write_out_toml(&args.out_toml, &resolved)?;
        return Ok(());
    }

    // An IconDir collects all the resolutions that will be written
    // together into ONE .ico file at the end.
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &size in &sizes {
        // ICO files officially only support edge lengths up to 256px.
        if size == 0 || size > 256 {
            if !resolved.silent {
                eprintln!("Skipping size {size} (valid range: 1-256).");
            }
            continue;
        }

        let square = make_square_icon(&rgba_source, size, resolved.padding, has_alpha);
        let (w, h) = square.dimensions();

        // into_raw() gives us the raw pixel bytes in RGBA order (red,
        // green, blue, alpha, red, green, blue, alpha, ...) - exactly the
        // format the ico crate expects as input.
        let raw_rgba = square.into_raw();

        let icon_image = ico::IconImage::from_rgba_data(w, h, raw_rgba);

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
        // PNG, on the other hand, always keeps full 32-bit color depth
        // including a clean alpha channel - exactly what was required for
        // "transparency as a feature". PNG-in-ICO has been supported by
        // Windows since Vista (2007), so it's safe for practically any
        // use case.
        let entry = ico::IconDirEntry::encode_as_png(&icon_image)
            .map_err(|e| format!("Could not encode size {size}: {e}"))?;
        icon_dir.add_entry(entry);
    }

    // Create the target file and write all the collected resolutions into it.
    let file =
        std::fs::File::create(&output_path).map_err(|e| format!("Could not create output file: {e}"))?;
    icon_dir
        .write(file)
        .map_err(|e| format!("Error writing ICO file: {e}"))?;

    println!(
        "Done: '{}' created with sizes {:?}.",
        output_path.display(),
        sizes
    );

    if resolved.delete_source {
        delete_source_files(std::slice::from_ref(input_path), Some(&output_path), resolved.silent);
    }

    maybe_write_out_toml(&args.out_toml, &resolved)?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use cli::SizePreset;

    fn args(extra: &[&str]) -> Args {
        let mut full = vec!["img2ico", "in.png"];
        full.extend_from_slice(extra);
        Args::parse_from(full)
    }

    // --- Built-in defaults -------------------------------------------------------

    #[test]
    fn without_cli_or_config_the_built_in_defaults_apply() {
        let resolved = ResolvedSettings::resolve(&args(&[]), &Settings::default());
        assert_eq!(resolved.tolerance, 20);
        assert_eq!(resolved.padding, 0);
        assert_eq!(resolved.gif_frame, 1);
        assert_eq!(resolved.find_min_size, DEFAULT_FIND_MIN_SIZE);
        assert_eq!(resolved.preset, None);
        assert_eq!(resolved.sizes, None);
        assert_eq!(resolved.chroma_key, None);
        assert!(resolved.seeds.is_empty());
        assert!(!resolved.grayscale && !resolved.force && !resolved.silent);
        assert!(!resolved.delete_source && !resolved.combine && !resolved.auto_apply);
    }

    // --- Config file values ------------------------------------------------------

    #[test]
    fn config_values_are_used_when_the_command_line_says_nothing() {
        let settings = Settings {
            tolerance: Some(45),
            padding: Some(10),
            gif_frame: Some(4),
            find_min_size: Some(2),
            sizes: Some("16,64".to_string()),
            preset: Some(SizePreset::Minimal),
            chroma_key: Some("#00FF00".to_string()),
            replace_color: Some("#000000".to_string()),
            find: Some("#FF00FF".to_string()),
            index: Some("1,2".to_string()),
            output_format: Some(OutputFormat::Icns),
            seeds: vec!["1,2".to_string()],
            ..Settings::default()
        };
        let resolved = ResolvedSettings::resolve(&args(&[]), &settings);
        assert_eq!(resolved.tolerance, 45);
        assert_eq!(resolved.padding, 10);
        assert_eq!(resolved.gif_frame, 4);
        assert_eq!(resolved.find_min_size, 2);
        assert_eq!(resolved.sizes.as_deref(), Some("16,64"));
        assert_eq!(resolved.preset, Some(SizePreset::Minimal));
        assert_eq!(resolved.chroma_key.as_deref(), Some("#00FF00"));
        assert_eq!(resolved.replace_color.as_deref(), Some("#000000"));
        assert_eq!(resolved.find.as_deref(), Some("#FF00FF"));
        assert_eq!(resolved.index.as_deref(), Some("1,2"));
        assert_eq!(resolved.output_format, Some(OutputFormat::Icns));
        assert_eq!(resolved.seeds, vec!["1,2"]);
    }

    // --- Priority: command line wins ---------------------------------------------

    #[test]
    fn command_line_values_win_over_the_config_file() {
        let settings = Settings {
            tolerance: Some(45),
            padding: Some(10),
            gif_frame: Some(4),
            sizes: Some("16,64".to_string()),
            preset: Some(SizePreset::Minimal),
            chroma_key: Some("#00FF00".to_string()),
            output_format: Some(OutputFormat::Icns),
            ..Settings::default()
        };
        let cli = args(&[
            "--tolerance", "5",
            "--padding", "0",
            "--gif-frame", "2",
            "--sizes", "32",
            "--preset", "favicon",
            "--chroma-key", "#FF0000",
            "--output-format", "ico",
        ]);
        let resolved = ResolvedSettings::resolve(&cli, &settings);
        assert_eq!(resolved.tolerance, 5);
        assert_eq!(resolved.padding, 0, "an explicit 0 must not fall back to the config value");
        assert_eq!(resolved.gif_frame, 2);
        assert_eq!(resolved.sizes.as_deref(), Some("32"));
        assert_eq!(resolved.preset, Some(SizePreset::Favicon));
        assert_eq!(resolved.chroma_key.as_deref(), Some("#FF0000"));
        assert_eq!(resolved.output_format, Some(OutputFormat::Ico));
    }

    #[test]
    fn command_line_seeds_replace_config_seeds_instead_of_adding_to_them() {
        let settings = Settings {
            seeds: vec!["1,1".to_string(), "2,2".to_string()],
            ..Settings::default()
        };
        let resolved = ResolvedSettings::resolve(&args(&["--seed", "9,9"]), &settings);
        assert_eq!(resolved.seeds, vec!["9,9"]);
    }

    // --- On/off flags are OR-ed --------------------------------------------------

    #[test]
    fn a_flag_is_on_if_either_the_command_line_or_the_config_turns_it_on() {
        let from_config = Settings {
            grayscale: true,
            force: true,
            silent: true,
            delete_source: true,
            combine: true,
            auto_apply: true,
            ..Settings::default()
        };
        let resolved = ResolvedSettings::resolve(&args(&[]), &from_config);
        assert!(resolved.grayscale && resolved.force && resolved.silent);
        assert!(resolved.delete_source && resolved.combine && resolved.auto_apply);

        let resolved = ResolvedSettings::resolve(
            &args(&["--grayscale", "--force", "--silent", "--delete-source", "--combine", "--auto-apply"]),
            &Settings::default(),
        );
        assert!(resolved.grayscale && resolved.force && resolved.silent);
        assert!(resolved.delete_source && resolved.combine && resolved.auto_apply);
    }

    // --- Snapshot for --out-toml -------------------------------------------------

    #[test]
    fn to_settings_records_the_resolved_values_including_defaults() {
        let resolved = ResolvedSettings::resolve(&args(&["--padding", "7", "--grayscale"]), &Settings::default());
        let snapshot = resolved.to_settings();
        assert_eq!(snapshot.padding, Some(7));
        assert_eq!(snapshot.tolerance, Some(20), "defaults are written out explicitly");
        assert_eq!(snapshot.gif_frame, Some(1));
        assert_eq!(snapshot.find_min_size, Some(DEFAULT_FIND_MIN_SIZE));
        assert!(snapshot.grayscale);
    }

    #[test]
    fn a_snapshot_resolves_to_the_same_settings_again() {
        let first = ResolvedSettings::resolve(
            &args(&["--tolerance", "33", "--seed", "4,5", "--preset", "windows", "--silent"]),
            &Settings::default(),
        );
        let second = ResolvedSettings::resolve(&args(&[]), &first.to_settings());
        assert_eq!(second.tolerance, first.tolerance);
        assert_eq!(second.seeds, first.seeds);
        assert_eq!(second.preset, first.preset);
        assert_eq!(second.silent, first.silent);
        assert_eq!(second.padding, first.padding);
    }

    // --- maybe_write_out_toml ----------------------------------------------------

    #[test]
    fn out_toml_is_only_written_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = ResolvedSettings::resolve(&args(&["--silent"]), &Settings::default());

        maybe_write_out_toml(&None, &resolved).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

        let path = dir.path().join("snapshot.toml");
        maybe_write_out_toml(&Some(path.clone()), &resolved).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("tolerance = 20"), "{text}");
    }
}
