// The conversion itself: a source image in, the bytes of an .ico or .icns file
// out. Everything between "the image is in memory" and "the icon is in
// memory" lives here - background removal, crop, trim, grayscale, fitting,
// scaling every size, encoding - so the command line and the web page run the
// very same code. Reading files, choosing output names, printing and writing
// are the callers' business.
//
// A raster source goes in two steps, because the command line has to stop
// between them for a `--find` preview: `analyze_raster` settles the
// background color (and, for `--find`, looks for regions), `convert_raster`
// does the rest. An SVG is drawn anew at every size (`convert_vector`).
// `convert` is the one-call form for callers that need no stop in between.

use crate::chroma_key::{
    FoundRegion, apply_chroma_key_feathered, detect_background_color, find_isolated_regions,
    format_hex, parse_hex_color, warn_about_removal_extent,
};
use crate::diag::{file_prefix, warn};
use crate::icns::{encode_icns, icns_sizes};
use crate::layout::{CropRect, Layout, Trimmed, auto_sizes, crop, make_icon, trim_transparent};
use crate::par::parallel_map;
use crate::resize::{
    AlphaMode, apply_grayscale, has_transparency, premultiply, shrink_for_sizes,
    warn_about_thin_content, warn_about_upscaling,
};
use crate::rotate::rotate;
use crate::source::Artwork;
use crate::vector::VectorImage;
use image::RgbaImage;
use std::time::{Duration, Instant};

/// The sizes of an .ico when nothing else is asked for.
pub const DEFAULT_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

/// ICO files officially only support edge lengths up to 256px.
pub const MAX_ICO_SIZE: u32 = 256;

/// A source with fewer pixels than this is scaled to its icon sizes one after
/// the other even when more threads are allowed: the sizes are done in a
/// few milliseconds then, and starting threads would cost more than it saves.
const PARALLEL_SIZES_MIN_PIXELS: u64 = 250_000;

/// The size an SVG is rendered at to detect and check the background color
/// once for all sizes.
const VECTOR_REFERENCE_SIZE: u32 = 256;

/// The largest edge a PNG output may have.
pub const MAX_PNG_SIZE: u32 = 1024;

/// Which kind of icon file to make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ico,
    Icns,
    /// A single PNG file, at the first of the sizes.
    Png,
}

/// Which sizes an .ico gets (an .icns has its own fixed set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sizes {
    /// Exactly these.
    Fixed(Vec<u32>),
    /// These, but only the ones the source can fill: what is possible depends
    /// on how big the source is after crop and trim.
    Auto(Vec<u32>),
}

/// How the background color is told, and what is done with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundMode {
    /// Remove the color from the image's border inward (`--chroma-key`).
    ChromaKey,
    /// Also look for regions of the color the border cannot reach (`--find`).
    /// `min_size` is the smallest region, in pixels, worth reporting.
    Find { min_size: usize },
}

/// A background color to remove or replace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    pub mode: BackgroundMode,
    /// A hex color (`00FF00`, `#00FF00`) or `auto`, to detect it from the
    /// image's border.
    pub spec: String,
    pub tolerance: u8,
    /// How soft the edge of what is removed is, in percent (0-100).
    pub feather: u8,
    /// Extra starting points for the removal, where the border cannot reach.
    pub seeds: Vec<(u32, u32)>,
    /// A color to put in place of the background instead of transparency.
    pub replacement: Option<[u8; 3]>,
}

/// Everything that decides what the icon looks like.
#[derive(Debug, Clone)]
pub struct Options {
    pub format: Format,
    pub sizes: Sizes,
    pub layout: Layout,
    pub background: Option<Background>,
    pub grayscale: bool,
    /// The picture mirrored left to right, before it is turned.
    pub flip_horizontal: bool,
    /// The picture mirrored top to bottom, before it is turned.
    pub flip_vertical: bool,
    /// The picture turned clockwise by this many degrees before it is cropped (0: not turned).
    pub rotate: i32,
    pub crop: Option<CropRect>,
    pub trim: bool,
    /// How many threads the sizes may be made on. 1 does it in a plain loop
    /// (a browser page has no threads to give).
    pub threads: usize,
    /// A color to lay the icon on, so that nothing is transparent: for the
    /// icon of an iPhone's home screen, which would fill transparency with
    /// black.
    pub flatten: Option<[u8; 3]>,
    /// Hides the warnings that would be printed. They are counted, and
    /// collected by `diag::collect`, either way.
    pub silent: bool,
}

