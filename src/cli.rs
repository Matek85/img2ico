// Everything clap needs to build the command-line interface: the two
// selectable-value enums (SizePreset, OutputFormat) and the top-level Args
// struct they're used from. Kept together in one module since they're
// really one unit - clap reads all of it together to build a single
// coherent --help output.

use clap::Parser;
use std::path::PathBuf;

/// Sizes Microsoft recommends including so Windows always has an exact
/// match for every common DPI scaling level, instead of having to stretch
/// a nearby size and lose sharpness. Used both by SizePreset::Windows (to
/// generate exactly this set) and by ico_ops::inspect_icons (to warn about
/// missing sizes in an existing .ico).
pub const RECOMMENDED_WINDOWS_SIZES: [u32; 10] = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

// clap automatically generates the following from this struct:
// - command-line argument parsing
// - a --help output
// - error messages for incorrect usage
//
// NOTE: "///" (three slashes) are Rust "doc comments". clap reads exactly
// these automatically and shows them in "--help". Regular comments with
// "//" (two slashes, like this one) do NOT end up in the help text - those
// are only meant as developer documentation for us.
/// A predefined set of icon sizes for a common use case, selectable via
/// --preset instead of listing sizes manually with --sizes.
///
/// clap's `ValueEnum` derive does the same job here that `Parser` does for
/// the whole `Args` struct: it generates the parsing/validation code (so
/// an invalid value like `--preset foo` gets a clear clap error listing
/// the valid options) and feeds `--help` from this enum's variants and
/// their doc comments, without us writing any of that by hand.
///
/// Adding a new preset later is just: add a variant here, add its doc
/// comment, and add one line to `SizePreset::sizes()` below - the CLI
/// parsing and --help text update themselves automatically.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SizePreset {
    /// Microsoft's recommended set for full DPI-scaling coverage (16, 20,
    /// 24, 32, 40, 48, 64, 96, 128, 256) - Windows never has to stretch a
    /// nearby size at any scaling level.
    Windows,
    /// The classic small multi-resolution favicon.ico set used on
    /// websites (16, 32, 48).
    Favicon,
    /// Just two sizes (16, 32) for a small file / quick test.
    Minimal,
}

impl SizePreset {
    /// Returns the concrete list of sizes this preset expands to.
    pub fn sizes(self) -> &'static [u32] {
        match self {
            SizePreset::Windows => &RECOMMENDED_WINDOWS_SIZES,
            SizePreset::Favicon => &[16, 32, 48],
            SizePreset::Minimal => &[16, 32],
        }
    }
}

/// Which icon container format to write, selectable via --output-format.
/// If --output-format isn't given at all (the field stays `None` in
/// `Args`), img2ico falls back to a platform-based default instead - see
/// where `use_icns` is computed in `run()` (main.rs).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// The Windows icon format.
    Ico,
    /// The macOS icon format.
    Icns,
}

#[derive(Parser, Debug)]
#[command(
    name = "img2ico",
    about = "Converts any image into an ICO file (with transparency)"
)]
pub struct Args {
    /// Input file(s).
    /// - Normal mode: exactly one image file (PNG, JPG, BMP, GIF) to convert.
    /// - With --merge: two or more existing .ico files whose icons should
    ///   be combined into one output file.
    #[arg(required = true)]
    pub input: Vec<PathBuf>,

    /// Path to the output file. If not given, the input file's name is
    /// used, just with the ".ico" extension instead.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Comma-separated list of icon sizes to embed into the .ico file.
    /// Windows ICOs can contain several resolutions at once (the operating
    /// system then picks the appropriate one depending on context, e.g.
    /// small for the taskbar, large for the desktop view). Defaults to
    /// "16,32,48,64,128,256" if neither this, a config file, nor --preset
    /// sets it.
    #[arg(short, long)]
    pub sizes: Option<String>,

    /// Overrides --sizes with a predefined set of sizes for a common use
    /// case, instead of listing sizes manually (see the presets listed
    /// below for what each one contains). --sizes is ignored whenever
    /// this is given. This is the Windows-side equivalent of what
    /// producing an .icns file always does unconditionally (uses Apple's
    /// full recommended size set, ignoring --sizes) - so --preset has no
    /// effect whenever the output ends up being .icns, whether that's
    /// because of an explicit --output-format icns or simply because you're
    /// running this on macOS, where icns is the default (see
    /// --output-format).
    #[arg(long = "preset", value_enum)]
    pub preset: Option<SizePreset>,

