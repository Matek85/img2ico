// Everything clap needs to build the command-line interface: the two
// selectable-value enums (SizePreset, OutputFormat) and the top-level Args
// struct they're used from. Kept together in one module since they're
// really one unit - clap reads all of it together to build a single
// coherent --help output.

use crate::layout::FitMode;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
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

/// What `img2ico --version` prints (after the program name): the version,
/// and the facts a bug report needs - the platform and the compiler the
/// binary was built with. Both come from build.rs. `-V` prints just the
/// version.
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\ntarget:   ",
    env!("IMG2ICO_TARGET"),
    "\ncompiler: ",
    env!("IMG2ICO_COMPILER"),
);

#[derive(Parser, Debug)]
#[command(
    name = "img2ico",
    about = "Converts any image into an ICO file (with transparency)",
    version,
    long_version = LONG_VERSION,
    // --json belongs to the two modes that print a report.
    group(clap::ArgGroup::new("listing").args(["inspect", "validate"]).multiple(true)),
)]
pub struct Args {
    /// Input file(s) or folder(s). A single "-" reads the image from standard
    /// input instead (for pipelines: `cat logo.png | img2ico - -o icon.ico`);
    /// that needs -o, and works for converting one image only.
    /// - Normal mode: one image file (PNG, JPG, BMP, GIF, WebP, TIFF, TGA,
    ///   a macOS .icns - its largest icon is used - or an SVG, which is
    ///   drawn anew at every icon size) to convert - or
    ///   several files, or a folder, to convert them all in one go (batch
    ///   mode; see --output, --keep-going and --skip-existing). A folder
    ///   contributes the files of those formats directly inside it, in name
    ///   order; subfolders are not searched (see --recursive).
    /// - With --merge: two or more existing .ico files whose icons should
    ///   be combined into one output file.
    #[arg(required_unless_present = "completions")]
    pub input: Vec<PathBuf>,

    /// Path to the output file; "-" writes the icon to standard output
    /// (`img2ico logo.png -o - > icon.ico`), for converting one image. All
    /// messages then go to standard error, so standard output holds nothing
    /// but the icon. If not given, the input file's name is
    /// used, just with the ".ico" extension instead. In batch mode (several
    /// inputs, or a folder) this is instead a FOLDER that receives one icon
    /// per input, named after the input file; it is created if it doesn't
    /// exist. Without it, each icon is written next to its input.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Comma-separated list of icon sizes to embed into the .ico file.
    /// Windows ICOs can contain several resolutions at once (the operating
    /// system then picks the appropriate one depending on context, e.g.
    /// small for the taskbar, large for the desktop view). Defaults to
    /// "16,32,48,64,128,256" if neither this, a config file, nor --preset
    /// sets it.
    ///
    /// "--sizes auto" takes the default sizes but leaves out those larger
    /// than the longer edge of the source image (after --crop and --trim),
    /// so nothing is upscaled: a 100x100 image gives 16, 32, 48 and 64 (an
    /// image smaller than all of them gives one icon at its own size).
    /// A drawing (SVG) has no resolution of its own, so it gets all the
    /// default sizes.
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
    /// removing a solid-colored background. Instead of a color you can write
    /// "auto": img2ico then takes the most common color along the image
    /// border as the background, prints the color it found (so you can pass
    /// it as the color next time) and stops with a clear message if the
    /// border has no clearly dominant color, as with a gradient or a photo.
    /// If this option is omitted, nothing changes compared to the previous
    /// behavior.
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

    /// How soft the edge of a removed or replaced background is (0-100),
    /// only relevant together with --chroma-key or --find. It is the share of
    /// the --tolerance range used for a gradual transition at the edge of
    /// the artwork; the rest is a hard core around the background color.
    /// 0 = hard edge (everything within the tolerance is removed fully),
    /// 100 = the transition starts right at the exact background color.
    /// Defaults to 50 if neither this nor a config file sets it - the
    /// behavior img2ico has always had.
    ///
    /// Example: -c 00FF00 --feather 10 for a crisp edge on a clean
    /// background, --feather 80 for a smoother one on a noisy source.
    #[arg(long = "feather", verbatim_doc_comment)]
    pub feather: Option<u8>,

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

