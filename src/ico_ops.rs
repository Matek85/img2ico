// Everything that reads or writes EXISTING .ico files rather than
// converting a fresh source image: --merge, --inspect, --extract and
// --select. These four share a lot of plumbing (opening/parsing an .ico,
// the overwrite check, PNG re-encoding for consistent quality), which is
// why they live together in one module.

use crate::cli::RECOMMENDED_WINDOWS_SIZES;
use crate::gif::{count_gif_frames, is_gif};
use crate::icns::icns_sizes;
use crate::util::check_overwrite;
use image::RgbaImage;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Opens and parses an existing .ico file, with the one consistent error
/// message used by every mode that reads one (--merge, --inspect,
/// --extract, --select) - instead of repeating the same open+read+map_err
/// block nearly verbatim in all four.
fn read_icon_dir(path: &Path) -> Result<ico::IconDir, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Could not open '{}': {e}", path.display()))?;
    ico::IconDir::read(file)
        .map_err(|e| format!("Could not read '{}' as an ICO file: {e}", path.display()))
}

/// Merges the icon entries of two or more existing .ico files into one.
///
/// Each entry is decoded back to raw RGBA pixels and then re-encoded as
/// PNG - same reasoning as for the normal conversion path: this guarantees
/// full 32-bit color depth and a clean alpha channel for every size in the
/// output, instead of silently inheriting whatever (possibly lower-quality
/// BMP) encoding the source file happened to use.
///
/// If two source files contain the same size, the first occurrence wins
/// and later duplicates are skipped (with a warning): an .ico file isn't
/// meant to contain the same size twice, and most consumers would only
/// ever look at one of them anyway.
pub fn merge_icons(
    paths: &[PathBuf],
    output_path: &Path,
    force: bool,
    silent: bool,
) -> Result<(), String> {
    if paths.len() < 2 {
        return Err("Merge mode needs at least two input .ico files.".to_string());
    }
    check_overwrite(output_path, force)?;

    let mut merged = ico::IconDir::new(ico::ResourceType::Icon);
    // Keeps track of which (width, height) pairs are already in the
    // output, so we can detect and skip duplicates across source files.
    let mut seen_sizes: HashSet<(u32, u32)> = HashSet::new();

    for path in paths {
        let source = read_icon_dir(path)?;

        for entry in source.entries() {
            let size = (entry.width(), entry.height());
            if !seen_sizes.insert(size) {
                if !silent {
                    eprintln!(
                        "Skipping {}x{} from '{}': that size is already present in the merged output.",
                        size.0,
                        size.1,
                        path.display()
                    );
                }
                continue;
            }

            let image = entry.decode().map_err(|e| {
                format!(
                    "Could not decode the {}x{} icon in '{}': {e}",
                    size.0,
                    size.1,
                    path.display()
                )
            })?;
            let new_entry = ico::IconDirEntry::encode_as_png(&image)
                .map_err(|e| format!("Could not re-encode the {}x{} icon: {e}", size.0, size.1))?;
            merged.add_entry(new_entry);
        }
    }

    if merged.entries().is_empty() {
        return Err("No icons found to merge - the resulting file would be empty.".to_string());
    }

    let out_file = std::fs::File::create(output_path)
        .map_err(|e| format!("Could not create output file: {e}"))?;
    merged
        .write(out_file)
        .map_err(|e| format!("Error writing merged ICO file: {e}"))?;

    say!(
        "Done: '{}' created from {} source file(s), containing {} icon(s) total.",
        output_path.display(),
        paths.len(),
        merged.entries().len()
    );

    Ok(())
}

/// Prints a human-readable report about one or more files: for an
/// existing .ico, which sizes it contains (with their zero-based index,
/// so you know what to pass to --select --index), at what color depth,
/// whether each entry is PNG- or (legacy) BMP-encoded, and a couple of
/// sanity warnings. For a regular source image (PNG/JPG/BMP/GIF - the
/// same formats the normal conversion mode accepts), prints its
/// resolution instead, along with which standard icon sizes it can
/// produce without upscaling versus which would come out soft/blurry.
/// Doesn't modify or create anything either way.
pub fn inspect_icons(paths: &[PathBuf]) -> Result<(), String> {
    for (file_index, path) in paths.iter().enumerate() {
        if file_index > 0 {
            println!();
        }

        match read_icon_dir(path) {
            Ok(dir) => inspect_ico_file(path, &dir),
            Err(ico_error) => {
                // Not a valid .ico - but --inspect is also happy to look
                // at a plain source image instead, so try that before
                // giving up. This mirrors what --output-format's
                // no-flags-given default does for macOS vs everyone else:
                // meeting the user where they already are, rather than
                // making them remember which mode to ask for.
                //
                // The result is bound to a variable first (instead of being
                // matched on directly) so it is dropped at the same point
                // in every Rust edition - see the "tail-expr-drop-order"
                // change in the 2024 edition guide.
                let opened = crate::source::open_image(path);
                match opened {
                    Ok(img) => inspect_source_image(path, &img),
                    Err(image_error) => {
                        return Err(neither_ico_nor_image(path, &ico_error, &image_error));
                    }
                }
            }
        }
    }

    Ok(())
}