    /// Optional hex color code (e.g. "#00FF00" or "00ff00") for "chroma
    /// key" mode: starting from the image border, a connected region of
    /// this color is found and made transparent - the classic use case is
    /// removing a solid-colored background. If this option is omitted,
    /// nothing changes compared to the previous behavior.
    #[arg(short = 'c', long = "chroma-key")]
    pub chroma_key: Option<String>,

    /// Tolerance for the chroma-key color comparison (0-100), only
    /// relevant together with --chroma-key.
    /// 0   = only (nearly) exactly the given color is recognized.
    /// 100 = almost any color would count as "background" (not useful in
    ///       practice).
    /// A good starting value to experiment with is 15-30. Defaults to 20
    /// if neither this nor a config file sets it.
    #[arg(short = 't', long = "tolerance")]
    pub tolerance: Option<u8>,

    /// Extra starting point for the chroma-key flood fill, as "x,y"
    /// (pixel coordinates IN THE ORIGINAL IMAGE, not in the resulting
    /// icon). Can be given multiple times (e.g. --seed 200,50 --seed
    /// 40,300).
    ///
    /// Normally the flood fill only starts from the image border. If the
    /// desired background color is enclosed by a frame/ring (e.g. an icon
    /// with a dark circular outline, inside which the background color
    /// occurs again), the image border can't reach that area. With --seed
    /// you give a point that lies somewhere INSIDE that enclosed area
    /// (e.g. the pixel coordinate where you'd hover the mouse over the
    /// background color in an image editor) - from there the flood fill
    /// then spreads exactly the same way it does from the border.
    #[arg(long = "seed")]
    pub seeds: Vec<String>,

    /// Scans the source image for additional regions matching this hex
    /// color that the border-based --chroma-key flood fill can't reach on
    /// its own (the same situation --seed solves, but found automatically
    /// instead of you having to hunt for pixel coordinates in an image
    /// editor first). Prints each found region as a ready-to-use --seed
    /// value and stops there - unless --auto-apply is also given, in
    /// which case the found regions are used as seeds automatically and
    /// the full conversion proceeds right away, with --find's color
    /// acting as the --chroma-key target. Mutually exclusive with
    /// --chroma-key - use whichever one fits (or --find, plus additional
    /// manual --seed values for anything it doesn't catch).
    #[arg(long = "find")]
    pub find: Option<String>,

    /// Minimum region size (in pixels) for --find to report, only
    /// relevant together with --find. Smaller connected regions are
    /// ignored, since a handful of stray pixels is almost always
    /// anti-aliasing/JPEG noise rather than a deliberate area worth
    /// pointing out. Defaults to 9 (catches real background patches while
    /// staying quiet about noise) if neither this nor a config file sets
    /// it; set this to 1 if you want --find to catch every matching
    /// pixel, however small.
    #[arg(long = "find-min-size")]
    pub find_min_size: Option<usize>,

    /// Together with --find: instead of only printing the found regions,
    /// automatically use them as extra seed points (in addition to any
    /// --seed values also given) and proceed with the full conversion
    /// right away. Has no effect without --find.
    #[arg(long = "auto-apply")]
    pub auto_apply: bool,

    /// Instead of making the detected background transparent, replaces it
    /// with this solid hex color (same format as --chroma-key/--find,
    /// e.g. "#000000" or "000000"). Uses the exact same detection - flood
    /// fill from the border, --seed/--find for enclosed regions, --tolerance
    /// for how strict the color match is, and the same soft edge
    /// transition, just blending towards the new color instead of towards
    /// transparency. Requires --chroma-key or --find (there has to be a
    /// background color to replace in the first place). Alpha is left
    /// untouched - this changes color only, not transparency.
    #[arg(long = "replace-color")]
    pub replace_color: Option<String>,

    /// Converts the entire image to grayscale (removes all color, leaving
    /// only brightness) before generating the icon. Unlike --chroma-key/
    /// --replace-color, this affects every pixel uniformly, not just a
    /// detected background region - alpha (transparency) is left
    /// untouched. Applied last, after any --chroma-key/--replace-color
    /// processing, so a --replace-color color ends up grayscaled too if
    /// both are used together. Works standalone (no --chroma-key/--find
    /// needed) and combines with everything else (--padding, --preset,
    /// --output-format, ...).
    #[arg(long = "grayscale")]
    pub grayscale: bool,