    /// Validate mode: checks the structure of existing .ico files (passed
    /// as INPUT - files, or folders, which stand for the .ico files in
    /// them; with --recursive also those in subfolders) and says exactly
    /// what is wrong with a damaged one. It looks at the header, the number
    /// of images, every directory entry (sizes, offsets, lengths), whether
    /// the images overlap or stick out of the file, and the image data
    /// itself - a PNG is checked chunk by chunk with its checksums, a BMP
    /// for its size and mask - and warns about unusual but harmless things
    /// (extra bytes, duplicate sizes, a cursor file). Prints a verdict per
    /// file and exits with a non-zero status if any file is invalid, so a
    /// build script can fail on it: `img2ico --validate icons/ --recursive`.
    /// Warnings alone do not make a file invalid. --json prints the result
    /// as JSON. Nothing is written to disk in this mode.
    #[arg(long = "validate")]
    pub validate: bool,

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

    /// How the image meets the square icon: "contain" (the default) fits
    /// the whole image inside the square - an image that is not square
    /// leaves transparent bars - and "cover" fills the whole square,
    /// cutting off what does not fit at the edges (the middle of the image
    /// is kept). --padding applies to both: with "cover" the image fills
    /// the area inside the padding.
    #[arg(long = "fit", value_enum)]
    pub fit: Option<FitMode>,

    /// Cuts this part out of the source image before anything else is
    /// done with it: X,Y,WIDTH,HEIGHT in pixels, counted from the top left
    /// corner - --crop 10,10,200,200 takes the 200x200 pixels starting at
    /// 10 pixels from the left and from the top. The part has to lie inside
    /// the image. Happens after a background removal (so --seed positions
    /// still refer to the whole image) and before --trim. For raster
    /// images only: an SVG has no pixels to count.
    #[arg(long = "crop", value_name = "X,Y,WIDTH,HEIGHT")]
    pub crop: Option<String>,

    /// Cuts off the transparent margin around the artwork, so it fills the
    /// icon instead of floating in a frame (and is centered). Runs after
    /// any background removal, which makes this the way to trim a logo on
    /// a solid background: --chroma-key ffffff --trim. A margin that is not
    /// transparent (a plain JPG, say) has nothing to cut - img2ico says so
    /// and leaves the image as it is. Also works for an SVG, where it fits
    /// the drawing's content instead of its page.
    #[arg(long = "trim")]
    pub trim: bool,

    /// Rounds the corners of the image, as the percentage of its shorter
    /// edge (0-50) the corner curve takes up: 0 (the default) leaves them
    /// square, around 20 gives the look of an app icon, 50 makes a circle
    /// (or a pill for an image that is not square). The edge is smooth at
    /// every size. The rounding follows the image, not the canvas, so with
    /// --padding the rounded image floats inside its margin.
    #[arg(long = "corner-radius", value_name = "PERCENT", value_parser = clap::value_parser!(u8).range(0..=50))]
    pub corner_radius: Option<u8>,

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

    /// Suppresses the normal output of a run: the "Done: ..." line, the
    /// progress lines and summary of a batch, and the notices about settings
    /// files and written files. Errors, warnings (see --silent for those),
    /// and the output a mode exists to produce - an --inspect or --find
    /// report, a --what-if rehearsal - are not affected. Command-line only.
    ///
    /// --quiet and --silent are separate: --silent hides warnings and hints,
    /// --quiet hides the success output. Together, only errors remain.
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Prints diagnostic details to standard error while working: the
    /// settings actually in effect (command line, settings files and
    /// defaults combined - so you can see which value came from where), the
    /// source image, the output, each generated size with its file size,
    /// and how long the steps took. Standard output is left alone. Useful
    /// for bug reports and for checking what a settings file really does.
    /// Command-line only - it isn't a settings-file option - and cannot be
    /// combined with --silent.
    #[arg(short = 'v', long = "verbose", conflicts_with = "silent")]
    pub verbose: bool,

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