/// The error for a file that --inspect could read neither as an .ico nor as
/// an image.
fn neither_ico_nor_image(
    path: &Path,
    ico_error: &impl std::fmt::Display,
    image_error: &impl std::fmt::Display,
) -> String {
    format!(
        "'{}' is neither a readable .ico file ({ico_error}) nor a readable image ({image_error}).",
        path.display()
    )
}

/// The common Windows icon sizes that `present` lacks.
fn missing_windows_sizes(present: &HashSet<u32>) -> Vec<u32> {
    RECOMMENDED_WINDOWS_SIZES
        .iter()
        .copied()
        .filter(|size| !present.contains(size))
        .collect()
}

/// Splits `sizes` into those a source whose longer edge is `native_max` pixels
/// covers natively and those that would need upscaling.
fn split_by_coverage(sizes: &[u32], native_max: u32) -> (Vec<u32>, Vec<u32>) {
    sizes.iter().copied().partition(|&size| size <= native_max)
}

/// `--inspect --json`: the same information as the text report, as one JSON
/// array with an entry per file (always an array, even for one file), so a
/// script can rely on the shape. Standard output holds the JSON and nothing
/// else; if any file cannot be read, nothing is printed and the error says why.
pub fn inspect_icons_json(paths: &[PathBuf]) -> Result<(), String> {
    let mut reports = Vec::with_capacity(paths.len());
    for path in paths {
        let report = match read_icon_dir(path) {
            Ok(dir) => ico_report_json(path, &dir),
            Err(ico_error) => {
                let opened = crate::source::open_image(path);
                match opened {
                    Ok(img) => image_report_json(path, &img),
                    Err(image_error) => {
                        return Err(neither_ico_nor_image(path, &ico_error, &image_error));
                    }
                }
            }
        };
        reports.push(report);
    }
    let text = serde_json::to_string_pretty(&reports)
        .map_err(|e| format!("Could not build the JSON report: {e}"))?;
    println!("{text}");
    Ok(())
}

/// The JSON form of `inspect_ico_file`'s report.
fn ico_report_json(path: &Path, dir: &ico::IconDir) -> serde_json::Value {
    let mut present_sizes: HashSet<u32> = HashSet::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut entries = Vec::with_capacity(dir.entries().len());
    for (index, entry) in dir.entries().iter().enumerate() {
        let (w, h) = (entry.width(), entry.height());
        let bpp = entry.bits_per_pixel();
        present_sizes.insert(w);
        if !entry.is_png() {
            warnings.push(format!(
                "[{index}] legacy BMP format - only a 1-bit transparency mask, no smooth alpha edges"
            ));
            if bpp < 32 {
                warnings.push(format!(
                    "[{index}] reduced color depth ({bpp}bpp instead of 32bpp) - likely visible color banding"
                ));
            }
        }
        entries.push(serde_json::json!({
            "index": index,
            "width": w,
            "height": h,
            "bits_per_pixel": bpp,
            "format": if entry.is_png() { "png" } else { "bmp" },
            "bytes": entry.data().len(),
        }));
    }
    serde_json::json!({
        "path": path.display().to_string(),
        "kind": "ico",
        "entries": entries,
        "missing_windows_sizes": missing_windows_sizes(&present_sizes),
        "warnings": warnings,
    })
}

/// The JSON form of `inspect_source_image`'s report.
fn image_report_json(path: &Path, img: &image::DynamicImage) -> serde_json::Value {
    let native_max = img.width().max(img.height());
    let frames = match is_gif(path) {
        Ok(true) => count_gif_frames(path).ok(),
        _ => None,
    };
    let coverage = |sizes: &[u32]| {
        let (native, upscaled) = split_by_coverage(sizes, native_max);
        serde_json::json!({ "native": native, "upscaled": upscaled })
    };
    serde_json::json!({
        "path": path.display().to_string(),
        "kind": "image",
        "width": img.width(),
        "height": img.height(),
        "frames": frames,
        "windows": coverage(&RECOMMENDED_WINDOWS_SIZES),
        "macos": coverage(&icns_sizes()),
    })
}