/// The finished icon file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub bytes: Vec<u8>,
    /// The edge lengths that went into the file, in order.
    pub sizes: Vec<u32>,
}

/// Where the commentary of a conversion goes. All of it is optional: the
/// default does nothing at all, which is what a caller without a console
/// wants.
pub trait Notes {
    /// A line of the run's own commentary, such as which background color was
    /// detected.
    fn say(&self, _line: &str) {}
    /// A line of detail for a verbose run.
    fn verbose(&self, _line: &str) {}
    /// Whether `verbose` lines are wanted. When they are not, they are not even
    /// made - and nothing is timed, which matters in a browser, where there is
    /// no clock to ask.
    fn verbose_enabled(&self) -> bool {
        false
    }
}

/// Notes that say nothing.
pub struct NoNotes;

impl Notes for NoNotes {}

fn note(notes: &dyn Notes, line: impl FnOnce() -> String) {
    if notes.verbose_enabled() {
        notes.verbose(&line());
    }
}

/// A stopwatch that only runs when someone will read the time.
struct Stopwatch(Option<Instant>);

impl Stopwatch {
    fn start(notes: &dyn Notes) -> Self {
        Stopwatch(notes.verbose_enabled().then(Instant::now))
    }

    fn elapsed(&self) -> Duration {
        self.0.map_or(Duration::ZERO, |started| started.elapsed())
    }
}

/// The background color of a run and how to refer to it.
#[derive(Debug)]
pub struct BackgroundTarget {
    /// The option that named it: `--chroma-key` or `--find`.
    pub flag: &'static str,
    pub color: [u8; 3],
    /// What to call the color in messages: the hex code as it was written, or
    /// `#RRGGBB` for a detected one.
    pub label: String,
}

/// What `analyze_raster` found out about a raster source.
#[derive(Debug, Default)]
pub struct Analysis {
    /// The background color, if the options name one.
    pub target: Option<BackgroundTarget>,
    /// With `--find`: the regions of that color the border cannot reach.
    pub regions: Vec<FoundRegion>,
}

impl BackgroundMode {
    fn flag(self) -> &'static str {
        match self {
            BackgroundMode::ChromaKey => "--chroma-key",
            BackgroundMode::Find { .. } => "--find",
        }
    }
}

/// Works out the background color of this run, if the options name one: a hex
/// code is parsed, and "auto" is detected from the border of `source` (and
/// announced, so it can be passed explicitly next time). In a batch every file
/// detects its own.
fn resolve_background(
    source: &RgbaImage,
    options: &Options,
    notes: &dyn Notes,
) -> Result<Option<BackgroundTarget>, String> {
    let Some(background) = &options.background else {
        return Ok(None);
    };
    let flag = background.mode.flag();
    let spec = background.spec.trim();

    if !spec.eq_ignore_ascii_case("auto") {
        let color = parse_hex_color(spec).map_err(|e| {
            crate::msg!(
                "convert.invalid_background_value",
                "Invalid {flag} value: {e}",
                flag = flag,
                e = e
            )
        })?;
        return Ok(Some(BackgroundTarget {
            flag,
            color,
            label: spec.to_string(),
        }));
    }

    let clock = Stopwatch::start(notes);
    let found = detect_background_color(source, background.tolerance)?;
    let label = format_hex(found.color);
    if !options.silent {
        notes.say(&format!(
            "{}Detected background color {label} ({:.0}% of the image border). To use it explicitly: {flag} {}",
            file_prefix(),
            found.coverage * 100.0,
            label.trim_start_matches('#')
        ));
    }
    note(notes, || {
        format!(
            "{}background detection took {:.1?}",
            file_prefix(),
            clock.elapsed()
        )
    });
    Ok(Some(BackgroundTarget {
        flag,
        color: found.color,
        label,
    }))
}

/// The first step for a raster source: settles the background color and, for
/// `--find`, looks for the regions of it that the border-based removal cannot
/// reach on its own.
pub fn analyze_raster(
    source: &RgbaImage,
    options: &Options,
    notes: &dyn Notes,
) -> Result<Analysis, String> {
    let target = resolve_background(source, options, notes)?;
    let mut regions = Vec::new();
    if let (Some(target), Some(background)) = (&target, &options.background)
        && let BackgroundMode::Find { min_size } = background.mode
    {
        regions = find_isolated_regions(
            source,
            target.color,
            background.tolerance,
            min_size,
            options.silent,
        );
    }
    Ok(Analysis { target, regions })
}