    /// Batch mode: when converting several files (or a folder) and one of
    /// them fails, carry on with the rest instead of stopping at the first
    /// failure. Every failure is reported, and the run still ends with a
    /// failing exit code (and deletes no source files) if any file failed.
    /// Has no effect on a single file.
    #[arg(long = "keep-going")]
    pub keep_going: bool,

    /// Skips an input whose output already exists, instead of failing -
    /// which makes a batch run safe to repeat: only what is missing gets
    /// converted. The existing output is left untouched, and skipped
    /// inputs are never deleted by --delete-source. Contradicts --force,
    /// which overwrites instead.
    #[arg(long = "skip-existing", conflicts_with = "force")]
    pub skip_existing: bool,

    /// Batch mode: also search the SUBFOLDERS of every folder given as input
    /// (all the way down), not just the folder itself. Only meaningful when
    /// an input is a folder. Subfolders are visited in name order, so a run
    /// is reproducible. Can also be set in a settings file (`recursive`);
    /// like all folder options it only matters when an input is a folder.
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Only convert folder files matching this pattern (repeatable).
    ///
    /// A pattern without a "/" is matched against the file NAME, wherever
    /// in the folder tree the file sits. A pattern with a "/" is matched
    /// against the PATH below the input folder, always written with forward
    /// slashes (also on Windows). Upper/lower case does not matter.
    ///
    /// Wildcards:
    ///   *        any characters, but not across folders
    ///   ?        exactly one character
    ///   [abc]    one of these characters ([0-9]: one digit)
    ///   {a,b}    either alternative
    ///   **       any number of folders (only as a whole folder part)
    ///
    /// Give --include several times to take files matching ANY of the
    /// patterns. Only supported image types (PNG, JPG, BMP, GIF, WebP, TIFF, TGA,
    /// ICNS, SVG) are ever
    /// converted - a pattern narrows them down, it cannot add others. It
    /// applies to what a FOLDER contributes; a file named directly on the
    /// command line is always converted. Put the pattern in quotes, or your
    /// shell may expand it before img2ico sees it. Use --exclude to leave
    /// files out.
    ///
    /// Can also be set in a settings file (`include = ["*.png"]`), where
    /// it only matters when an input is a folder. A list given on the
    /// command line replaces the file's list.
    ///
    /// Examples:
    ///   --include "*.png"           only PNG files
    ///   --include "icon-??.png"     icon-16.png, icon-32.png, ...
    ///   --include "*.{png,gif}"     PNG and GIF files
    ///   --include "ui/*.png"        PNG files directly inside the ui folder
    #[arg(long = "include", value_name = "GLOB", verbatim_doc_comment)]
    pub include: Vec<String>,

    /// Leave out folder files matching this pattern (repeatable).
    ///
    /// Same patterns as --include ("*", "?", "[abc]", "{a,b}", "**"; a
    /// pattern with a "/" matches the path below the input folder, one
    /// without it the file name). A file matching ANY --exclude pattern is
    /// skipped, even if it also matches an --include pattern. Like
    /// --include it applies to what a FOLDER contributes only, and the
    /// pattern should be put in quotes. Can also be set in a settings file
    /// (`exclude = ["*_old*"]`), where it only matters when an input is a
    /// folder; a list given on the command line replaces the file's list.
    ///
    /// Examples:
    ///   --exclude "*_old*"          every file with "_old" in its name
    ///   --exclude "ui/*"            files directly inside ui (not deeper)
    ///   --exclude "backup/**"       everything below the top-level backup
    ///   --exclude "**/old/**"       everything in any folder called old
    ///
    /// Combined: --include "*.png" --exclude "backup/**" takes all PNG
    /// files except those below backup.
    #[arg(long = "exclude", value_name = "GLOB", verbatim_doc_comment)]
    pub exclude: Vec<String>,