/// The .ico-specific half of --inspect's report - see inspect_icons()
/// above for the full picture, including when this isn't the branch that
/// runs.
fn inspect_ico_file(path: &Path, dir: &ico::IconDir) {
    println!("{}:", path.display());
    if dir.entries().is_empty() {
        println!("  (no icons found)");
        return;
    }

    let mut present_sizes: HashSet<u32> = HashSet::new();
    for (index, entry) in dir.entries().iter().enumerate() {
        let (w, h) = (entry.width(), entry.height());
        present_sizes.insert(w);
        let format = if entry.is_png() { "PNG" } else { "BMP" };
        let bpp = entry.bits_per_pixel();
        println!(
            "  [{index}] {w:>4}x{h:<4}  {bpp:>2}bpp  {format}  {} bytes",
            entry.data().len()
        );
        if !entry.is_png() {
            println!(
                "      warning: legacy BMP format - only a 1-bit transparency mask, no smooth alpha edges"
            );
            if bpp < 32 {
                println!(
                    "      warning: reduced color depth ({bpp}bpp instead of 32bpp) - likely visible color banding"
                );
            }
        }
    }

    let missing = missing_windows_sizes(&present_sizes);
    if !missing.is_empty() {
        println!(
            "  note: missing common Windows sizes (Windows will have to scale a nearby size for these): {missing:?}"
        );
    }
}

/// The source-image half of --inspect's report: instead of listing
/// existing icon entries (there are none yet - this isn't an .ico file),
/// reports the image's resolution and, for both the Windows and macOS
/// standard size sets, which sizes it could produce natively versus which
/// would need upscaling. Upscaling can't add detail that isn't there, so
/// this is the same distinction demonstrated visually earlier in this
/// project's history (Lanczos3 vs. nearest-neighbor on a tiny source
/// image) - here as a quick heads-up before you even run the conversion,
/// not an error, since upscaling still produces a valid (if softer) icon.
fn inspect_source_image(path: &Path, img: &image::DynamicImage) {
    let (w, h) = (img.width(), img.height());
    let native_max = w.max(h);

    println!("{} (source image, {w}x{h}):", path.display());

    // GIF-specific: report the frame count, since img2ico (like the
    // normal, non-animation-aware image::open() path it's built on) only
    // ever uses ONE frame from an animated GIF, not the whole animation.
    // A read/decode error here is treated as "couldn't determine this",
    // not a reason to fail the rest of the report - the resolution and
    // size-coverage information below is still fully valid either way.
    if let Ok(true) = is_gif(path) {
        match count_gif_frames(path) {
            Ok(count) => println!(
                "  frames: {count} (img2ico uses frame 1 by default; pass --gif-frame N to pick another)"
            ),
            Err(e) => eprintln!("  (could not determine the frame count: {e})"),
        }
    }

    let report_one = |label: &str, sizes: &[u32]| {
        let (native, upscaled) = split_by_coverage(sizes, native_max);
        if !native.is_empty() {
            println!("  {label} sizes this image covers natively: {native:?}");
        }
        if !upscaled.is_empty() {
            println!(
                "  {label} sizes that would need upscaling (may look soft/blurry): {upscaled:?}"
            );
        }
    };

    report_one("Windows", &RECOMMENDED_WINDOWS_SIZES);
    report_one("macOS (.icns)", &icns_sizes());

    if native_max < 256 {
        println!(
            "  tip: for consistently sharp icons at every common size, a source of at least 256x256 (1024x1024 if you also need .icns) is recommended."
        );
    }
}

/// The file name for the `width`x`height` icon of `stem` -
/// "<stem>_<width>x<height>.<extension>" - made unique among the names
/// already `taken` by appending "_2", "_3", ... if needed. Two entries of
/// the same size only happen in a broken (or hand-built) .ico file, but
/// without this one would silently overwrite the other.
fn unique_file_name(
    taken: &[String],
    stem: &str,
    width: u32,
    height: u32,
    extension: &str,
) -> String {
    let mut name = format!("{stem}_{width}x{height}.{extension}");
    let mut suffix = 2;
    while taken.contains(&name) {
        name = format!("{stem}_{width}x{height}_{suffix}.{extension}");
        suffix += 1;
    }
    name
}