    /// Merge mode: instead of converting an image, combine the icon
    /// entries of two or more existing .ico files (passed as INPUT) into
    /// a single output .ico file. Every entry is decoded and re-encoded
    /// as PNG, so the merged file has the same consistent 32-bit quality
    /// as a freshly converted one, regardless of how the source files
    /// were created. If two source files contain the same size, the
    /// first occurrence wins and later duplicates are skipped (with a
    /// warning). All image-specific options (--sizes, --chroma-key,
    /// --tolerance, --seed) are ignored in this mode.
    #[arg(long = "merge")]
    pub merge: bool,

    /// Inspect mode: instead of converting anything, print a report about
    /// one or more files (passed as INPUT). For an existing .ico: which
    /// sizes it contains, at what color depth, whether each entry is
    /// stored as PNG or as the legacy BMP format, and a few sanity
    /// warnings (e.g. reduced color depth, or common Windows DPI sizes
    /// that are missing). For a regular source image instead: its
    /// resolution, and which standard icon sizes (Windows and macOS) it
    /// can produce without upscaling versus which would come out
    /// soft/blurry - handy to check before converting. Nothing is written
    /// to disk in this mode.
    #[arg(long = "inspect")]
    pub inspect: bool,

    /// Extract mode: instead of converting an image, pull every icon size
    /// out of an existing .ico file (passed as INPUT, exactly one file)
    /// and save each one as a separate PNG. -o/--output is used as the
    /// target DIRECTORY here (not a file path) and is created if it
    /// doesn't exist yet; if omitted, a directory named after the input
    /// file is created next to it.
    #[arg(long = "extract")]
    pub extract: bool,

    /// Select mode: instead of converting an image, pull one or more
    /// specific icon(s) out of an existing .ico file (passed as INPUT,
    /// exactly one file) BY INDEX and re-export them as standalone .ico
    /// file(s) - unlike --extract, which always exports every size as
    /// PNG. Use --inspect first to see which index corresponds to which
    /// size (indices are shown there in [brackets]).
    #[arg(long = "select")]
    pub select: bool,

    /// Comma-separated list of zero-based indices to pull out, only
    /// relevant together with --select. If omitted, defaults to just
    /// index 0 (the first icon in the file). With more than one index:
    /// by default each selected icon is saved as its own separate .ico
    /// file into a directory (like --extract does for PNGs) - pass
    /// --combine to bundle them into a single multi-size .ico file
    /// instead.
    #[arg(long = "index")]
    pub index: Option<String>,

    /// Bundles multiple --index selections into a single output .ico file
    /// with all of them, instead of one separate file per selected icon
    /// (the default). Has no effect with only a single index.
    #[arg(long = "combine")]
    pub combine: bool,

    /// Adds transparent padding around the image before it's placed onto
    /// the square icon canvas, as a percentage (0-100) of the icon size.
    /// For example, --padding 10 on a 256x256 icon leaves roughly a 10%
    /// margin on every side, so the actual artwork ends up smaller and
    /// more centered instead of touching the edges. Defaults to 0 (fills
    /// the canvas as much as possible while preserving aspect ratio) if
    /// neither this nor a config file sets it.
    #[arg(long = "padding")]
    pub padding: Option<u8>,

    /// Which frame to use as the source image, if the input is an
    /// animated GIF (has no effect on any other format). Frames are
    /// numbered starting at 1 (the first frame), matching how you'd
    /// naturally describe it. Defaults to 1 if neither this nor a config
    /// file sets it - the same frame img2ico has always used, even
    /// before this option existed. Use `--inspect` on the GIF first to
    /// see how many frames it has.
    #[arg(long = "gif-frame")]
    pub gif_frame: Option<usize>,

    /// Suppresses all advisory warnings and informational notices (e.g.
    /// upscaling/thin-content warnings, an out-of-range --seed point, a
    /// skipped duplicate size in --merge, an unrecognized setting in a
    /// config file, the "Using settings from ..." notice). Does NOT
    /// suppress errors (those always need to reach you, in scripts
    /// especially) or the actual report --inspect/--find print, which is
    /// their whole purpose, not incidental chatter around it. Like the
    /// other tuning settings, this is config-file-eligible - but a config
    /// file's own --silent only takes effect once that file has already
    /// been loaded, so it never silences warnings about problems with the
    /// config file itself.
    #[arg(long = "silent")]
    pub silent: bool,