/// Rejects what an SVG cannot do: the options that name pixel positions mean
/// something different at every size, and an SVG is drawn anew at every size.
pub fn check_vector_options(options: &Options) -> Result<(), String> {
    let unsupported = if matches!(
        options.background.as_ref().map(|b| b.mode),
        Some(BackgroundMode::Find { .. })
    ) {
        Some("--find")
    } else if options
        .background
        .as_ref()
        .is_some_and(|b| !b.seeds.is_empty())
    {
        Some("--seed")
    } else if options.crop.is_some() {
        Some("--crop")
    } else if crate::rotate::normalize(options.rotate) != 0 {
        Some("--rotate")
    } else if options.flip_horizontal {
        Some("--flip-horizontal")
    } else if options.flip_vertical {
        Some("--flip-vertical")
    } else {
        None
    };
    match unsupported {
        Some(option) => Err(vector_refusal(option)),
        None => Ok(()),
    }
}

/// The message for an option an SVG cannot honor.
pub fn vector_refusal(option: &str) -> String {
    crate::msg!(
        "convert.vector_option",
        "{option} does not apply to an SVG: it names pixel positions, and an SVG is drawn anew at every size. Use --chroma-key to remove a background color, --trim to cut the empty margin, or convert to a raster image first.",
        option = option
    )
}

/// Applies the background removal or replacement, if a background color was
/// named: `--chroma-key` (with the given seed values), or `--find` (the seed
/// values plus the regions found). Does nothing without one. Warns if almost
/// nothing, or almost everything, matched.
pub fn remove_background(
    img: &mut RgbaImage,
    options: &Options,
    analysis: &Analysis,
    notes: &dyn Notes,
) -> Result<(), String> {
    let (Some(target), Some(background)) = (&analysis.target, &options.background) else {
        return Ok(());
    };

    let clock = Stopwatch::start(notes);
    let mut seeds = background.seeds.clone();
    seeds.extend(analysis.regions.iter().map(|region| region.seed));
    let affected = apply_chroma_key_feathered(
        img,
        target.color,
        background.tolerance,
        background.feather,
        &seeds,
        background.replacement,
        options.silent,
    );
    warn_about_removal_extent(
        affected,
        img.width() as usize * img.height() as usize,
        &target.label,
        options.silent,
    );
    note(notes, || {
        format!(
            "background removal ({} {}, {} seed point(s), feather {}%) took {:.1?}",
            target.flag,
            target.label,
            seeds.len(),
            background.feather,
            clock.elapsed()
        )
    });
    Ok(())
}

/// --trim for a raster image: cut off the transparent margin, or say why
/// nothing was cut.
fn trim_source(source: &mut RgbaImage, options: &Options, notes: &dyn Notes) {
    match trim_transparent(source) {
        Trimmed::Cut(cut) => {
            note(notes, || {
                format!(
                    "{}trimmed from {}x{} to {}x{} pixels",
                    file_prefix(),
                    source.width(),
                    source.height(),
                    cut.width(),
                    cut.height()
                )
            });
            *source = cut;
        }
        Trimmed::NothingToCut => warn(
            options.silent,
            crate::msg!(
                "trim.nothing_to_cut",
                "Warning: {prefix}--trim found no transparent margin to cut - the image has content up to its edges. For a solid-color background, remove it first (--chroma-key).",
                prefix = file_prefix()
            ),
        ),
        Trimmed::Empty => warn(
            options.silent,
            crate::msg!(
                "trim.empty",
                "Warning: {prefix}--trim found nothing to keep - the image is completely transparent.",
                prefix = file_prefix()
            ),
        ),
    }
}

/// A heads-up (not an error - upscaling still produces a valid icon, just a
/// softer one) if any requested size exceeds what the source image actually
/// has to offer, or would leave only a sliver of content.
fn warn_about_small_source(source: &RgbaImage, options: &Options, ico_sizes: &[u32]) {
    let (width, height) = source.dimensions();
    let icns_list;
    let sizes = if options.format == Format::Icns {
        icns_list = icns_sizes();
        &icns_list
    } else {
        ico_sizes
    };
    warn_about_upscaling(width, height, sizes, options.silent);
    warn_about_thin_content(width, height, options.layout.padding, sizes, options.silent);
}

/// The sizes an .ico gets before anything is known about the source.
fn starting_sizes(options: &Options) -> &[u32] {
    match &options.sizes {
        Sizes::Fixed(list) | Sizes::Auto(list) => list,
    }
}