/// Extracts every icon size out of an existing .ico file and saves each
/// one as a separate PNG file into a target directory.
pub fn extract_icons(input: &Path, output_dir: Option<&Path>, force: bool) -> Result<(), String> {
    let dir = read_icon_dir(input)?;

    if dir.entries().is_empty() {
        return Err(format!(
            "'{}' contains no icons to extract.",
            input.display()
        ));
    }

    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("icon");

    // Target directory: either given explicitly, or "<stem>_extracted"
    // created right next to the input file.
    let target_dir = match output_dir {
        Some(dir) => dir.to_path_buf(),
        None => input
            .parent()
            .unwrap_or(Path::new(""))
            .join(format!("{stem}_extracted")),
    };
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "Could not create output directory '{}': {e}",
            target_dir.display()
        )
    })?;

    // First pass: figure out the (deduplicated) output filename for every
    // entry, and make sure NONE of them would silently overwrite an
    // existing file - before we write anything at all. Without this
    // separate pass, a conflict discovered halfway through would leave a
    // half-extracted directory behind (some sizes written, some not).
    //
    // Guards against the (rare, but possible) case of a broken .ico file
    // that lists the same size more than once - without this we'd
    // silently overwrite one extracted file with another.
    let mut names: Vec<String> = Vec::with_capacity(dir.entries().len());
    for entry in dir.entries() {
        let name = unique_file_name(&names, stem, entry.width(), entry.height(), "png");
        check_overwrite(&target_dir.join(&name), force)?;
        names.push(name);
    }

    // Second pass: now that we know none of the target files will be
    // silently clobbered, actually decode and write every one of them.
    let mut extracted_count = 0u32;
    for (entry, name) in dir.entries().iter().zip(names.iter()) {
        let (w, h) = (entry.width(), entry.height());
        let image = entry.decode().map_err(|e| {
            format!(
                "Could not decode the {w}x{h} icon in '{}': {e}",
                input.display()
            )
        })?;

        let rgba = RgbaImage::from_raw(w, h, image.into_rgba_data())
            .ok_or_else(|| format!("Unexpected pixel data size for the {w}x{h} icon"))?;

        let out_path = target_dir.join(name);
        rgba.save(&out_path)
            .map_err(|e| format!("Could not save '{}': {e}", out_path.display()))?;
        extracted_count += 1;
    }

    say!(
        "Done: extracted {extracted_count} icon(s) from '{}' into '{}'.",
        input.display(),
        target_dir.display()
    );

    Ok(())
}

/// Parses a "--index" value ("0,2,4") into a list of zero-based indices.
/// Falls back to just index 0 if the argument was omitted entirely - the
/// agreed-on default for --select when the user doesn't care which exact
/// icon they get.
pub fn parse_indices(input: Option<&str>) -> Result<Vec<usize>, String> {
    let Some(input) = input else {
        return Ok(vec![0]);
    };
    let indices: Vec<usize> = input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<usize>()
                .map_err(|_| format!("Invalid index: '{s}' (must be a non-negative whole number)"))
        })
        .collect::<Result<Vec<usize>, String>>()?;

    if indices.is_empty() {
        return Err(
            "At least one index must be given (or omit --index to use the default: 0).".to_string(),
        );
    }
    Ok(indices)
}

/// Pulls one or more specific icon(s) out of an existing .ico file BY
/// INDEX and re-exports them as standalone .ico file(s) - the ICO-to-ICO
/// counterpart of --extract (which always exports every size as PNG).
///
/// Every selected entry is decoded and re-encoded as PNG, for the same
/// reason as everywhere else in this program: guarantees full 32-bit
/// color depth and a clean alpha channel, regardless of how the source
/// file happened to encode it.
///
/// Returns the single output file path if exactly one file was written
/// (either because only one index was selected, or --combine was used),
/// or None if a whole directory of separate files was written instead -
/// the caller uses this to know whether --delete-source's "don't delete
/// the file we just wrote as output" safety check even applies here.
pub fn select_icons(
    input: &Path,
    indices: &[usize],
    combine: bool,
    output: Option<&Path>,
    force: bool,
) -> Result<Option<PathBuf>, String> {
    let dir = read_icon_dir(input)?;
    let entries = dir.entries();

    if entries.is_empty() {
        return Err(format!(
            "'{}' contains no icons to select from.",
            input.display()
        ));
    }

    // Validate every requested index up front, with a message that tells
    // the user the valid range instead of a generic "out of bounds" - and
    // points them at --inspect, which is how they'd find a valid index in
    // the first place.
    for &i in indices {
        if i >= entries.len() {
            return Err(format!(
                "Index {i} is out of range for '{}' - it contains {} icon(s), so valid indices are 0..{}. Use --inspect to see them.",
                input.display(),
                entries.len(),
                entries.len() - 1
            ));
        }
    }

    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("icon");
    let parent = input.parent().unwrap_or(Path::new(""));

    if combine || indices.len() == 1 {
        // A single output .ico file: either because the user only picked
        // one icon (no point creating a whole directory for that), or
        // because --combine was explicitly given for multiple.
        let output_path = match (output, indices) {
            (Some(path), _) => path.to_path_buf(),
            (None, &[only_index]) => {
                let entry = &entries[only_index];
                parent.join(format!("{stem}_{}x{}.ico", entry.width(), entry.height()))
            }
            (None, _) => parent.join(format!("{stem}_selected.ico")),
        };
        select_into_one_file(input, entries, indices, &output_path, force)?;
        Ok(Some(output_path))
    } else {
        // Default for multiple indices: one separate .ico file per
        // selected icon, written into a directory - same shape as
        // --extract, just .ico output instead of .png.
        let target_dir = match output {
            Some(path) => path.to_path_buf(),
            None => parent.join(format!("{stem}_selected")),
        };
        select_into_directory(input, entries, indices, &target_dir, stem, force)?;
        Ok(None)
    }
}