    /// Which icon container format to write: "ico" (the Windows format)
    /// or "icns" (the macOS format, using a fixed, Apple-recommended set
    /// of sizes - 16, 32, 64, 128, 256, 512 and 1024 pixels, each
    /// PNG-encoded - instead of --sizes, which is ignored whenever the
    /// output is icns). --chroma-key, --tolerance, --seed and --padding
    /// all still apply either way, since they're independent of the
    /// container format.
    ///
    /// If this is omitted entirely, img2ico picks automatically based on
    /// the platform it's running on: icns on macOS, ico everywhere else -
    /// since that matches what almost everyone building on a given
    /// platform actually wants. Pass --output-format explicitly to
    /// override that default in either direction (e.g. to produce a
    /// Windows .ico file while working on a Mac).
    ///
    /// Note: the icns container format was verified byte-by-byte against
    /// the public ICNS specification and cross-checked with an
    /// independent parser, but this was built without access to a real
    /// Mac - a real-world check on macOS is still recommended before
    /// relying on it for anything important.
    #[arg(long = "output-format", value_enum)]
    pub output_format: Option<OutputFormat>,

    /// Deletes the original source file(s) after a successful run, but
    /// only after the output has been written completely - if anything
    /// fails along the way, nothing gets deleted. Disabled by default,
    /// since accidentally deleting the only copy of a source image is a
    /// real risk; has to be turned on explicitly. Applies to --merge and
    /// --extract too (deletes the merged .ico files / the extracted .ico,
    /// respectively). Has no effect with --inspect, which never writes
    /// anything and has nothing to delete.
    ///
    /// Safety net: a source file is never deleted if it turns out to also
    /// be the output path (which would delete the just-written result
    /// instead of a source file) - that one file is skipped with a
    /// warning instead.
    #[arg(long = "delete-source")]
    pub delete_source: bool,

