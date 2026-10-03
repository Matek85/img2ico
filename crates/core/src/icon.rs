// Working with .ico files that already exist: reading the pictures inside
// them back out as pixels (the page's pixel inspector), describing them,
// extracting them as PNG, selecting some of them and merging several files
// into one. Every image that is written out again is re-encoded as PNG - full
// 32-bit color and a clean alpha channel, whatever the source file used.

use image::RgbaImage;
use std::collections::HashSet;
use std::io::Cursor;

/// The icon sizes Microsoft recommends, so Windows never has to stretch one at
/// any display scaling.
pub const RECOMMENDED_WINDOWS_SIZES: [u32; 10] = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

/// The common Windows icon sizes that `present` lacks.
pub fn missing_windows_sizes(present: &HashSet<u32>) -> Vec<u32> {
    RECOMMENDED_WINDOWS_SIZES
        .iter()
        .copied()
        .filter(|size| !present.contains(size))
        .collect()
}

/// Parses an existing .ico file. `name` is how the file is called in the
/// message when it cannot be read.
pub fn read_dir(bytes: &[u8], name: &str) -> Result<ico::IconDir, String> {
    ico::IconDir::read(Cursor::new(bytes)).map_err(|e| {
        crate::msg!(
            "icon.unreadable",
            "Could not read '{name}' as an ICO file: {e}",
            name = name,
            e = e
        )
    })
}

/// The bytes of an icon directory as an .ico file.
pub fn write_dir(dir: &ico::IconDir) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    dir.write(&mut bytes).map_err(|e| {
        crate::msg!(
            "icon.write_failed",
            "Error writing the ICO file: {e}",
            e = e
        )
    })?;
    Ok(bytes)
}

/// The share (0.0 to 1.0) of an entry's pixels that are not fully opaque -
/// transparent or translucent - or `None` if the entry cannot be decoded.
/// For a BMP entry this reflects its 1-bit transparency mask.
pub fn alpha_share(entry: &ico::IconDirEntry) -> Option<f64> {
    let image = entry.decode().ok()?;
    let pixels = image.rgba_data();
    let not_opaque = pixels
        .iter()
        .skip(3)
        .step_by(4)
        .filter(|&&alpha| alpha < 255)
        .count();
    Some(not_opaque as f64 / (pixels.len() / 4).max(1) as f64)
}

/// `alpha_share` as a short text.
pub fn alpha_summary(entry: &ico::IconDirEntry) -> String {
    match alpha_share(entry) {
        None => "unknown (could not decode)".to_string(),
        Some(0.0) => "no (fully opaque)".to_string(),
        Some(share) => format!("yes ({:.0}% of pixels not fully opaque)", share * 100.0),
    }
}

/// A file name `<stem>_<w>x<h>.<extension>` that is not in `taken`: a second
/// image of the same size becomes `..._2`, and so on.
pub fn unique_file_name(
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

/// Decodes the icon at `index` and re-encodes it as PNG, so it gets the same
/// full color depth and clean alpha channel as everything else this program
/// writes, regardless of how the source file encoded it.
pub fn reencode_as_png(
    entry: &ico::IconDirEntry,
    index: usize,
    name: &str,
) -> Result<ico::IconDirEntry, String> {
    let image = entry.decode().map_err(|e| {
        crate::msg!(
            "icon.entry_undecodable",
            "Could not decode icon at index {index} in '{name}': {e}",
            index = index,
            name = name,
            e = e
        )
    })?;
    ico::IconDirEntry::encode_as_png(&image).map_err(|e| {
        crate::msg!(
            "icon.entry_not_reencoded",
            "Could not re-encode icon at index {index}: {e}",
            index = index,
            e = e
        )
    })
}

/// The icon as the bytes of a PNG file.
pub fn entry_png(entry: &ico::IconDirEntry, name: &str) -> Result<Vec<u8>, String> {
    let (w, h) = (entry.width(), entry.height());
    let image = entry.decode().map_err(|e| {
        crate::msg!(
            "icon.size_undecodable",
            "Could not decode the {w}x{h} icon in '{name}': {e}",
            w = w,
            h = h,
            name = name,
            e = e
        )
    })?;
    let rgba = RgbaImage::from_raw(w, h, image.into_rgba_data()).ok_or_else(|| {
        crate::msg!(
            "icon.pixel_data_size",
            "Unexpected pixel data size for the {w}x{h} icon",
            w = w,
            h = h
        )
    })?;
    let mut png = Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| {
            crate::msg!(
                "icon.png_encode_failed",
                "Could not encode the {w}x{h} icon as PNG: {e}",
                w = w,
                h = h,
                e = e
            )
        })?;
    Ok(png.into_inner())
}