    /// Batch mode with -o: rebuild the input folder's subfolder structure
    /// below the output folder (assets/ui/save.png becomes
    /// icons/ui/save.ico) instead of putting every icon directly into it.
    /// Subfolders are created as needed. Without it, two inputs that share a
    /// file name in different folders would collide and the run is refused.
    /// Command-line only.
    #[arg(long = "keep-structure")]
    pub keep_structure: bool,

    /// Chooses the icons' file names: a pattern with the variables {stem}
    /// (the input's name without extension), {ext} (its extension, lowercase)
    /// and {format} (ico or icns), e.g. --name "{stem}-app". The icon's own
    /// extension is added after it. Without it, the name is just {stem}.
    /// Handy to keep logo.png and logo.jpg apart: --name "{stem}-{ext}".
    /// Not usable together with an explicit output FILE. Command-line only.
    #[arg(long = "name", value_name = "PATTERN")]
    pub name: Option<String>,

    /// Rehearsal: shows what would happen - for every input the output file
    /// it would write, the icon sizes, and whether the output already exists
    /// (and would be skipped, overwritten or refused) - without converting or
    /// writing anything, not even --out-toml. Also checks the options and
    /// looks for output name collisions, exactly like a real run. Cannot be
    /// combined with --report.
    #[arg(long = "what-if", alias = "dry-run", conflicts_with = "report")]
    pub what_if: bool,

    /// Writes a report about the run to this file: one line per input with
    /// the output, status (converted, skipped, failed), file size, icon
    /// sizes, warning count, duration and error message, plus totals. The
    /// format follows the extension: .csv or .json. Written even when files
    /// failed, and replaced if it exists. Command-line only.
    #[arg(long = "report", value_name = "FILE")]
    pub report: Option<PathBuf>,

    /// Prints the SHA-256 checksum of every icon file written by this run,
    /// one line per file in the format of the `sha256sum` tool - the hash,
    /// two spaces, the file - so the output can be saved and checked later
    /// with `sha256sum -c`. It is the hash of exactly the bytes written
    /// (also with `-o -`, where the line goes to standard error and names
    /// the file `-`). The lines are printed even with --quiet, so
    /// `img2ico icons/ -o out --quiet --checksum > SHA256SUMS` leaves
    /// nothing but the checksums. A file that was skipped (--skip-existing)
    /// or not written (--find preview) has none. With --report the checksum
    /// is a column of the report too. Useful to show that two runs - on
    /// different machines, say - produce identical icons. Command-line only.
    #[arg(long = "checksum", conflicts_with = "what_if")]
    pub checksum: bool,

    /// With --inspect or --validate: prints the report as JSON (one array
    /// with an entry per file) instead of text, for scripts. Nothing else is printed to
    /// standard output.
    #[arg(long = "json", requires = "listing")]
    pub json: bool,