    /// Allows overwriting output file(s) that already exist. Disabled by
    /// default: without --force, img2ico refuses to run if the resolved
    /// output already exists on disk, instead of silently overwriting it,
    /// so converting the same file twice by accident doesn't quietly
    /// destroy the previous result. Applies to the normal output file
    /// (regardless of --output-format) and --merge; for --extract it
    /// checks every individual file that would be written before writing
    /// any of them, so a conflict never leaves a half-extracted directory
    /// behind.
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Loads default values for the "tuning" settings above (--sizes,
    /// --preset, --chroma-key, --tolerance, --seed, --find,
    /// --find-min-size, --auto-apply, --replace-color, --grayscale,
    /// --padding, --gif-frame, --output-format, --delete-source, --force,
    /// --combine, --index, --silent) from a TOML file. An explicit
    /// command-line flag for the same setting still wins over whatever
    /// the file says - this only changes what happens when you DON'T
    /// pass a flag. Deliberately does NOT cover the mode
    /// (--merge/--inspect/--extract/--select), the input file(s), or
    /// -o/--output - those stay command-line-only,
    /// since defaulting those rarely makes sense.
    ///
    /// If this is omitted entirely, img2ico looks for "img2ico.toml" in
    /// the current directory instead and uses it automatically if
    /// present - --config takes priority over that automatic lookup
    /// entirely (the two are never combined). Either way, a short notice
    /// is printed naming the file actually used, so this never silently
    /// changes behavior without saying so.
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// After resolving all the "tuning" settings for this run (built-in
    /// defaults, any config file, and command-line flags - in that
    /// priority order, lowest to highest), writes them out to this path
    /// as a TOML file - handy for turning a hand-tuned command line into
    /// a reusable config file for next time (see --config). The normal
    /// conversion still runs as usual; this just additionally saves a
    /// snapshot of the settings that were actually used.
    #[arg(long = "out-toml")]
    pub out_toml: Option<PathBuf>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Args, clap::Error> {
        let mut full = vec!["img2ico"];
        full.extend_from_slice(args);
        Args::try_parse_from(full)
    }

    #[test]
    fn the_clap_definition_is_internally_consistent() {
        Args::command().debug_assert();
    }

    // --- Presets ---------------------------------------------------------------

    #[test]
    fn windows_preset_is_the_recommended_size_set() {
        assert_eq!(SizePreset::Windows.sizes(), &RECOMMENDED_WINDOWS_SIZES);
        assert_eq!(SizePreset::Windows.sizes().len(), 10);
    }

    #[test]
    fn favicon_and_minimal_presets_have_their_documented_sizes() {
        assert_eq!(SizePreset::Favicon.sizes(), &[16, 32, 48]);
        assert_eq!(SizePreset::Minimal.sizes(), &[16, 32]);
    }

    #[test]
    fn recommended_windows_sizes_are_sorted_unique_and_valid_for_ico() {
        let sizes = RECOMMENDED_WINDOWS_SIZES;
        assert!(sizes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(sizes.iter().all(|&s| (1..=256).contains(&s)));
    }

    // --- Parsing ---------------------------------------------------------------

    #[test]
    fn an_input_file_is_required() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn a_bare_input_leaves_every_setting_unset() {
        let args = parse(&["in.png"]).unwrap();
        assert_eq!(args.input, vec![PathBuf::from("in.png")]);
        assert_eq!(args.output, None);
        assert_eq!(args.sizes, None);
        assert_eq!(args.preset, None);
        assert_eq!(args.tolerance, None);
        assert_eq!(args.padding, None);
        assert_eq!(args.gif_frame, None);
        assert_eq!(args.output_format, None);
        assert!(args.seeds.is_empty());
        assert!(!args.merge && !args.inspect && !args.extract && !args.select);
        assert!(!args.force && !args.silent && !args.delete_source && !args.grayscale);
    }

    #[test]
    fn short_flags_work() {
        let args = parse(&[
            "in.png", "-o", "out.ico", "-s", "16,32", "-c", "#00FF00", "-t", "30", "-f",
        ])
        .unwrap();
        assert_eq!(args.output, Some(PathBuf::from("out.ico")));
        assert_eq!(args.sizes.as_deref(), Some("16,32"));
        assert_eq!(args.chroma_key.as_deref(), Some("#00FF00"));
        assert_eq!(args.tolerance, Some(30));
        assert!(args.force);
    }

    #[test]
    fn seed_can_be_given_multiple_times() {
        let args = parse(&["in.png", "--seed", "1,2", "--seed", "3,4"]).unwrap();
        assert_eq!(args.seeds, vec!["1,2", "3,4"]);
    }

    #[test]
    fn several_inputs_are_collected_for_merge() {
        let args = parse(&["a.ico", "b.ico", "c.ico", "--merge"]).unwrap();
        assert_eq!(args.input.len(), 3);
        assert!(args.merge);
    }

    #[test]
    fn preset_and_output_format_accept_their_lowercase_names() {
        let args = parse(&["in.png", "--preset", "favicon", "--output-format", "icns"]).unwrap();
        assert_eq!(args.preset, Some(SizePreset::Favicon));
        assert_eq!(args.output_format, Some(OutputFormat::Icns));
    }

    #[test]
    fn unknown_preset_or_format_values_are_rejected() {
        assert!(parse(&["in.png", "--preset", "huge"]).is_err());
        assert!(parse(&["in.png", "--output-format", "tiff"]).is_err());
    }

    #[test]
    fn numeric_options_reject_values_out_of_range() {
        assert!(parse(&["in.png", "--tolerance", "256"]).is_err());
        assert!(parse(&["in.png", "--tolerance", "-1"]).is_err());
        assert!(parse(&["in.png", "--padding", "300"]).is_err());
        assert!(parse(&["in.png", "--gif-frame", "-2"]).is_err());
        assert!(parse(&["in.png", "--find-min-size", "abc"]).is_err());
    }

    #[test]
    fn unknown_flags_are_rejected() {
        assert!(parse(&["in.png", "--no-such-flag"]).is_err());
    }

    #[test]
    fn config_and_out_toml_take_paths() {
        let args = parse(&["in.png", "--config", "a.toml", "--out-toml", "b.toml"]).unwrap();
        assert_eq!(args.config, Some(PathBuf::from("a.toml")));
        assert_eq!(args.out_toml, Some(PathBuf::from("b.toml")));
    }
}
