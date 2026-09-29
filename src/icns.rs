// Everything related to writing the macOS .icns icon format: the size-to-
// OSType mapping Apple's own tools use, and the container writer itself.

use crate::resize::make_square_icon;
use image::{ImageEncoder, RgbaImage};
use std::path::Path;

/// Standard Apple icon sizes (in pixels) mapped to their corresponding
/// ICNS "OSType" codes, matching what Apple's own `iconutil` produces from
/// a .iconset folder of PNGs. Some pixel sizes map to more than one
/// OSType, because the ICNS format distinguishes a size's "native"
/// resolution from its use as the 2x ("Retina") asset of a smaller
/// nominal size - e.g. a 64x64 pixel image is both the native "64x64"
/// icon (icp6) AND the Retina asset for "32x32" (ic12). In a real design
/// workflow those could be different artwork; here they're pixel-identical
/// since we generate every size from the same source image, so we simply
/// reuse the same encoded PNG bytes for both OSTypes.
///
/// Also used by ico_ops::inspect_source_image, to report which of these
/// sizes a source image can cover natively vs. would need upscaling.
pub const ICNS_SIZES: &[(u32, &[[u8; 4]])] = &[
    (16, &[*b"icp4"]),
    (32, &[*b"icp5", *b"ic11"]),
    (64, &[*b"icp6", *b"ic12"]),
    (128, &[*b"ic07"]),
    (256, &[*b"ic08", *b"ic13"]),
    (512, &[*b"ic09", *b"ic14"]),
    (1024, &[*b"ic10"]),
];

