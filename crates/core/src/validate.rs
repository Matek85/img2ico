// --validate: checks the structure of existing .ico files and says exactly
// what is wrong with a damaged one.
//
// This is deliberately NOT built on the `ico` crate: that library refuses a
// damaged file with one short error, which is no help in finding out what is
// broken. The parser here reads the bytes itself - header, directory,
// image data - and reports every problem it can find, with the entry and the
// numbers involved, so a build script can fail on it and a person can fix it.
//
// What an .ico file looks like (all numbers little-endian):
//   header, 6 bytes:      reserved (0), type (1 = icon, 2 = cursor), count
//   directory, 16 bytes   width, height (0 means 256), color count, reserved,
//   per image:            planes, bits per pixel, size of the image data,
//                         offset of the image data from the file's start
//   image data:           a complete PNG file, or a BMP without its file
//                         header (a DIB: height doubled for the AND mask)

use std::collections::BTreeSet;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The file is damaged or will not be read correctly.
    Error,
    /// The file works but is unusual, or not what was probably intended.
    Warning,
}

/// One problem found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The zero-based directory entry it is about, if it is about one.
    pub entry: Option<usize>,
    pub message: String,
}

/// How the image data of an entry is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Png,
    Bmp,
    Unknown,
}

impl DataFormat {
    pub fn name(self) -> &'static str {
        match self {
            DataFormat::Png => "PNG",
            DataFormat::Bmp => "BMP",
            DataFormat::Unknown => "unknown",
        }
    }
}

/// What the directory says about one image, plus what the data turned out to
/// be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFacts {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u16,
    pub offset: u32,
    pub size: u32,
    pub format: DataFormat,
}

/// The result of checking one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub file_size: u64,
    /// 1 for an icon, 2 for a cursor, as in the header (0 if unreadable).
    pub kind: u16,
    pub entries: Vec<EntryFacts>,
    pub findings: Vec<Finding>,
}

impl Report {
    /// Whether the file has no errors (warnings do not make it invalid).
    pub fn is_valid(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    pub fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }
}