/// Converts a raster source that `analyze_raster` has looked at.
pub fn convert_raster(
    mut source: RgbaImage,
    options: &Options,
    analysis: &Analysis,
    notes: &dyn Notes,
) -> Result<Converted, String> {
    remove_background(&mut source, options, analysis, notes)?;

    // The picture is mirrored, then turned, after the background is gone (so the color is found on the
    // picture as it is, not on a canvas with transparent corners) and before the crop, whose frame is
    // placed on the mirrored and turned picture.
    crate::rotate::flip(&mut source, options.flip_horizontal, options.flip_vertical);
    // A free angle with a frame on it: only the frame's pixels are turned (the same picture, much sooner).
    let mut cropped = false;
    if crate::rotate::normalize(options.rotate) != 0 {
        let turned = match options.crop {
            Some(rect) if crate::rotate::is_free_angle(options.rotate) => {
                let canvas =
                    crate::rotate::rotated_size(source.width(), source.height(), options.rotate);
                source = crate::rotate::rotate_then_crop(
                    &source,
                    options.rotate,
                    rect,
                    crate::source::DEFAULT_MAX_PIXELS,
                )?;
                cropped = true;
                canvas
            }
            _ => {
                source = rotate(&source, options.rotate)?;
                source.dimensions()
            }
        };
        note(notes, || {
            format!(
                "{}rotated by {} degrees to {}x{} pixels",
                file_prefix(),
                options.rotate,
                turned.0,
                turned.1
            )
        });
    }

    // The part of the image that is wanted: --crop first, then --trim, which
    // works on what is left. Both come after the background removal, so
    // --seed positions still refer to the whole image and --trim finds the
    // transparent margin a removed background leaves.
    if let Some(rect) = options.crop {
        if !cropped {
            source = crop(&source, rect)?;
        }
        note(notes, || {
            format!(
                "{}cropped to {}x{} pixels",
                file_prefix(),
                source.width(),
                source.height()
            )
        });
    }
    if options.trim {
        trim_source(&mut source, options, notes);
    }

    // --grayscale runs LAST, after any background processing above - so it
    // uniformly affects the final colors, including a replacement color if
    // both were combined, rather than leaving a confusing "everything except
    // the replaced background is grayscale" exception.
    if options.grayscale {
        apply_grayscale(&mut source);
    }

    // `Sizes::Auto` can only be settled now: it depends on how big the image
    // is after crop and trim.
    let mut sizes = starting_sizes(options).to_vec();
    if matches!(options.sizes, Sizes::Auto(_)) {
        sizes = auto_sizes(&sizes, source.width().max(source.height()));
        note(notes, || {
            format!(
                "{}sizes auto: {sizes:?} (source {}x{} pixels)",
                file_prefix(),
                source.width(),
                source.height()
            )
        });
    }

    warn_about_small_source(&source, options, &sizes);

    // A huge picture is made smaller once, by a whole factor, so that the icons are not each scaled down from the
    // full picture (see `shrink_for_sizes`). After the sizes and the warnings, which go by the real size.
    if let Some(smaller) = sizes
        .iter()
        .max()
        .and_then(|&largest| shrink_for_sizes(&source, largest))
    {
        note(notes, || {
            format!(
                "{}the {}x{} pixel picture is made smaller to {}x{} pixels first, the icons are made from that",
                file_prefix(),
                source.width(),
                source.height(),
                smaller.width(),
                smaller.height()
            )
        });
        source = smaller;
    }

    // Computed ONCE here and passed down to every icon size instead of
    // re-checking per size - see has_transparency()'s doc comment for why
    // this matters. A source with transparency is multiplied with its alpha
    // ONCE, here - not once per icon size, which for a large image is a full
    // copy each time (see resize.rs). The sizes are then scaled from that.
    let alpha_mode = if has_transparency(&source) {
        premultiply(&mut source);
        AlphaMode::Premultiplied
    } else {
        AlphaMode::Opaque
    };

    let render = |size: u32| make_icon(&source, size, &options.layout, alpha_mode);
    // A small source is scaled so quickly that threads would not pay.
    let size_threads =
        if u64::from(source.width()) * u64::from(source.height()) >= PARALLEL_SIZES_MIN_PIXELS {
            options.threads
        } else {
            1
        };
    encode(&render, &sizes, options, size_threads, notes)
}