/// Decodes the icon at `index` and re-encodes it as PNG, so it gets the
/// same full color depth and clean alpha channel as everything else this
/// program writes, regardless of how the source file encoded it.
fn reencode_as_png(
    entry: &ico::IconDirEntry,
    index: usize,
    input: &Path,
) -> Result<ico::IconDirEntry, String> {
    let image = entry.decode().map_err(|e| {
        format!(
            "Could not decode icon at index {index} in '{}': {e}",
            input.display()
        )
    })?;
    ico::IconDirEntry::encode_as_png(&image)
        .map_err(|e| format!("Could not re-encode icon at index {index}: {e}"))
}

/// Writes the selected icons together into ONE .ico file.
fn select_into_one_file(
    input: &Path,
    entries: &[ico::IconDirEntry],
    indices: &[usize],
    output_path: &Path,
    force: bool,
) -> Result<(), String> {
    check_overwrite(output_path, force)?;

    let mut out_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &i in indices {
        out_dir.add_entry(reencode_as_png(&entries[i], i, input)?);
    }

    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Could not create output file: {e}"))?;
    out_dir
        .write(file)
        .map_err(|e| format!("Error writing ICO file: {e}"))?;

    say!(
        "Done: '{}' created with {} icon(s) selected from '{}'.",
        output_path.display(),
        indices.len(),
        input.display()
    );
    Ok(())
}