    /// Loads default values for the "tuning" settings above (--sizes,
    /// --preset, --chroma-key, --tolerance, --seed, --find,
    /// --find-min-size, --auto-apply, --replace-color, --grayscale,
    /// --feather, --padding, --fit, --crop, --trim, --corner-radius, --gif-frame,
    /// --output-format, --delete-source, --force,
    /// --skip-existing, --keep-going, --recursive, --include, --exclude,
    /// --combine, --index, --silent) from a TOML file. An explicit
    /// command-line flag for the same setting still wins over whatever
    /// the file says - this only changes what happens when you DON'T
    /// pass a flag. Deliberately does NOT cover the mode
    /// (--merge/--inspect/--extract/--select), the input file(s), -o/--output,
    /// the output side of a batch (--keep-structure, --name) or the run's own output
    /// (--what-if, --report, --json) - those stay command-line-only, since
    /// defaulting those rarely makes sense.
    ///
    /// If this is omitted entirely, img2ico looks for "img2ico.toml" in
    /// the current directory instead and uses it automatically if
    /// present - --config takes priority over that automatic lookup
    /// entirely (the two are never combined). Below that project file sits
    /// an optional per-user file (Windows: %APPDATA%\img2ico\config.toml,
    /// macOS: ~/Library/Application Support/img2ico/config.toml, elsewhere:
    /// ~/.config/img2ico/config.toml) that supplies defaults for anything
    /// the project file doesn't set. Either way, a short notice is printed
    /// naming each file actually used, so this never silently changes
    /// behavior without saying so.
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Ignores every settings file for this run: no --config, no
    /// "img2ico.toml" in the current directory and no per-user file -
    /// only the command line and img2ico's built-in defaults apply. Handy
    /// for reproducing a result, or when an automatically picked-up file
    /// gets in the way. Cannot be combined with --config.
    #[arg(long = "no-config", conflicts_with = "config")]
    pub no_config: bool,

    /// After resolving all the "tuning" settings for this run (built-in
    /// defaults, any config file, and command-line flags - in that
    /// priority order, lowest to highest), writes them out to this path
    /// as a TOML file - handy for turning a hand-tuned command line into
    /// a reusable config file for next time (see --config). The normal
    /// conversion still runs as usual; this just additionally saves a
    /// snapshot of the settings that were actually used.
    #[arg(long = "out-toml")]
    pub out_toml: Option<PathBuf>,

    /// Prints a tab-completion script for the given shell (bash, zsh, fish,
    /// powershell or elvish) to standard output and exits - nothing is
    /// converted. Save it where your shell looks for completions, e.g.
    /// `img2ico --completions bash > ~/.local/share/bash-completion/completions/img2ico`
    /// or, for PowerShell, add `img2ico --completions powershell | Out-String | Invoke-Expression`
    /// to your profile. Cannot be combined with any other option.
    #[arg(
        long = "completions",
        value_enum,
        value_name = "SHELL",
        exclusive = true
    )]
    pub completions: Option<Shell>,
}