/// Writes an .icns (macOS icon) file.
///
/// Container format (verified against the public ICNS specification):
///   - 8-byte file header: 4-byte magic "icns", then a 4-byte big-endian
///     total file length.
///   - Followed directly by any number of entries, back to back. Each
///     entry is: a 4-byte OSType code (e.g. "ic07"), a 4-byte big-endian
///     length (INCLUDING this 8-byte entry header, not just the payload),
///     and then the payload itself.
///   - Since macOS 10.7, the payload for all the OSTypes we use here is
///     simply a complete, standalone PNG file - nothing more exotic than
///     that.
///
/// This function's container-writing logic was checked byte-by-byte
/// against the public ICNS format documentation and the resulting file
/// was independently verified to parse back correctly with a separate
/// ICNS implementation (see this project's history for the exact checks
/// performed) - built and verified without access to an actual Mac, since
/// the environment this was written in only has Linux available. A
/// real-world test on macOS is still worth doing before relying on this
/// for anything important.
pub fn write_icns(
    rgba_source: &RgbaImage,
    padding: u8,
    has_alpha: bool,
    output_path: &Path,
) -> Result<(), String> {
    let mut body: Vec<u8> = Vec::new();

    for &(size, type_codes) in ICNS_SIZES {
        let square = make_square_icon(rgba_source, size, padding, has_alpha);

        // Encode this size as a standalone PNG in memory (not a file on
        // disk) - image::codecs::png::PngEncoder can write directly into
        // any std::io::Write, and a Vec<u8> implements that.
        let mut png_bytes: Vec<u8> = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(square.as_raw(), size, size, image::ExtendedColorType::Rgba8)
            .map_err(|e| format!("Could not encode the {size}x{size} icon as PNG: {e}"))?;

        for type_code in type_codes {
            body.extend_from_slice(type_code);
            let entry_len: u32 = 8 + png_bytes.len() as u32;
            body.extend_from_slice(&entry_len.to_be_bytes());
            body.extend_from_slice(&png_bytes);
        }
    }

    let total_len: u32 = 8 + body.len() as u32;
    let mut file_bytes: Vec<u8> = Vec::with_capacity(total_len as usize);
    file_bytes.extend_from_slice(b"icns");
    file_bytes.extend_from_slice(&total_len.to_be_bytes());
    file_bytes.extend_from_slice(&body);

    std::fs::write(output_path, &file_bytes).map_err(|e| format!("Could not write ICNS file: {e}"))?;

    println!(
        "Done: '{}' created with {} icon size(s) (icns format).",
        output_path.display(),
        ICNS_SIZES.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    struct Entry {
        os_type: [u8; 4],
        payload: Vec<u8>,
    }

    /// Parses an .icns file back into its entries, checking the container
    /// structure on the way.
    fn parse_icns(bytes: &[u8]) -> Vec<Entry> {
        assert_eq!(&bytes[0..4], b"icns", "missing magic");
        let total = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(total, bytes.len(), "header length must equal the file length");

        let mut entries = Vec::new();
        let mut pos = 8;
        while pos < bytes.len() {
            let os_type: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap();
            let len = u32::from_be_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
            assert!(len >= 8 && pos + len <= bytes.len(), "entry length out of range");
            entries.push(Entry {
                os_type,
                payload: bytes[pos + 8..pos + len].to_vec(),
            });
            pos += len;
        }
        assert_eq!(pos, bytes.len(), "entries must end exactly at the end of the file");
        entries
    }

    fn write_sample(dir: &Path, source: &RgbaImage, padding: u8) -> Vec<u8> {
        let path = dir.join("out.icns");
        let has_alpha = crate::resize::has_transparency(source);
        write_icns(source, padding, has_alpha, &path).unwrap();
        std::fs::read(path).unwrap()
    }

    #[test]
    fn size_table_covers_the_standard_apple_sizes() {
        let sizes: Vec<u32> = ICNS_SIZES.iter().map(|&(size, _)| size).collect();
        assert_eq!(sizes, vec![16, 32, 64, 128, 256, 512, 1024]);
    }

    #[test]
    fn every_os_type_is_unique() {
        let mut all: Vec<[u8; 4]> = ICNS_SIZES.iter().flat_map(|&(_, codes)| codes.iter().copied()).collect();
        let before = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), before);
    }

    #[test]
    fn file_has_a_valid_icns_container() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(64, 64, Rgba([200, 30, 30, 255]));
        let bytes = write_sample(dir.path(), &source, 0);
        // parse_icns asserts magic, total length and entry framing.
        let entries = parse_icns(&bytes);
        assert_eq!(entries.len(), 11);
    }

    #[test]
    fn all_expected_os_types_are_present() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 255]));
        let entries = parse_icns(&write_sample(dir.path(), &source, 0));
        let written: Vec<[u8; 4]> = entries.iter().map(|e| e.os_type).collect();
        for &(_, codes) in ICNS_SIZES {
            for code in codes {
                assert!(written.contains(code), "missing {}", String::from_utf8_lossy(code));
            }
        }
    }

    #[test]
    fn every_payload_is_a_png_with_the_matching_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(50, 50, Rgba([10, 200, 90, 255]));
        let entries = parse_icns(&write_sample(dir.path(), &source, 0));

        for &(size, codes) in ICNS_SIZES {
            for code in codes {
                let entry = entries.iter().find(|e| e.os_type == *code).unwrap();
                assert_eq!(&entry.payload[..8], &PNG_SIGNATURE, "not a PNG");
                let decoded = image::load_from_memory(&entry.payload).unwrap();
                assert_eq!((decoded.width(), decoded.height()), (size, size));
            }
        }
    }

    #[test]
    fn retina_variants_share_the_same_pixels_as_their_native_size() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(64, 64, Rgba([1, 2, 3, 255]));
        let entries = parse_icns(&write_sample(dir.path(), &source, 0));
        let payload = |code: &[u8; 4]| entries.iter().find(|e| e.os_type == *code).unwrap().payload.clone();
        assert_eq!(payload(b"icp5"), payload(b"ic11"));
        assert_eq!(payload(b"icp6"), payload(b"ic12"));
    }

    #[test]
    fn non_square_sources_are_letterboxed_inside_every_size() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(200, 50, Rgba([255, 0, 0, 255]));
        let entries = parse_icns(&write_sample(dir.path(), &source, 0));
        let entry = entries.iter().find(|e| e.os_type == *b"ic07").unwrap(); // 128x128
        let decoded = image::load_from_memory(&entry.payload).unwrap().to_rgba8();
        assert_eq!(decoded.get_pixel(64, 64)[3], 255, "centre is content");
        assert_eq!(decoded.get_pixel(64, 2)[3], 0, "top margin is transparent");
    }

    #[test]
    fn padding_is_applied_to_every_size() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 0, 255]));
        let entries = parse_icns(&write_sample(dir.path(), &source, 50));
        let entry = entries.iter().find(|e| e.os_type == *b"ic08").unwrap(); // 256x256
        let decoded = image::load_from_memory(&entry.payload).unwrap().to_rgba8();
        assert_eq!(decoded.get_pixel(5, 128)[3], 0);
        assert_eq!(decoded.get_pixel(128, 128)[3], 255);
    }

    #[test]
    fn writing_to_a_missing_directory_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let path = dir.path().join("no-such-dir").join("out.icns");
        let err = write_icns(&source, 0, false, &path).unwrap_err();
        assert!(err.contains("Could not write ICNS file"), "unexpected message: {err}");
    }
}
