// ============================================================================
// img2ico - converts any image format into a Windows ICO file (or a macOS
// ICNS file) with support for transparency (alpha channel).
//
// This file only contains the top-level orchestration: start-up, choosing
// the mode, and the three small "existing .ico file" modes. Everything
// else lives in its own module, grouped by feature:
//   - cli.rs         the Args struct and the two --preset/--output-format
//                     value enums (everything clap needs)
//   - config.rs       the optional --config/--out-toml TOML settings file
//   - settings.rs     merging command line + config file + defaults into
//                     the values a run actually uses
//   - convert.rs      the normal mode: one source image into an icon file
//   - chroma_key.rs   --chroma-key/--find hex parsing and the flood-fill
//                     and region-discovery algorithms
//   - resize.rs       alpha-aware resizing and square-icon construction
//   - gif.rs          animated GIF frame selection
//   - icns.rs         the macOS .icns container format
//   - ico_ops.rs      --merge, --inspect, --extract, --select (everything
//                     that reads/writes EXISTING .ico files)
//   - util.rs         small general-purpose helpers (--seed parsing,
//                     overwrite protection, --delete-source)
// ============================================================================

mod chroma_key;
mod cli;
mod config;
mod convert;
mod gif;
mod icns;
mod ico_ops;
mod resize;
mod settings;
mod util;

use clap::Parser;
use cli::Args;
use config::{Settings, load_layered, user_config_path};
use ico_ops::{extract_icons, inspect_icons, merge_icons, parse_indices, select_icons};
use settings::{ResolvedSettings, finish_run};
use std::path::{Path, PathBuf};

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

        if let Some(message) = message
            && message.contains("Broken pipe")
        {
            std::process::exit(0);
        }

        default_hook(panic_info);
    }));
}

/// Contains the top-level program logic: parse the command line, pick the
/// mode, and hand over to it.
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

    // --completions only prints a script; it needs none of the rest (and, by
    // the way it's declared in cli.rs, can't be combined with anything).
    if let Some(shell) = args.completions {
        return cli::write_completions(shell, &mut std::io::stdout());
    }

    check_single_mode(&args)?;
    reject_stdout_output(&args)?;

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
    // the resolved settings, not just the normal conversion path.
    let settings = load_settings(&args)?;
    let resolved = ResolvedSettings::resolve(&args, &settings);
    resolved.print_effective_settings()?;

    if args.extract {
        run_extract(&args, &resolved)
    } else if args.select {
        run_select(&args, &resolved)
    } else if args.merge {
        run_merge(&args, &resolved)
    } else {
        convert::run(&args, &resolved)
    }
}

/// --merge, --inspect, --extract and --select are mutually exclusive -
/// each one replaces the normal image-conversion pipeline with something
/// else entirely. --output-format is different: it's still part of the
/// normal conversion pipeline (just picking a different output container
/// at the end), so they only conflict with the other four modes, not with
/// "no flag at all". Note this check deliberately looks at the RAW
/// args.output_format (whether the user typed --output-format at all), not
/// any config-file/platform-based default - a Mac user (or someone with
/// --output-format in their config file) running --merge without ever
/// explicitly typing --output-format shouldn't trip this just because icns
/// happens to be the default.
fn check_single_mode(args: &Args) -> Result<(), String> {
    let mode_count = [args.merge, args.inspect, args.extract, args.select]
        .into_iter()
        .filter(|&on| on)
        .count();
    if mode_count > 1 || (mode_count == 1 && args.output_format.is_some()) {
        return Err(
            "--merge, --inspect, --extract, --select and --output-format are mutually exclusive - please use only one at a time."
                .to_string(),
        );
    }
    Ok(())
}

/// `-o -` is the usual Unix spelling for "write to standard output", which
/// img2ico doesn't support (yet). Without this check the run would quietly
/// create a file literally named "-" and report success - exactly what
/// nobody who types `-o -` wants. Saying so up front is safer; a file that
/// really is called "-" can still be written as `-o ./-`.
fn reject_stdout_output(args: &Args) -> Result<(), String> {
    if args.output.as_deref() == Some(Path::new("-")) {
        return Err(
            "'-o -' (writing to standard output) is not supported. Give a file name instead - or use './-' for a file that is really called '-'."
                .to_string(),
        );
    }
    Ok(())
}

/// Loads the settings files for this run (an explicit --config or an
/// auto-discovered "img2ico.toml", layered over the per-user file), or plain
/// defaults if there are none - or if --no-config says to ignore them.
/// Announces every file it used - img2ico never silently changes its own
/// behavior because of a file sitting in the current directory or in the
/// user's profile without saying so out loud.
fn load_settings(args: &Args) -> Result<Settings, String> {
    if args.no_config {
        return Ok(Settings::default());
    }

    let loaded = load_layered(
        args.config.as_deref(),
        user_config_path().as_deref(),
        args.silent,
    )?;
    if !args.silent {
        for path in &loaded.sources {
            println!("Using settings from '{}'.", path.display());
        }
    }
    Ok(loaded.settings)
}

/// The single input path of a mode that works on exactly one existing .ico
/// file (--extract, --select), or an error naming the `flag` and how many
/// inputs were actually given.
fn single_ico_input<'a>(args: &'a Args, flag: &str) -> Result<&'a PathBuf, String> {
    let [input_path] = args.input.as_slice() else {
        return Err(format!(
            "{flag} expects exactly one input .ico file, got {}.",
            args.input.len()
        ));
    };
    Ok(input_path)
}

/// --extract: every size of one .ico saved as a separate PNG.
fn run_extract(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    let input_path = single_ico_input(args, "--extract")?;
    extract_icons(input_path, args.output.as_deref(), resolved.force)?;
    // The output here is a directory, not a file, so it can never collide
    // with the (file) source path - no output path needed for the
    // same-file safety check.
    finish_run(args, resolved, std::slice::from_ref(input_path), None)
}

/// --select: specific icons of one .ico, by index, into new .ico file(s).
fn run_select(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    let input_path = single_ico_input(args, "--select")?;
    let indices = parse_indices(resolved.index)?;
    let written_file = select_icons(
        input_path,
        &indices,
        resolved.combine,
        args.output.as_deref(),
        resolved.force,
    )?;
    // written_file is Some(path) when a single combined file was written
    // (protect against deleting it if it happens to equal the source), or
    // None when a whole directory of separate files was written instead
    // (can't collide with a file path).
    finish_run(
        args,
        resolved,
        std::slice::from_ref(input_path),
        written_file.as_deref(),
    )
}

/// --merge: the icons of several .ico files combined into one. Has its
/// own, much simpler pipeline (no resizing, no chroma-key).
fn run_merge(args: &Args, resolved: &ResolvedSettings) -> Result<(), String> {
    // Merging several files into one has no natural "obvious" output name
    // the way single-image conversion does (input name + .ico), so we
    // require an explicit -o here instead of guessing.
    let output_path = args
        .output
        .as_deref()
        .ok_or("Merge mode requires an explicit output path (-o/--output).")?;
    merge_icons(&args.input, output_path, resolved.force, resolved.silent)?;
    finish_run(args, resolved, &args.input, Some(output_path))
}