/// An image that `merge` left out because its size was already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub width: u32,
    pub height: u32,
    /// The name of the file it came from.
    pub source: String,
}

/// Merges the icon entries of two or more existing .ico files into one, each
/// re-encoded as PNG. If two files hold the same size, the first one wins and
/// the later ones are skipped (and reported): an .ico file is not meant to hold
/// the same size twice, and most consumers only look at one of them anyway.
pub fn merge(sources: &[(&str, &ico::IconDir)]) -> Result<(ico::IconDir, Vec<Skipped>), String> {
    let mut merged = ico::IconDir::new(ico::ResourceType::Icon);
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    let mut skipped = Vec::new();

    for (name, source) in sources {
        for entry in source.entries() {
            let size = (entry.width(), entry.height());
            if !seen.insert(size) {
                skipped.push(Skipped {
                    width: size.0,
                    height: size.1,
                    source: (*name).to_string(),
                });
                continue;
            }
            let image = entry.decode().map_err(|e| {
                crate::msg!(
                    "icon.size_undecodable",
                    "Could not decode the {w}x{h} icon in '{name}': {e}",
                    w = size.0,
                    h = size.1,
                    name = name,
                    e = e
                )
            })?;
            let new_entry = ico::IconDirEntry::encode_as_png(&image).map_err(|e| {
                crate::msg!(
                    "icon.size_not_reencoded",
                    "Could not re-encode the {w}x{h} icon: {e}",
                    w = size.0,
                    h = size.1,
                    e = e
                )
            })?;
            merged.add_entry(new_entry);
        }
    }

    if merged.entries().is_empty() {
        return Err(crate::msg!(
            "icon.nothing_to_merge",
            "No icons found to merge - the resulting file would be empty."
        ));
    }
    Ok((merged, skipped))
}

/// A new icon directory with the images at `indices` (in that order),
/// each re-encoded as PNG.
pub fn select(dir: &ico::IconDir, indices: &[usize], name: &str) -> Result<ico::IconDir, String> {
    let entries = dir.entries();
    let mut out = ico::IconDir::new(ico::ResourceType::Icon);
    for &index in indices {
        let entry = entries.get(index).ok_or_else(|| {
            crate::msg!(
                "icon.index_out_of_range",
                "Index {index} is out of range for '{name}' - it contains {count} icon(s).",
                index = index,
                name = name,
                count = entries.len()
            )
        })?;
        out.add_entry(reencode_as_png(entry, index, name)?);
    }
    Ok(out)
}

/// One image of an icon file, decoded: `rgba` holds four bytes (red, green,
/// blue, alpha - not premultiplied) per pixel, row by row from the top left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// How many images the .ico file in `bytes` holds.
pub fn entry_count(bytes: &[u8]) -> Result<usize, String> {
    let dir = ico::IconDir::read(Cursor::new(bytes)).map_err(|e| {
        crate::msg!(
            "icon.file_unreadable",
            "Could not read the icon file: {e}",
            e = e
        )
    })?;
    Ok(dir.entries().len())
}