const HEADER_LEN: usize = 6;
const DIR_ENTRY_LEN: usize = 16;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// Windows reads icon images up to 256x256.
const MAX_ICON_EDGE: u32 = 256;

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Checks `bytes` as an .ico file.
pub fn validate_bytes(bytes: &[u8]) -> Report {
    let mut report = Report {
        file_size: bytes.len() as u64,
        kind: 0,
        entries: Vec::new(),
        findings: Vec::new(),
    };
    let mut add = |severity, entry: Option<usize>, message: String| {
        report.findings.push(Finding {
            severity,
            entry,
            message,
        });
    };

    // --- header -------------------------------------------------------------
    if bytes.is_empty() {
        add(Severity::Error, None, "the file is empty".to_string());
        return report;
    }
    if bytes.len() < HEADER_LEN {
        add(
            Severity::Error,
            None,
            format!(
                "the file is {} bytes, shorter than the {HEADER_LEN}-byte ICO header",
                bytes.len()
            ),
        );
        return report;
    }
    let reserved = u16_at(bytes, 0);
    let kind = u16_at(bytes, 2);
    let count = usize::from(u16_at(bytes, 4));
    report.kind = kind;
    let mut add = |severity, entry: Option<usize>, message: String| {
        report.findings.push(Finding {
            severity,
            entry,
            message,
        });
    };

    if reserved != 0 {
        add(
            Severity::Error,
            None,
            format!(
                "the header's reserved field is {reserved}, it must be 0 - this does not look like an ICO file{}",
                if bytes.starts_with(&PNG_SIGNATURE) {
                    " (it is a PNG image: convert it with img2ico instead)"
                } else {
                    ""
                }
            ),
        );
        return report;
    }
    match kind {
        1 => {}
        2 => add(
            Severity::Warning,
            None,
            "the file is a cursor (.cur), not an icon (header type 2)".to_string(),
        ),
        other => {
            add(
                Severity::Error,
                None,
                format!("the header's type is {other}; an icon file has type 1 (a cursor, 2)"),
            );
            return report;
        }
    }
    if count == 0 {
        add(
            Severity::Error,
            None,
            "the header says the file holds no images".to_string(),
        );
        return report;
    }

    // --- directory ------------------------------------------------------------
    let directory_end = HEADER_LEN + count * DIR_ENTRY_LEN;
    if bytes.len() < directory_end {
        let complete = (bytes.len() - HEADER_LEN) / DIR_ENTRY_LEN;
        add(
            Severity::Error,
            None,
            format!(
                "the header announces {count} images, whose directory needs {directory_end} bytes, but the file has only {} bytes (room for {complete} directory entries)",
                bytes.len()
            ),
        );
        return report;
    }

    let mut ranges: Vec<(u64, u64, usize)> = Vec::new();
    let mut seen_sizes = BTreeSet::new();
    for index in 0..count {
        let at = HEADER_LEN + index * DIR_ENTRY_LEN;
        let width = if bytes[at] == 0 {
            256
        } else {
            u32::from(bytes[at])
        };
        let height = if bytes[at + 1] == 0 {
            256
        } else {
            u32::from(bytes[at + 1])
        };
        let planes = u16_at(bytes, at + 4);
        let bits = u16_at(bytes, at + 6);
        let size = u32_at(bytes, at + 8);
        let offset = u32_at(bytes, at + 12);

        let mut facts = EntryFacts {
            index,
            width,
            height,
            bits_per_pixel: bits,
            offset,
            size,
            format: DataFormat::Unknown,
        };

        if bytes[at + 3] != 0 {
            add(
                Severity::Warning,
                Some(index),
                format!(
                    "the reserved byte of its directory entry is {}, it should be 0",
                    bytes[at + 3]
                ),
            );
        }
        if kind == 1 && planes > 1 {
            add(
                Severity::Warning,
                Some(index),
                format!("the directory says {planes} color planes; an icon has 0 or 1"),
            );
        }
        if ![0, 1, 2, 4, 8, 16, 24, 32].contains(&bits) {
            add(
                Severity::Warning,
                Some(index),
                format!("the directory says {bits} bits per pixel, which is not a usual value"),
            );
        }
        if !seen_sizes.insert((width, height, bits)) {
            add(
                Severity::Warning,
                Some(index),
                format!(
                    "the size {width}x{height} at {bits} bits per pixel is in the file more than once"
                ),
            );
        }
        if size == 0 {
            add(
                Severity::Error,
                Some(index),
                "the directory says the image data is 0 bytes long".to_string(),
            );
            report.entries.push(facts);
            continue;
        }
        let (start, end) = (u64::from(offset), u64::from(offset) + u64::from(size));
        if start < directory_end as u64 {
            add(
                Severity::Error,
                Some(index),
                format!(
                    "its image data starts at byte {offset}, inside the header and directory (which end at byte {directory_end})"
                ),
            );
            report.entries.push(facts);
            continue;
        }
        if end > bytes.len() as u64 {
            add(
                Severity::Error,
                Some(index),
                format!(
                    "its image data (bytes {offset} to {end}) reaches beyond the end of the file, which has {} bytes - the file is truncated or the directory is wrong",
                    bytes.len()
                ),
            );
            report.entries.push(facts);
            continue;
        }
        ranges.push((start, end, index));

        let data = &bytes[start as usize..end as usize];
        facts.format = check_image_data(index, data, &facts, &mut add);
        report.entries.push(facts);
    }

    // --- how the pieces lie in the file ----------------------------------------
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        let (_, previous_end, previous) = pair[0];
        let (start, _, next) = pair[1];
        if start < previous_end {
            add(
                Severity::Error,
                Some(next),
                format!(
                    "its image data (from byte {start}) overlaps the data of image [{previous}] (which runs to byte {previous_end})"
                ),
            );
        }
    }
    if let (Some(first), Some(last)) = (ranges.first(), ranges.iter().map(|r| r.1).max()) {
        if first.0 > directory_end as u64 {
            add(
                Severity::Warning,
                None,
                format!(
                    "{} unused bytes between the directory and the first image",
                    first.0 - directory_end as u64
                ),
            );
        }
        if last < bytes.len() as u64 {
            add(
                Severity::Warning,
                None,
                format!(
                    "{} bytes after the last image that no directory entry points to",
                    bytes.len() as u64 - last
                ),
            );
        }
    }

    report
}