/// Converts an SVG: instead of loading one picture and scaling it down, the
/// drawing is rendered anew at every size of the icon. A background color can
/// still be removed (from every rendered size); the options that name pixel
/// positions are refused rather than silently ignored (`check_vector_options`).
pub fn convert_vector(
    drawing: &VectorImage,
    options: &Options,
    notes: &dyn Notes,
) -> Result<Converted, String> {
    check_vector_options(options)?;
    // Text is drawn only where there are fonts to draw it with (the command line, with the system's); a build
    // without that (the web page) leaves it out, and says so rather than hand back an icon that quietly lacks it.
    if cfg!(not(feature = "svg-text")) && drawing.has_text() {
        warn(
            options.silent,
            crate::msg!(
                "svg.text_left_out",
                "Warning: {prefix}this SVG has text, which is left out: there are no fonts here to draw it with. To keep it, turn the text into outlines (paths) in your drawing program.",
                prefix = file_prefix()
            ),
        );
    }
    // A drawing has no resolution to stay below, so `Sizes::Auto` gets all the
    // sizes it started with.
    let sizes = starting_sizes(options).to_vec();
    let layout = options.layout;

    // A background color (--chroma-key, also "auto") is removed from every
    // rendered size. The color is settled - and the removal checked and
    // reported - once, on a reference rendering; the sizes then each get the
    // same color without repeating the messages.
    let background = if options.background.is_some() {
        let mut reference = drawing.render(VECTOR_REFERENCE_SIZE, &layout, options.trim);
        let target = resolve_background(&reference, options, notes)?;
        let analysis = Analysis {
            target,
            regions: Vec::new(),
        };
        remove_background(&mut reference, options, &analysis, notes)?;
        analysis.target
    } else {
        None
    };

    let render = |size: u32| {
        let mut square = drawing.render(size, &layout, options.trim);
        if let (Some(target), Some(background)) = (&background, &options.background) {
            apply_chroma_key_feathered(
                &mut square,
                target.color,
                background.tolerance,
                background.feather,
                &[],
                background.replacement,
                true,
            );
        }
        if options.grayscale {
            apply_grayscale(&mut square);
        }
        square
    };
    encode(&render, &sizes, options, options.threads, notes)
}

/// The one-call form: analyzes and converts a source of either kind. A
/// `--find` is applied (its regions are used) rather than previewed.
pub fn convert(
    artwork: Artwork,
    options: &Options,
    notes: &dyn Notes,
) -> Result<Converted, String> {
    match artwork {
        Artwork::Raster(image) => {
            let source = image.into_rgba8();
            let analysis = analyze_raster(&source, options, notes)?;
            convert_raster(source, options, &analysis, notes)
        }
        Artwork::Vector(drawing) => convert_vector(&drawing, options, notes),
    }
}

/// Builds the file of the format asked for from the render function.
fn encode(
    render: &(dyn Fn(u32) -> RgbaImage + Sync),
    sizes: &[u32],
    options: &Options,
    threads: usize,
    notes: &dyn Notes,
) -> Result<Converted, String> {
    // With a color to flatten onto, every image is laid on it first.
    let flat = |size: u32| {
        let mut image = render(size);
        if let Some(color) = options.flatten {
            flatten_onto(&mut image, color);
        }
        image
    };
    let render = &flat;
    match options.format {
        Format::Icns => Ok(Converted {
            bytes: encode_icns(render, threads)?,
            sizes: icns_sizes(),
        }),
        Format::Png => {
            let size = sizes
                .first()
                .copied()
                .ok_or_else(|| crate::msg!("convert.png_needs_size", "A PNG needs a size."))?;
            if !(1..=MAX_PNG_SIZE).contains(&size) {
                return Err(crate::msg!(
                    "convert.png_size_range",
                    "A PNG can be 1 to {max} pixels wide; {size} is not.",
                    max = MAX_PNG_SIZE,
                    size = size
                ));
            }
            let mut png = std::io::Cursor::new(Vec::new());
            render(size)
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|e| {
                    crate::msg!(
                        "convert.png_encode_failed",
                        "Could not encode the {size}x{size} PNG: {e}",
                        size = size,
                        e = e
                    )
                })?;
            Ok(Converted {
                bytes: png.into_inner(),
                sizes: vec![size],
            })
        }
        Format::Ico => {
            let (bytes, written) = encode_ico(render, sizes, options.silent, threads, notes)?;
            Ok(Converted {
                bytes,
                sizes: written,
            })
        }
    }
}

/// Lays the picture on a solid color: every pixel becomes opaque, with the
/// picture's colors mixed with `color` in proportion to its transparency.
pub fn flatten_onto(image: &mut RgbaImage, color: [u8; 3]) {
    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            let over =
                u32::from(pixel[channel]) * alpha + u32::from(color[channel]) * (255 - alpha);
            pixel[channel] = ((over + 127) / 255) as u8;
        }
        pixel[3] = 255;
    }
}