/// Decodes image number `index` (counted from 0, in the order of the file's
/// directory) of the .ico file in `bytes`.
pub fn read_entry(bytes: &[u8], index: usize) -> Result<IconImage, String> {
    let dir = ico::IconDir::read(Cursor::new(bytes)).map_err(|e| {
        crate::msg!(
            "icon.file_unreadable",
            "Could not read the icon file: {e}",
            e = e
        )
    })?;
    let entry = dir.entries().get(index).ok_or_else(|| {
        crate::msg!(
            "icon.no_such_image",
            "The icon file has {count} image(s); there is no image number {index}.",
            count = dir.entries().len(),
            index = index
        )
    })?;
    let image = entry.decode().map_err(|e| {
        crate::msg!(
            "icon.image_undecodable",
            "Could not decode image {index}: {e}",
            index = index,
            e = e
        )
    })?;
    Ok(IconImage {
        width: image.width(),
        height: image.height(),
        rgba: image.rgba_data().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An .ico of two images: 2x1 (red, then a half-transparent blue) and 1x1.
    fn sample() -> Vec<u8> {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        let first = ico::IconImage::from_rgba_data(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]);
        dir.add_entry(ico::IconDirEntry::encode_as_png(&first).unwrap());
        let second = ico::IconImage::from_rgba_data(1, 1, vec![1, 2, 3, 0]);
        dir.add_entry(ico::IconDirEntry::encode_as_png(&second).unwrap());
        let mut bytes = Vec::new();
        dir.write(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn the_images_are_counted_and_decoded_exactly() {
        let bytes = sample();
        assert_eq!(entry_count(&bytes), Ok(2));
        let first = read_entry(&bytes, 0).unwrap();
        assert_eq!((first.width, first.height), (2, 1));
        assert_eq!(first.rgba, vec![255, 0, 0, 255, 0, 0, 255, 128]);
        // A fully transparent pixel keeps its color: nothing is premultiplied.
        assert_eq!(read_entry(&bytes, 1).unwrap().rgba, vec![1, 2, 3, 0]);
    }

    fn dir_of(sizes: &[u32]) -> ico::IconDir {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        for &size in sizes {
            let pixels = [200, 200, 200, 255].repeat((size * size) as usize);
            let image = ico::IconImage::from_rgba_data(size, size, pixels);
            dir.add_entry(ico::IconDirEntry::encode_as_png(&image).unwrap());
        }
        dir
    }

    fn sizes_of(dir: &ico::IconDir) -> Vec<u32> {
        dir.entries().iter().map(|e| e.width()).collect()
    }

    #[test]
    fn merging_keeps_the_first_of_each_size_and_reports_the_rest() {
        let (a, b) = (dir_of(&[16, 32]), dir_of(&[32, 48]));
        let (merged, skipped) = merge(&[("a.ico", &a), ("b.ico", &b)]).unwrap();
        assert_eq!(sizes_of(&merged), vec![16, 32, 48]);
        assert_eq!(
            skipped,
            vec![Skipped {
                width: 32,
                height: 32,
                source: "b.ico".to_string()
            }]
        );
        assert!(merge(&[("empty.ico", &ico::IconDir::new(ico::ResourceType::Icon))]).is_err());
    }

    #[test]
    fn selecting_takes_the_images_asked_for_in_order() {
        let dir = dir_of(&[16, 32, 48]);
        let picked = select(&dir, &[2, 0], "a.ico").unwrap();
        assert_eq!(sizes_of(&picked), vec![48, 16]);
        assert!(
            select(&dir, &[3], "a.ico")
                .unwrap_err()
                .contains("out of range")
        );
    }

    #[test]
    fn an_image_can_be_taken_out_as_png() {
        let dir = dir_of(&[16]);
        let png = entry_png(&dir.entries()[0], "a.ico").unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
    }

    #[test]
    fn file_names_are_made_unique() {
        assert_eq!(
            unique_file_name(&[], "logo", 16, 16, "png"),
            "logo_16x16.png"
        );
        let taken = vec!["logo_16x16.png".to_string()];
        assert_eq!(
            unique_file_name(&taken, "logo", 16, 16, "png"),
            "logo_16x16_2.png"
        );
    }

    #[test]
    fn the_missing_windows_sizes_are_named() {
        let present: HashSet<u32> = [16, 32, 48, 256].into_iter().collect();
        assert_eq!(
            missing_windows_sizes(&present),
            vec![20, 24, 40, 64, 96, 128]
        );
    }

    #[test]
    fn alpha_is_summarized() {
        let opaque = dir_of(&[16]);
        assert_eq!(alpha_summary(&opaque.entries()[0]), "no (fully opaque)");
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        let half = ico::IconImage::from_rgba_data(2, 1, vec![0, 0, 0, 255, 0, 0, 0, 0]);
        dir.add_entry(ico::IconDirEntry::encode_as_png(&half).unwrap());
        assert_eq!(alpha_share(&dir.entries()[0]), Some(0.5));
        assert!(alpha_summary(&dir.entries()[0]).starts_with("yes (50%"));
    }

    #[test]
    fn a_missing_image_and_a_broken_file_are_clear_errors() {
        assert!(
            read_entry(&sample(), 2)
                .unwrap_err()
                .contains("no image number 2")
        );
        assert!(read_entry(b"nope", 0).is_err());
        assert!(entry_count(b"").is_err());
    }
}