/// Checks the data of one image against what its directory entry says and
/// returns how it is stored.
fn check_image_data(
    index: usize,
    data: &[u8],
    facts: &EntryFacts,
    add: &mut impl FnMut(Severity, Option<usize>, String),
) -> DataFormat {
    let (width, height) = (facts.width, facts.height);
    if data.starts_with(&PNG_SIGNATURE) {
        match check_png(data) {
            Err(problem) => add(
                Severity::Error,
                Some(index),
                format!("PNG image data: {problem}"),
            ),
            Ok((png_width, png_height)) => {
                if (png_width, png_height) != (width, height) {
                    add(
                        Severity::Error,
                        Some(index),
                        format!(
                            "the directory says {width}x{height}, but the PNG inside is {png_width}x{png_height}"
                        ),
                    );
                }
                if png_width > MAX_ICON_EDGE || png_height > MAX_ICON_EDGE {
                    add(
                        Severity::Warning,
                        Some(index),
                        format!(
                            "the PNG is {png_width}x{png_height}; Windows reads icon images only up to {MAX_ICON_EDGE}x{MAX_ICON_EDGE}"
                        ),
                    );
                }
            }
        }
        return DataFormat::Png;
    }

    // Not a PNG: it has to be a DIB (a BMP without the 14-byte file header).
    if data.len() < 40 {
        add(
            Severity::Error,
            Some(index),
            format!(
                "the image data ({} bytes) is neither a PNG nor a complete BMP header (40 bytes)",
                data.len()
            ),
        );
        return DataFormat::Unknown;
    }
    let header_size = u32_at(data, 0);
    // The sizes of the BMP info header versions (3.x, 4.x and 5.x).
    if ![40, 52, 56, 64, 108, 124].contains(&header_size) {
        add(
            Severity::Error,
            Some(index),
            format!(
                "the image data is neither a PNG nor a BMP: its first four bytes would be a BMP header size of {header_size}, which no BMP version has"
            ),
        );
        return DataFormat::Unknown;
    }
    let bmp_width = i64::from(u32_at(data, 4) as i32);
    let bmp_height = i64::from(u32_at(data, 8) as i32);
    let planes = u16_at(data, 12);
    let bits = u16_at(data, 14);
    let compression = u32_at(data, 16);
    let colors_used = u64::from(u32_at(data, 32));

    if bmp_width != i64::from(width) {
        add(
            Severity::Error,
            Some(index),
            format!("the directory says width {width}, but the BMP inside is {bmp_width} wide"),
        );
    }
    // The BMP in an icon holds the color image and the AND mask on top of
    // each other, so its height is twice the icon's.
    if bmp_height != 2 * i64::from(height) {
        let note = if bmp_height == i64::from(height) {
            " - without the doubling for the transparency mask, so it has no mask"
        } else {
            ""
        };
        add(
            Severity::Error,
            Some(index),
            format!(
                "the directory says height {height}, so the BMP should be {} high (color image and mask), but it is {bmp_height}{note}",
                2 * height
            ),
        );
    }
    if planes != 1 {
        add(
            Severity::Error,
            Some(index),
            format!("the BMP says {planes} color planes, it must be 1"),
        );
    }
    if ![1, 4, 8, 16, 24, 32].contains(&bits) {
        add(
            Severity::Error,
            Some(index),
            format!("the BMP says {bits} bits per pixel, which BMP does not have"),
        );
        return DataFormat::Bmp;
    }
    if facts.bits_per_pixel != 0 && facts.bits_per_pixel != bits {
        add(
            Severity::Warning,
            Some(index),
            format!(
                "the directory says {} bits per pixel, the BMP inside {bits}",
                facts.bits_per_pixel
            ),
        );
    }
    if compression != 0 && compression != 3 {
        add(
            Severity::Error,
            Some(index),
            format!(
                "the BMP is compressed (method {compression}); icons use uncompressed BMP data"
            ),
        );
        return DataFormat::Bmp;
    }

    // How many bytes the pixel data has to have.
    let (w, h, b) = (u64::from(width), u64::from(height), u64::from(bits));
    let palette = match bits {
        1..=8 => {
            (if colors_used > 0 {
                colors_used
            } else {
                1 << bits
            }) * 4
        }
        _ => colors_used * 4,
    };
    let masks = if compression == 3 && header_size == 40 {
        12
    } else {
        0
    };
    let color_rows = (w * b).div_ceil(32) * 4 * h;
    let mask_rows = w.div_ceil(32) * 4 * h;
    let needed = u64::from(header_size) + masks + palette + color_rows + mask_rows;
    let have = data.len() as u64;
    // A 32-bit BMP carries its transparency in the pixels, and many writers
    // leave out the AND mask that is otherwise part of every icon image.
    let minimum = if bits == 32 {
        needed - mask_rows
    } else {
        needed
    };
    if have < minimum {
        add(
            Severity::Error,
            Some(index),
            format!(
                "the BMP data is truncated: a {width}x{height} image with {bits} bits per pixel needs {minimum} bytes, the entry has {have}"
            ),
        );
    } else if have > needed {
        add(
            Severity::Warning,
            Some(index),
            format!(
                "the BMP data has {} bytes more than a {width}x{height} image with {bits} bits per pixel needs",
                have - needed
            ),
        );
    }
    DataFormat::Bmp
}