/// Builds the .ico: one square icon per requested size, all PNG-encoded, all
/// together in one file's bytes - and the sizes that went into it.
pub fn encode_ico(
    render: &(dyn Fn(u32) -> RgbaImage + Sync),
    sizes: &[u32],
    silent: bool,
    threads: usize,
    notes: &dyn Notes,
) -> Result<(Vec<u8>, Vec<u32>), String> {
    // An IconDir collects all the resolutions that will be written
    // together into ONE .ico file at the end.
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    let mut written = Vec::with_capacity(sizes.len());

    // Sizes an .ico cannot hold are named and left out first - in one thread,
    // so the messages keep naming the right file.
    let valid: Vec<u32> = sizes
        .iter()
        .copied()
        .filter(|&size| {
            let fits = size != 0 && size <= MAX_ICO_SIZE;
            if !fits {
                warn(
                    silent,
                    crate::msg!(
                        "convert.size_skipped",
                        "{prefix}Skipping size {size} (valid range: 1-{max}).",
                        prefix = file_prefix(),
                        size = size,
                        max = MAX_ICO_SIZE
                    ),
                );
            }
            fits
        })
        .collect();

    // Scaling and PNG-encoding a size depends on nothing but the source, so
    // the sizes can be made at the same time; the entries are collected in
    // the order of the sizes either way.
    let entries = parallel_map(&valid, threads, |&size| {
        let square = render(size);
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
        ico::IconDirEntry::encode_as_png(&icon_image).map_err(|e| {
            crate::msg!(
                "convert.size_encode_failed",
                "Could not encode size {size}: {e}",
                size = size,
                e = e
            )
        })
    });

    for (&size, entry) in valid.iter().zip(entries) {
        let entry = entry?;
        note(notes, || {
            format!(
                "{}{size}x{size}: {} bytes",
                file_prefix(),
                entry.data().len()
            )
        });
        icon_dir.add_entry(entry);
        written.push(size);
    }

    // Write all the collected resolutions into one buffer.
    let mut buffer = Vec::new();
    icon_dir.write(&mut buffer).map_err(|e| {
        crate::msg!(
            "convert.ico_write_failed",
            "Error writing ICO file: {e}",
            e = e
        )
    })?;

    Ok((buffer, written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::FitMode;
    use image::Rgba;

    fn options() -> Options {
        Options {
            format: Format::Ico,
            sizes: Sizes::Fixed(vec![16, 32]),
            layout: Layout::default(),
            background: None,
            grayscale: false,
            flip_horizontal: false,
            flip_vertical: false,
            rotate: 0,
            crop: None,
            trim: false,
            threads: 1,
            flatten: None,
            silent: true,
        }
    }

    /// A green background with a red square in the middle.
    fn picture() -> RgbaImage {
        RgbaImage::from_fn(40, 40, |x, y| {
            if (10..30).contains(&x) && (10..30).contains(&y) {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 255])
            }
        })
    }

    fn sizes_in(bytes: &[u8]) -> Vec<u32> {
        ico::IconDir::read(std::io::Cursor::new(bytes))
            .unwrap()
            .entries()
            .iter()
            .map(|e| e.width())
            .collect()
    }

    fn pixel(bytes: &[u8], entry: usize, x: u32, y: u32) -> [u8; 4] {
        let dir = ico::IconDir::read(std::io::Cursor::new(bytes)).unwrap();
        dir.entries()[entry].decode().unwrap().rgba_data()
            [((y * dir.entries()[entry].width() + x) * 4) as usize..][..4]
            .try_into()
            .unwrap()
    }

    fn green_key() -> Background {
        Background {
            mode: BackgroundMode::ChromaKey,
            spec: "00FF00".to_string(),
            tolerance: 20,
            feather: 0,
            seeds: Vec::new(),
            replacement: None,
        }
    }

    #[test]
    fn a_picture_becomes_an_ico_with_the_requested_sizes() {
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options(),
            &NoNotes,
        )
        .unwrap();
        assert_eq!(result.sizes, vec![16, 32]);
        assert_eq!(sizes_in(&result.bytes), vec![16, 32]);
    }

    #[test]
    fn the_background_color_becomes_transparent() {
        let mut options = options();
        options.background = Some(green_key());
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        assert_eq!(pixel(&result.bytes, 1, 0, 0)[3], 0, "the corner is gone");
        assert_eq!(pixel(&result.bytes, 1, 16, 16)[3], 255, "the square stays");
    }

    #[test]
    fn auto_asks_for_no_more_than_the_source_has() {
        let mut options = options();
        options.sizes = Sizes::Auto(DEFAULT_SIZES.to_vec());
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        assert!(result.sizes.iter().all(|&s| s <= 64), "{:?}", result.sizes);
        assert!(result.sizes.contains(&16));
    }

    #[test]
    fn crop_and_trim_and_grayscale_apply_in_that_order() {
        let mut options = options();
        options.background = Some(green_key());
        options.trim = true;
        options.grayscale = true;
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        // The square fills the icon once the background is gone and cut off,
        // and it is gray.
        let [r, g, b, a] = pixel(&result.bytes, 1, 16, 16);
        assert_eq!(a, 255);
        assert!(r == g && g == b, "{r} {g} {b}");
        assert_eq!(
            pixel(&result.bytes, 1, 0, 0)[3],
            255,
            "trimmed to the square"
        );
    }

    #[test]
    fn a_single_png_can_be_asked_for_at_any_size_up_to_the_limit() {
        let mut options = options();
        options.format = Format::Png;
        options.sizes = Sizes::Fixed(vec![512]);
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        assert_eq!(result.sizes, vec![512]);
        let decoded = image::load_from_memory(&result.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (512, 512));

        options.sizes = Sizes::Fixed(vec![MAX_PNG_SIZE + 1]);
        let error = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap_err();
        assert!(error.contains("1 to 1024"), "{error}");
    }

    #[test]
    fn flattening_lays_the_icon_on_a_color() {
        let mut image = RgbaImage::new(3, 1);
        image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, Rgba([255, 0, 0, 0]));
        image.put_pixel(2, 0, Rgba([255, 0, 0, 128]));
        flatten_onto(&mut image, [0, 0, 255]);
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255], "opaque stays");
        assert_eq!(
            image.get_pixel(1, 0).0,
            [0, 0, 255, 255],
            "transparent becomes the color"
        );
        let mixed = image.get_pixel(2, 0).0;
        assert_eq!(mixed[3], 255);
        assert!(
            (120..=135).contains(&mixed[0]) && (120..=135).contains(&mixed[2]),
            "{mixed:?}"
        );

        let mut options = options();
        options.format = Format::Png;
        options.sizes = Sizes::Fixed(vec![16]);
        options.flatten = Some([255, 255, 255]);
        options.layout.padding = 40;
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        let decoded = image::load_from_memory(&result.bytes).unwrap().to_rgba8();
        assert!(
            decoded.pixels().all(|p| p[3] == 255),
            "nothing is transparent"
        );
        assert_eq!(
            decoded.get_pixel(0, 0).0,
            [255, 255, 255, 255],
            "the margin is the color"
        );
    }

    #[test]
    fn an_icns_has_its_own_sizes() {
        let mut options = options();
        options.format = Format::Icns;
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        assert!(result.bytes.starts_with(b"icns"));
        assert_eq!(result.sizes, icns_sizes());
    }

    #[test]
    fn sizes_an_ico_cannot_hold_are_left_out() {
        let mut options = options();
        options.sizes = Sizes::Fixed(vec![0, 16, 512]);
        let (result, warnings) = crate::diag::collect(|| {
            convert(
                Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
                &options,
                &NoNotes,
            )
        });
        assert_eq!(result.unwrap().sizes, vec![16]);
        assert!(warnings.iter().any(|w| w.contains("Skipping size 512")));
    }

    #[test]
    fn find_regions_are_found_and_used() {
        // A ring of the background color around a hole of it: the border
        // cannot reach the hole, --find does.
        let mut img = RgbaImage::from_pixel(60, 60, Rgba([0, 255, 0, 255]));
        for y in 0..60 {
            for x in 0..60 {
                let d = (x as i32 - 30).abs().max((y as i32 - 30).abs());
                if (15..25).contains(&d) {
                    img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
        let mut options = options();
        options.background = Some(Background {
            mode: BackgroundMode::Find { min_size: 10 },
            ..green_key()
        });
        let analysis = analyze_raster(&img, &options, &NoNotes).unwrap();
        assert_eq!(analysis.regions.len(), 1, "the enclosed hole");
        let result = convert_raster(img, &options, &analysis, &NoNotes).unwrap();
        assert_eq!(pixel(&result.bytes, 1, 16, 16)[3], 0, "the hole is cleared");
    }

    #[test]
    fn a_bad_color_is_named_with_its_option() {
        let mut options = options();
        options.background = Some(Background {
            spec: "nonsense".to_string(),
            ..green_key()
        });
        let error = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap_err();
        assert!(error.starts_with("Invalid --chroma-key value"), "{error}");
    }

    const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#00ff00"/><circle cx="10" cy="10" r="6" fill="#ff0000"/></svg>"##;

    #[test]
    fn an_svg_is_drawn_at_every_size() {
        let drawing = VectorImage::parse(SVG, "a.svg").unwrap();
        let mut options = options();
        options.sizes = Sizes::Fixed(vec![16, 64]);
        let result = convert_vector(&drawing, &options, &NoNotes).unwrap();
        assert_eq!(sizes_in(&result.bytes), vec![16, 64]);
        assert_eq!(pixel(&result.bytes, 1, 32, 32), [255, 0, 0, 255]);
    }

    // Without the `svg-text` feature (the web page) the text of a drawing is left out, and the conversion says so.
    // (With it, as in the command line, the text is drawn and there is nothing to say.)
    #[test]
    fn an_svg_with_text_gets_a_warning_where_text_cannot_be_drawn() {
        let with_text = String::from_utf8_lossy(SVG)
            .replace("</svg>", "<text x=\"2\" y=\"18\">Hi</text></svg>");
        let (plain, plain_warnings) = crate::diag::collect(|| {
            convert_vector(
                &VectorImage::parse(SVG, "a.svg").unwrap(),
                &options(),
                &NoNotes,
            )
        });
        plain.unwrap();
        assert!(plain_warnings.is_empty(), "{plain_warnings:?}");

        let drawing = VectorImage::parse(with_text.as_bytes(), "a.svg").unwrap();
        let (result, warnings) =
            crate::diag::collect(|| convert_vector(&drawing, &options(), &NoNotes));
        result.unwrap();
        let said = warnings.iter().any(|w| w.contains("has text"));
        assert_eq!(said, cfg!(not(feature = "svg-text")), "{warnings:?}");
    }

    #[test]
    fn an_svg_is_not_mirrored() {
        let drawing = VectorImage::parse(SVG, "a.svg").unwrap();
        let mut options = options();
        options.flip_horizontal = true;
        let error = convert_vector(&drawing, &options, &NoNotes).unwrap_err();
        assert!(error.contains("flip"), "{error}");
    }

    #[test]
    fn an_svg_background_can_be_removed_but_pixel_positions_are_refused() {
        let drawing = VectorImage::parse(SVG, "a.svg").unwrap();
        let mut options = options();
        options.background = Some(green_key());
        let result = convert_vector(&drawing, &options, &NoNotes).unwrap();
        assert_eq!(pixel(&result.bytes, 1, 0, 0)[3], 0);

        options.background = Some(Background {
            seeds: vec![(1, 1)],
            ..green_key()
        });
        assert!(
            convert_vector(&drawing, &options, &NoNotes)
                .unwrap_err()
                .starts_with("--seed does not apply to an SVG")
        );
        let mut cropped = self::options();
        cropped.crop = Some(CropRect {
            x: 0,
            y: 0,
            width: 5,
            height: 5,
        });
        assert!(
            check_vector_options(&cropped)
                .unwrap_err()
                .starts_with("--crop")
        );
    }

    #[test]
    fn padding_and_fit_reach_the_icon() {
        let mut options = options();
        options.layout = Layout {
            fit: FitMode::Contain,
            padding: 50,
            corner_radius: 0,
        };
        let result = convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options,
            &NoNotes,
        )
        .unwrap();
        assert_eq!(pixel(&result.bytes, 1, 1, 1)[3], 0, "a transparent margin");
        assert_eq!(pixel(&result.bytes, 1, 16, 16)[3], 255);
    }

    #[test]
    fn verbose_notes_are_only_made_when_they_are_wanted() {
        use std::cell::RefCell;
        struct Collect(RefCell<Vec<String>>);
        impl Notes for Collect {
            fn verbose(&self, line: &str) {
                self.0.borrow_mut().push(line.to_string());
            }
            fn verbose_enabled(&self) -> bool {
                true
            }
        }
        let notes = Collect(RefCell::new(Vec::new()));
        convert(
            Artwork::Raster(image::DynamicImage::ImageRgba8(picture())),
            &options(),
            &notes,
        )
        .unwrap();
        let lines = notes.0.into_inner();
        assert!(lines.iter().any(|l| l.starts_with("16x16: ")), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("32x32: ")), "{lines:?}");
    }
}