/// Writes the tab-completion script for `shell` to `out`.
///
/// The whole script is built in memory first and written in one go, so a
/// reader that closes the pipe early (e.g. `img2ico --completions bash | head`)
/// is handled here as an ordinary, quiet stop - instead of as a panic from
/// inside the generator, which writes piece by piece and treats any failure
/// as fatal.
pub fn write_completions(shell: Shell, out: &mut dyn std::io::Write) -> Result<(), String> {
    let mut script: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut Args::command(), "img2ico", &mut script);
    match out.write_all(&script).and_then(|()| out.flush()) {
        Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => {
            Err(format!("Could not write the completion script: {e}"))
        }
        _ => Ok(()),
    }
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

    // --- Version ---------------------------------------------------------------

    #[test]
    fn short_version_prints_only_the_version() {
        let err = parse(&["-V"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
        let text = err.to_string();
        assert_eq!(
            text.trim(),
            format!("img2ico {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn long_version_adds_platform_and_compiler() {
        let err = parse(&["--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
        let text = err.to_string();
        assert!(
            text.starts_with(&format!("img2ico {}", env!("CARGO_PKG_VERSION"))),
            "{text}"
        );
        assert!(text.contains("target:"), "{text}");
        assert!(text.contains("compiler: rustc"), "{text}");
    }

    #[test]
    fn version_does_not_need_an_input_file() {
        // Without special handling, the required INPUT would be reported missing.
        assert_eq!(
            parse(&["--version"]).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayVersion
        );
    }

    // --- Batch flags -----------------------------------------------------------

    #[test]
    fn batch_flags_default_to_off_and_can_be_turned_on() {
        let args = parse(&["a.png"]).unwrap();
        assert!(!args.keep_going && !args.skip_existing);
        let args = parse(&["a.png", "b.png", "--keep-going", "--skip-existing"]).unwrap();
        assert!(args.keep_going && args.skip_existing);
        assert_eq!(args.input.len(), 2);
    }

    #[test]
    fn skip_existing_contradicts_force() {
        assert!(parse(&["a.png", "--skip-existing", "--force"]).is_err());
        assert!(parse(&["a.png", "--skip-existing", "-f"]).is_err());
    }

    #[test]
    fn folder_and_naming_flags_are_parsed() {
        let args = parse(&[
            "assets",
            "-r",
            "--include",
            "*.png",
            "--include",
            "*.gif",
            "--exclude",
            "*_old*",
            "--keep-structure",
            "--name",
            "{stem}-app",
        ])
        .unwrap();
        assert!(args.recursive && args.keep_structure);
        assert_eq!(args.include, ["*.png", "*.gif"]);
        assert_eq!(args.exclude, ["*_old*"]);
        assert_eq!(args.name.as_deref(), Some("{stem}-app"));

        let plain = parse(&["a.png"]).unwrap();
        assert!(!plain.recursive && !plain.keep_structure && !plain.what_if && !plain.json);
        assert!(plain.include.is_empty() && plain.exclude.is_empty());
        assert!(plain.name.is_none() && plain.report.is_none());
    }

    #[test]
    fn what_if_and_report_exclude_each_other() {
        assert!(parse(&["a.png", "--what-if"]).is_ok());
        assert!(parse(&["a.png", "--report", "r.csv"]).is_ok());
        assert!(parse(&["a.png", "--what-if", "--report", "r.csv"]).is_err());
    }

    #[test]
    fn the_old_name_dry_run_still_works() {
        assert!(parse(&["a.png", "--dry-run"]).unwrap().what_if);
    }

    #[test]
    fn json_needs_inspect() {
        assert!(parse(&["a.ico", "--inspect", "--json"]).is_ok());
        assert!(parse(&["a.ico", "--json"]).is_err());
        assert!(parse(&["a.ico", "--validate", "--json"]).is_ok());
    }

    // --- Completions -----------------------------------------------------------

    #[test]
    fn completions_need_no_input_file() {
        let args = parse(&["--completions", "bash"]).unwrap();
        assert_eq!(args.completions, Some(Shell::Bash));
        assert!(args.input.is_empty());
    }

    #[test]
    fn every_supported_shell_is_accepted() {
        for (name, shell) in [
            ("bash", Shell::Bash),
            ("zsh", Shell::Zsh),
            ("fish", Shell::Fish),
            ("powershell", Shell::PowerShell),
            ("elvish", Shell::Elvish),
        ] {
            assert_eq!(
                parse(&["--completions", name]).unwrap().completions,
                Some(shell)
            );
        }
        assert!(parse(&["--completions", "cmd"]).is_err());
        assert!(parse(&["--completions"]).is_err());
    }

    #[test]
    fn completions_cannot_be_combined_with_anything_else() {
        assert!(parse(&["logo.png", "--completions", "bash"]).is_err());
        assert!(parse(&["--completions", "bash", "--force"]).is_err());
    }

    #[test]
    fn a_completion_script_mentions_the_program_and_its_flags() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let mut out: Vec<u8> = Vec::new();
            write_completions(shell, &mut out).unwrap();
            let script = String::from_utf8(out).unwrap();
            assert!(script.contains("img2ico"), "{shell:?}");
            assert!(script.contains("chroma-key"), "{shell:?}");
            assert!(script.contains("no-config"), "{shell:?}");
        }
    }

    #[test]
    fn a_closed_pipe_is_not_an_error() {
        struct ClosedPipe;
        impl std::io::Write for ClosedPipe {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(write_completions(Shell::Bash, &mut ClosedPipe), Ok(()));
    }

    #[test]
    fn other_write_failures_are_reported() {
        struct Full;
        impl std::io::Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let err = write_completions(Shell::Bash, &mut Full).unwrap_err();
        assert!(err.contains("disk full"), "{err}");
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