/// Walks the chunks of a PNG, checks their lengths and checksums, and returns
/// its width and height.
fn check_png(data: &[u8]) -> Result<(u32, u32), String> {
    let mut at = PNG_SIGNATURE.len();
    let mut dimensions = None;
    let mut has_pixels = false;
    let mut first = true;
    while at + 12 <= data.len() {
        let length =
            u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        let kind = &data[at + 4..at + 8];
        let name = String::from_utf8_lossy(kind).into_owned();
        let end = at + 8 + length + 4;
        if end > data.len() {
            return Err(format!(
                "the chunk '{name}' says it is {length} bytes long, which reaches beyond the end of the image data - truncated"
            ));
        }
        let stored =
            u32::from_be_bytes([data[end - 4], data[end - 3], data[end - 2], data[end - 1]]);
        if crc32(&data[at + 4..end - 4]) != stored {
            return Err(format!(
                "the checksum of the chunk '{name}' is wrong - the data is corrupted"
            ));
        }
        if first {
            if kind != b"IHDR" || length != 13 {
                return Err("the first chunk is not a valid IHDR".to_string());
            }
            let body = &data[at + 8..at + 8 + 13];
            dimensions = Some((
                u32::from_be_bytes([body[0], body[1], body[2], body[3]]),
                u32::from_be_bytes([body[4], body[5], body[6], body[7]]),
            ));
            first = false;
        }
        if kind == b"IDAT" {
            has_pixels = true;
        }
        if kind == b"IEND" {
            if !has_pixels {
                return Err("there is no IDAT chunk - the image has no pixel data".to_string());
            }
            return dimensions.ok_or_else(|| "there is no IHDR chunk".to_string());
        }
        at = end;
    }
    Err("the end chunk IEND is missing - the image data is truncated".to_string())
}