/// Writes every selected icon as its own .ico file into `target_dir`.
fn select_into_directory(
    input: &Path,
    entries: &[ico::IconDirEntry],
    indices: &[usize],
    target_dir: &Path,
    stem: &str,
    force: bool,
) -> Result<(), String> {
    std::fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Could not create output directory '{}': {e}",
            target_dir.display()
        )
    })?;

    // First pass: compute every target filename and check all of them
    // for --force conflicts before writing anything at all - same
    // reasoning as --extract, so a conflict never leaves a
    // half-written directory behind.
    let mut names: Vec<String> = Vec::with_capacity(indices.len());
    for &i in indices {
        let entry = &entries[i];
        let name = unique_file_name(&names, stem, entry.width(), entry.height(), "ico");
        check_overwrite(&target_dir.join(&name), force)?;
        names.push(name);
    }

    // Second pass: now actually decode and write each one.
    for (&i, name) in indices.iter().zip(&names) {
        let mut single = ico::IconDir::new(ico::ResourceType::Icon);
        single.add_entry(reencode_as_png(&entries[i], i, input)?);

        let out_path = target_dir.join(name);
        let out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("Could not save '{}': {e}", out_path.display()))?;
        single
            .write(out_file)
            .map_err(|e| format!("Error writing '{}': {e}", out_path.display()))?;
    }

    say!(
        "Done: {} icon(s) selected from '{}' into '{}'.",
        indices.len(),
        input.display(),
        target_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    /// Writes an .ico with one solid-color, PNG-encoded square entry per
    /// `(size, color)` pair, in the given order.
    fn make_ico(path: &Path, entries: &[(u32, [u8; 3])]) {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        for &(size, color) in entries {
            let mut data = Vec::with_capacity((size * size * 4) as usize);
            for _ in 0..size * size {
                data.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
            let image = ico::IconImage::from_rgba_data(size, size, data);
            dir.add_entry(ico::IconDirEntry::encode_as_png(&image).unwrap());
        }
        dir.write(std::fs::File::create(path).unwrap()).unwrap();
    }

    fn sizes_of(path: &Path) -> Vec<u32> {
        read_icon_dir(path)
            .unwrap()
            .entries()
            .iter()
            .map(ico::IconDirEntry::width)
            .collect()
    }

    /// The first pixel's RGB of the entry at `index`.
    fn first_pixel_of(path: &Path, index: usize) -> [u8; 3] {
        let dir = read_icon_dir(path).unwrap();
        let image = dir.entries()[index].decode().unwrap();
        let data = image.rgba_data();
        [data[0], data[1], data[2]]
    }

    fn file_names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    // --- unique_file_name --------------------------------------------------------

    #[test]
    fn a_free_name_is_used_as_is() {
        assert_eq!(
            unique_file_name(&[], "logo", 16, 16, "png"),
            "logo_16x16.png"
        );
        let taken = vec!["logo_32x32.png".to_string()];
        assert_eq!(
            unique_file_name(&taken, "logo", 16, 16, "png"),
            "logo_16x16.png"
        );
    }

    #[test]
    fn taken_names_get_an_increasing_suffix() {
        let mut taken = Vec::new();
        for expected in [
            "a_16x16.ico",
            "a_16x16_2.ico",
            "a_16x16_3.ico",
            "a_16x16_4.ico",
        ] {
            let name = unique_file_name(&taken, "a", 16, 16, "ico");
            assert_eq!(name, expected);
            taken.push(name);
        }
    }

    #[test]
    fn the_extension_is_part_of_the_name_but_not_of_the_uniqueness_of_other_sizes() {
        let taken = vec!["a_16x16.ico".to_string()];
        assert_eq!(unique_file_name(&taken, "a", 16, 16, "png"), "a_16x16.png");
        assert_eq!(unique_file_name(&taken, "a", 32, 32, "ico"), "a_32x32.ico");
    }

    // --- parse_indices -----------------------------------------------------------

    #[test]
    fn missing_index_argument_defaults_to_the_first_icon() {
        assert_eq!(parse_indices(None), Ok(vec![0]));
    }

    #[test]
    fn indices_are_parsed_in_the_given_order() {
        assert_eq!(parse_indices(Some("0,2,4")), Ok(vec![0, 2, 4]));
        assert_eq!(parse_indices(Some("3,1")), Ok(vec![3, 1]));
    }

    #[test]
    fn indices_tolerate_whitespace_and_empty_items() {
        assert_eq!(parse_indices(Some(" 1 , 3 ")), Ok(vec![1, 3]));
        assert_eq!(parse_indices(Some("1,,2,")), Ok(vec![1, 2]));
    }

    #[test]
    fn indices_may_repeat() {
        assert_eq!(parse_indices(Some("1,1")), Ok(vec![1, 1]));
    }

    #[test]
    fn an_index_list_without_any_number_is_an_error() {
        assert!(parse_indices(Some("")).is_err());
        assert!(parse_indices(Some(" , ")).is_err());
    }

    #[test]
    fn invalid_indices_are_errors_naming_the_bad_value() {
        for bad in ["a", "-1", "1.5", "1,x"] {
            assert!(parse_indices(Some(bad)).is_err(), "{bad} should fail");
        }
        assert!(parse_indices(Some("2,oops")).unwrap_err().contains("oops"));
    }

    // --- read_icon_dir -----------------------------------------------------------

    #[test]
    fn reading_a_missing_ico_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_icon_dir(&dir.path().join("none.ico"))
            .map(|_| ())
            .unwrap_err();
        assert!(err.contains("Could not open"), "unexpected message: {err}");
    }

    #[test]
    fn reading_a_non_ico_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.ico");
        std::fs::write(&path, b"definitely not an icon").unwrap();
        let err = read_icon_dir(&path).map(|_| ()).unwrap_err();
        assert!(err.contains("as an ICO file"), "unexpected message: {err}");
    }

    // --- merge_icons -------------------------------------------------------------

    #[test]
    fn merge_combines_distinct_sizes_from_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.ico"),
            dir.path().join("b.ico"),
            dir.path().join("out.ico"),
        );
        make_ico(&a, &[(16, RED)]);
        make_ico(&b, &[(32, GREEN), (64, BLUE)]);
        merge_icons(&[a, b], &out, false, true).unwrap();
        assert_eq!(sizes_of(&out), vec![16, 32, 64]);
    }

    #[test]
    fn merge_keeps_the_first_occurrence_of_a_duplicate_size() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.ico"),
            dir.path().join("b.ico"),
            dir.path().join("out.ico"),
        );
        make_ico(&a, &[(16, RED), (32, RED)]);
        make_ico(&b, &[(32, GREEN), (64, GREEN)]);
        merge_icons(&[a, b], &out, false, true).unwrap();
        assert_eq!(sizes_of(&out), vec![16, 32, 64]);
        assert_eq!(
            first_pixel_of(&out, 1),
            RED,
            "the 32px entry must come from the first file"
        );
        assert_eq!(first_pixel_of(&out, 2), GREEN);
    }

    #[test]
    fn merge_needs_at_least_two_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.ico");
        make_ico(&a, &[(16, RED)]);
        let err = merge_icons(&[a], &dir.path().join("out.ico"), false, true).unwrap_err();
        assert!(err.contains("at least two"), "unexpected message: {err}");
    }

    #[test]
    fn merge_refuses_to_overwrite_without_force_and_allows_it_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.ico"),
            dir.path().join("b.ico"),
            dir.path().join("out.ico"),
        );
        make_ico(&a, &[(16, RED)]);
        make_ico(&b, &[(32, GREEN)]);
        std::fs::write(&out, b"precious").unwrap();

        let err = merge_icons(&[a.clone(), b.clone()], &out, false, true).unwrap_err();
        assert!(err.contains("--force"));
        assert_eq!(
            std::fs::read(&out).unwrap(),
            b"precious",
            "the file must be untouched"
        );

        merge_icons(&[a, b], &out, true, true).unwrap();
        assert_eq!(sizes_of(&out), vec![16, 32]);
    }

    #[test]
    fn merge_fails_cleanly_on_an_unreadable_input() {
        let dir = tempfile::tempdir().unwrap();
        let (a, out) = (dir.path().join("a.ico"), dir.path().join("out.ico"));
        make_ico(&a, &[(16, RED)]);
        let missing = dir.path().join("missing.ico");
        assert!(merge_icons(&[a, missing], &out, false, true).is_err());
        assert!(!out.exists(), "no half-written output may be left behind");
    }

    #[test]
    fn merged_entries_are_always_png_encoded() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.ico"),
            dir.path().join("b.ico"),
            dir.path().join("out.ico"),
        );
        make_ico(&a, &[(16, RED)]);
        make_ico(&b, &[(32, GREEN)]);
        merge_icons(&[a, b], &out, false, true).unwrap();
        assert!(
            read_icon_dir(&out)
                .unwrap()
                .entries()
                .iter()
                .all(ico::IconDirEntry::is_png)
        );
    }

    #[test]
    fn merging_files_without_any_icons_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.ico"),
            dir.path().join("b.ico"),
            dir.path().join("out.ico"),
        );
        make_ico(&a, &[]);
        make_ico(&b, &[]);
        let err = merge_icons(&[a, b], &out, false, true).unwrap_err();
        assert!(err.contains("No icons found"), "unexpected message: {err}");
    }

    // --- inspect_icons (report goes to stdout; here we check success/failure) ----

    #[test]
    fn inspecting_an_ico_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ico");
        make_ico(&path, &[(16, RED), (256, GREEN)]);
        assert_eq!(inspect_icons(&[path]), Ok(()));
    }

    #[test]
    fn inspecting_an_empty_ico_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.ico");
        make_ico(&path, &[]);
        assert_eq!(inspect_icons(&[path]), Ok(()));
    }

    #[test]
    fn inspecting_a_plain_image_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        image::RgbaImage::from_pixel(10, 10, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        assert_eq!(inspect_icons(&[path]), Ok(()));
    }

    #[test]
    fn inspecting_several_files_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.ico"), dir.path().join("b.png"));
        make_ico(&a, &[(32, RED)]);
        image::RgbaImage::from_pixel(300, 300, image::Rgba([1, 2, 3, 255]))
            .save(&b)
            .unwrap();
        assert_eq!(inspect_icons(&[a, b]), Ok(()));
    }

    #[test]
    fn inspecting_something_that_is_neither_ico_nor_image_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk.bin");
        std::fs::write(&path, b"junk").unwrap();
        let err = inspect_icons(&[path]).unwrap_err();
        assert!(
            err.contains("neither a readable .ico"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn inspecting_a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(inspect_icons(&[dir.path().join("none.ico")]).is_err());
    }

    // --- extract_icons -----------------------------------------------------------

    #[test]
    fn extract_writes_one_png_per_size_with_the_right_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let out = dir.path().join("out");
        make_ico(&input, &[(16, RED), (32, GREEN), (48, BLUE)]);
        extract_icons(&input, Some(&out), false).unwrap();

        assert_eq!(
            file_names_in(&out),
            vec!["logo_16x16.png", "logo_32x32.png", "logo_48x48.png"]
        );
        let png = image::open(out.join("logo_32x32.png")).unwrap().to_rgba8();
        assert_eq!(png.dimensions(), (32, 32));
        assert_eq!(png.get_pixel(0, 0).0, [0, 255, 0, 255]);
    }

    #[test]
    fn extract_defaults_to_a_directory_next_to_the_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        make_ico(&input, &[(16, RED)]);
        extract_icons(&input, None, false).unwrap();
        assert!(
            dir.path()
                .join("logo_extracted")
                .join("logo_16x16.png")
                .is_file()
        );
    }

    #[test]
    fn extract_gives_duplicate_sizes_distinct_names() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("dup.ico");
        let out = dir.path().join("out");
        make_ico(&input, &[(16, RED), (16, GREEN), (16, BLUE)]);
        extract_icons(&input, Some(&out), false).unwrap();
        assert_eq!(
            file_names_in(&out),
            vec!["dup_16x16.png", "dup_16x16_2.png", "dup_16x16_3.png"]
        );
    }

    #[test]
    fn extract_writes_nothing_if_any_target_file_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let out = dir.path().join("out");
        make_ico(&input, &[(16, RED), (32, GREEN)]);
        std::fs::create_dir(&out).unwrap();
        std::fs::write(out.join("logo_32x32.png"), b"precious").unwrap();

        let err = extract_icons(&input, Some(&out), false).unwrap_err();
        assert!(err.contains("--force"));
        assert_eq!(
            file_names_in(&out),
            vec!["logo_32x32.png"],
            "no half-extracted directory"
        );
        assert_eq!(
            std::fs::read(out.join("logo_32x32.png")).unwrap(),
            b"precious"
        );
    }

    #[test]
    fn extract_overwrites_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let out = dir.path().join("out");
        make_ico(&input, &[(16, RED)]);
        std::fs::create_dir(&out).unwrap();
        std::fs::write(out.join("logo_16x16.png"), b"old").unwrap();
        extract_icons(&input, Some(&out), true).unwrap();
        assert!(image::open(out.join("logo_16x16.png")).is_ok());
    }

    #[test]
    fn extract_from_an_empty_ico_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("empty.ico");
        make_ico(&input, &[]);
        let err = extract_icons(&input, None, false).unwrap_err();
        assert!(
            err.contains("no icons to extract"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn extract_from_a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(extract_icons(&dir.path().join("none.ico"), None, false).is_err());
    }

    // --- select_icons ------------------------------------------------------------

    #[test]
    fn selecting_one_icon_writes_a_single_ico_named_after_its_size() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        make_ico(&input, &[(16, RED), (32, GREEN), (48, BLUE)]);

        let written = select_icons(&input, &[1], false, None, false).unwrap();
        let expected = dir.path().join("logo_32x32.ico");
        assert_eq!(written, Some(expected.clone()));
        assert_eq!(sizes_of(&expected), vec![32]);
        assert_eq!(first_pixel_of(&expected, 0), GREEN);
    }

    #[test]
    fn selecting_with_an_explicit_output_path_uses_it() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let out = dir.path().join("chosen.ico");
        make_ico(&input, &[(16, RED), (32, GREEN)]);
        let written = select_icons(&input, &[0], false, Some(&out), false).unwrap();
        assert_eq!(written, Some(out.clone()));
        assert_eq!(sizes_of(&out), vec![16]);
    }

    #[test]
    fn combine_bundles_the_selection_into_one_file_in_the_given_order() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        make_ico(&input, &[(16, RED), (32, GREEN), (48, BLUE)]);

        let written = select_icons(&input, &[2, 0], true, None, false).unwrap();
        let expected = dir.path().join("logo_selected.ico");
        assert_eq!(written, Some(expected.clone()));
        assert_eq!(sizes_of(&expected), vec![48, 16]);
    }

    #[test]
    fn several_indices_without_combine_write_one_file_each_into_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        make_ico(&input, &[(16, RED), (32, GREEN), (48, BLUE)]);

        let written = select_icons(&input, &[0, 2], false, None, false).unwrap();
        assert_eq!(written, None, "a directory was written, not a single file");
        let target = dir.path().join("logo_selected");
        assert_eq!(
            file_names_in(&target),
            vec!["logo_16x16.ico", "logo_48x48.ico"]
        );
        assert_eq!(sizes_of(&target.join("logo_48x48.ico")), vec![48]);
    }

    #[test]
    fn selecting_the_same_size_twice_gives_distinct_file_names() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("dup.ico");
        make_ico(&input, &[(16, RED), (16, GREEN)]);
        select_icons(&input, &[0, 1], false, None, false).unwrap();
        assert_eq!(
            file_names_in(&dir.path().join("dup_selected")),
            vec!["dup_16x16.ico", "dup_16x16_2.ico"]
        );
    }

    #[test]
    fn an_out_of_range_index_reports_the_valid_range() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        make_ico(&input, &[(16, RED), (32, GREEN)]);
        let err = select_icons(&input, &[0, 2], false, None, false).unwrap_err();
        assert!(
            err.contains("Index 2 is out of range"),
            "unexpected message: {err}"
        );
        assert!(err.contains("0..1"), "unexpected message: {err}");
        assert!(
            !dir.path().join("logo_selected").exists(),
            "nothing may be written on error"
        );
    }

    #[test]
    fn selecting_from_an_empty_ico_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("empty.ico");
        make_ico(&input, &[]);
        let err = select_icons(&input, &[0], false, None, false).unwrap_err();
        assert!(
            err.contains("no icons to select from"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn select_refuses_to_overwrite_a_single_output_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let out = dir.path().join("out.ico");
        make_ico(&input, &[(16, RED)]);
        std::fs::write(&out, b"precious").unwrap();

        assert!(select_icons(&input, &[0], false, Some(&out), false).is_err());
        assert_eq!(std::fs::read(&out).unwrap(), b"precious");
        assert!(select_icons(&input, &[0], false, Some(&out), true).is_ok());
    }

    #[test]
    fn select_writes_nothing_if_any_target_in_the_directory_exists() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.ico");
        let target = dir.path().join("logo_selected");
        make_ico(&input, &[(16, RED), (32, GREEN)]);
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("logo_32x32.ico"), b"precious").unwrap();

        assert!(select_icons(&input, &[0, 1], false, None, false).is_err());
        assert_eq!(file_names_in(&target), vec!["logo_32x32.ico"]);
    }
}