/// The CRC-32 PNG uses for its chunks.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png(size: u32) -> Vec<u8> {
        let img = RgbaImage::from_pixel(size, size, Rgba([10, 20, 30, 255]));
        let mut bytes = Vec::new();
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(img.as_raw(), size, size, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }

    /// A good .ico with PNG images of these sizes.
    fn good_ico(sizes: &[u32]) -> Vec<u8> {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        for &size in sizes {
            let image =
                ico::IconImage::from_rgba_data(size, size, vec![50; (size * size * 4) as usize]);
            dir.add_entry(ico::IconDirEntry::encode_as_png(&image).unwrap());
        }
        let mut bytes = Vec::new();
        dir.write(&mut bytes).unwrap();
        bytes
    }

    /// An .ico whose images are old-style BMPs.
    fn bmp_ico(size: u32) -> Vec<u8> {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        let image =
            ico::IconImage::from_rgba_data(size, size, vec![255; (size * size * 4) as usize]);
        dir.add_entry(ico::IconDirEntry::encode_as_bmp(&image).unwrap());
        let mut bytes = Vec::new();
        dir.write(&mut bytes).unwrap();
        bytes
    }

    fn messages(report: &Report, severity: Severity) -> Vec<String> {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| f.message.clone())
            .collect()
    }

    fn has_error(report: &Report, needle: &str) -> bool {
        messages(report, Severity::Error)
            .iter()
            .any(|m| m.contains(needle))
    }

    #[test]
    fn a_good_icon_file_is_valid_and_described() {
        let report = validate_bytes(&good_ico(&[16, 32, 256]));
        assert!(report.is_valid(), "{:?}", report.findings);
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.kind, 1);
        let sizes: Vec<u32> = report.entries.iter().map(|e| e.width).collect();
        assert_eq!(sizes, vec![16, 32, 256]);
        assert!(report.entries.iter().all(|e| e.format == DataFormat::Png));
    }

    #[test]
    fn a_bmp_icon_is_valid_too() {
        let report = validate_bytes(&bmp_ico(16));
        assert!(report.is_valid(), "{:?}", report.findings);
        assert_eq!(report.entries[0].format, DataFormat::Bmp);
    }

    #[test]
    fn the_empty_file_and_a_short_file_are_errors() {
        assert!(has_error(&validate_bytes(&[]), "empty"));
        assert!(has_error(
            &validate_bytes(&[0, 0, 1]),
            "shorter than the 6-byte ICO header"
        ));
    }

    #[test]
    fn a_wrong_header_is_named() {
        let mut bytes = good_ico(&[16]);
        bytes[0] = 7;
        assert!(has_error(&validate_bytes(&bytes), "reserved field is 7"));

        let mut bytes = good_ico(&[16]);
        bytes[2] = 9;
        assert!(has_error(&validate_bytes(&bytes), "type is 9"));

        let mut bytes = good_ico(&[16]);
        bytes[4] = 0;
        bytes[5] = 0;
        assert!(has_error(&validate_bytes(&bytes), "no images"));
    }

    #[test]
    fn a_png_given_instead_of_an_ico_is_recognized() {
        let report = validate_bytes(&png(16));
        assert!(
            has_error(&report, "it is a PNG image"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn a_cursor_is_valid_but_warned_about() {
        let mut bytes = good_ico(&[16]);
        bytes[2] = 2;
        let report = validate_bytes(&bytes);
        assert!(report.is_valid());
        assert!(
            messages(&report, Severity::Warning)
                .iter()
                .any(|m| m.contains("cursor"))
        );
    }

    #[test]
    fn a_count_larger_than_the_directory_is_an_error() {
        let mut bytes = good_ico(&[16, 32]);
        bytes[4] = 200;
        let report = validate_bytes(&bytes);
        assert!(
            has_error(&report, "announces 200 images"),
            "{:?}",
            report.findings
        );
        assert!(has_error(&report, "room for "));
    }

    #[test]
    fn a_truncated_file_names_the_entry_and_the_numbers() {
        let bytes = good_ico(&[16, 32, 64]);
        let cut = &bytes[..bytes.len() - 20];
        let report = validate_bytes(cut);
        assert!(!report.is_valid());
        let errors = messages(&report, Severity::Error);
        assert!(
            errors
                .iter()
                .any(|m| m.contains("reaches beyond the end of the file")),
            "{errors:?}"
        );
        let finding = report
            .findings
            .iter()
            .find(|f| f.severity == Severity::Error)
            .unwrap();
        assert_eq!(
            finding.entry,
            Some(2),
            "the last image is the one that is cut"
        );
    }

    #[test]
    fn an_offset_inside_the_directory_is_an_error() {
        let mut bytes = good_ico(&[16]);
        // The offset field of the first directory entry sits at byte 18.
        bytes[18..22].copy_from_slice(&3u32.to_le_bytes());
        assert!(has_error(
            &validate_bytes(&bytes),
            "inside the header and directory"
        ));
    }

    #[test]
    fn a_zero_length_entry_is_an_error() {
        let mut bytes = good_ico(&[16]);
        bytes[14..18].copy_from_slice(&0u32.to_le_bytes());
        assert!(has_error(&validate_bytes(&bytes), "0 bytes long"));
    }

    #[test]
    fn overlapping_images_are_an_error() {
        let mut bytes = good_ico(&[16, 32]);
        // Point the second image at the start of the first one's data.
        let first_offset = u32_at(&bytes, 18);
        bytes[6 + 16 + 12..6 + 16 + 16].copy_from_slice(&(first_offset + 4).to_le_bytes());
        let report = validate_bytes(&bytes);
        assert!(
            has_error(&report, "overlaps the data of image [0]"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn a_directory_size_that_disagrees_with_the_png_is_an_error() {
        let mut bytes = good_ico(&[32]);
        bytes[6] = 16;
        bytes[7] = 16;
        let report = validate_bytes(&bytes);
        assert!(
            has_error(&report, "directory says 16x16, but the PNG inside is 32x32"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn a_zero_in_the_directory_means_256() {
        let report = validate_bytes(&good_ico(&[256]));
        assert!(report.is_valid());
        assert_eq!(report.entries[0].width, 256);
    }

    #[test]
    fn a_damaged_png_is_found_by_its_checksum_and_its_ends() {
        let mut bytes = good_ico(&[16]);
        let offset = u32_at(&bytes, 18) as usize;
        // Flip a byte inside the pixel data.
        let middle = offset + 40;
        bytes[middle] ^= 0xFF;
        let report = validate_bytes(&bytes);
        assert!(has_error(&report, "checksum"), "{:?}", report.findings);

        // A PNG whose IEND is cut off.
        let mut bytes = good_ico(&[16]);
        let size = u32_at(&bytes, 14);
        bytes[14..18].copy_from_slice(&(size - 12).to_le_bytes());
        let report = validate_bytes(&bytes);
        assert!(has_error(&report, "IEND"), "{:?}", report.findings);
    }

    #[test]
    fn a_truncated_bmp_is_found_by_its_size() {
        let mut bytes = bmp_ico(16);
        let size = u32_at(&bytes, 14);
        bytes[14..18].copy_from_slice(&(size - 30).to_le_bytes());
        let cut = bytes.len() - 30;
        let report = validate_bytes(&bytes[..cut]);
        assert!(
            has_error(&report, "BMP data is truncated"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn a_bmp_with_the_wrong_height_is_explained() {
        let mut bytes = bmp_ico(16);
        let offset = u32_at(&bytes, 18) as usize;
        bytes[offset + 8..offset + 12].copy_from_slice(&16i32.to_le_bytes());
        let report = validate_bytes(&bytes);
        assert!(
            has_error(&report, "should be 32 high"),
            "{:?}",
            report.findings
        );
        assert!(has_error(&report, "no mask"));
    }

    #[test]
    fn garbage_as_image_data_is_an_error() {
        let mut bytes = good_ico(&[16]);
        let offset = u32_at(&bytes, 18) as usize;
        let size = u32_at(&bytes, 14) as usize;
        for byte in &mut bytes[offset..offset + size] {
            *byte = 0x41;
        }
        let report = validate_bytes(&bytes);
        assert!(
            has_error(&report, "neither a PNG nor a BMP"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn extra_bytes_and_duplicate_sizes_are_only_warnings() {
        let mut bytes = good_ico(&[16]);
        bytes.extend_from_slice(&[0; 7]);
        let report = validate_bytes(&bytes);
        assert!(report.is_valid());
        assert!(
            messages(&report, Severity::Warning)
                .iter()
                .any(|m| m.contains("7 bytes after the last image"))
        );

        let report = validate_bytes(&good_ico(&[16, 16]));
        assert!(report.is_valid());
        assert!(
            messages(&report, Severity::Warning)
                .iter()
                .any(|m| m.contains("more than once"))
        );
    }

    #[test]
    fn the_crc_matches_the_known_check_value() {
        // The standard check value of CRC-32 for "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn random_bytes_never_panic() {
        // A cheap fuzz: every prefix of a good file and a byte-flipped copy.
        let good = good_ico(&[16, 32]);
        for end in 0..good.len() {
            let _ = validate_bytes(&good[..end]);
        }
        for at in 0..good.len().min(120) {
            let mut copy = good.clone();
            copy[at] ^= 0xA5;
            let _ = validate_bytes(&copy);
        }
    }

    #[test]
    fn a_32_bit_bmp_without_an_and_mask_is_fine() {
        // 16x16, 32 bits: header 40 + 16*16*4 pixel bytes, no mask.
        let mut data = vec![0u8; 40 + 16 * 16 * 4];
        data[0..4].copy_from_slice(&40u32.to_le_bytes());
        data[4..8].copy_from_slice(&16i32.to_le_bytes());
        data[8..12].copy_from_slice(&32i32.to_le_bytes());
        data[12..14].copy_from_slice(&1u16.to_le_bytes());
        data[14..16].copy_from_slice(&32u16.to_le_bytes());
        let mut file = vec![0, 0, 1, 0, 1, 0, 16, 16, 0, 0, 1, 0, 32, 0];
        file.extend_from_slice(&(data.len() as u32).to_le_bytes());
        file.extend_from_slice(&22u32.to_le_bytes());
        file.extend_from_slice(&data);
        let report = validate_bytes(&file);
        assert!(report.is_valid(), "{:?}", report.findings);
        // Cutting it further is still found.
        file[14..18].copy_from_slice(&((data.len() - 10) as u32).to_le_bytes());
        let cut = file.len() - 10;
        assert!(has_error(&validate_bytes(&file[..cut]), "truncated"));
    }

    #[test]
    fn the_same_size_at_different_color_depths_is_not_a_duplicate() {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        let image = ico::IconImage::from_rgba_data(16, 16, vec![255; 16 * 16 * 4]);
        dir.add_entry(ico::IconDirEntry::encode_as_bmp(&image).unwrap());
        dir.add_entry(ico::IconDirEntry::encode_as_png(&image).unwrap());
        let mut bytes = Vec::new();
        dir.write(&mut bytes).unwrap();
        let report = validate_bytes(&bytes);
        assert!(report.is_valid(), "{:?}", report.findings);
        let duplicate = |r: &Report| {
            messages(r, Severity::Warning)
                .iter()
                .any(|m| m.contains("more than once"))
        };
        // Same depth in both directory entries: a duplicate; different: none.
        bytes[6 + 6] = 32;
        bytes[6 + 16 + 6] = 32;
        assert!(
            duplicate(&validate_bytes(&bytes)),
            "same size and depth is a duplicate"
        );
        bytes[6 + 6] = 8;
        assert!(!duplicate(&validate_bytes(&bytes)));
    }
}
