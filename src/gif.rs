// Support for detecting animated GIF input and selecting a specific frame
// from it. image::open() - used everywhere else in this project for
// loading a source image - only ever returns a GIF's FIRST frame (the
// standard, non-animation-aware decode path), with no way to ask for a
// different one and no visibility into how many frames exist at all.
// This module fills that one gap; everything else about a GIF (or any
// other format) still goes through the normal image::open() path.

use image::AnimationDecoder;
use image::codecs::gif::GifDecoder;
use std::io::{BufRead, BufReader, Cursor, Seek};
use std::path::Path;

/// Returns true if `path` is actually a GIF file, detected from its
/// CONTENT (the file's own header bytes), not just its extension -
/// consistent with how image::open() itself detects formats elsewhere in
/// this project, so a mislabeled file (e.g. a PNG saved with a ".gif"
/// extension) is handled correctly either way.
pub fn is_gif(path: &Path) -> Result<bool, String> {
    let format = image::ImageReader::open(path)
        .map_err(|e| format!("Could not read '{}': {e}", path.display()))?
        .with_guessed_format()
        .map_err(|e| format!("Could not read '{}': {e}", path.display()))?
        .format();
    Ok(format == Some(image::ImageFormat::Gif))
}

/// Whether `bytes` - an image already read into memory, for instance from
/// standard input - start like a GIF. Like `is_gif`, this goes by the content.
pub fn is_gif_bytes(bytes: &[u8]) -> bool {
    image::guess_format(bytes).is_ok_and(|format| format == image::ImageFormat::Gif)
}

/// Decodes every frame of the GIF at `path` and returns the total count -
/// used by --inspect to report it. GIF files don't store a frame count
/// up front (a GIF is just a sequence of image blocks, one after
/// another, until the file ends) - the only way to know how many frames
/// exist is to actually walk through decoding all of them, which is
/// exactly what this does.
pub fn count_gif_frames(path: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Could not read '{}': {e}", path.display()))?;
    let decoder = GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("Could not read '{}' as a GIF: {e}", path.display()))?;

    let mut count = 0usize;
    for frame in decoder.into_frames() {
        frame.map_err(|e| format!("Could not decode a frame in '{}': {e}", path.display()))?;
        count += 1;
    }
    Ok(count)
}

/// Extracts one specific frame (1-indexed - "frame 1" means the first
/// frame, matching how someone would naturally describe it, not the
/// zero-based indexing used elsewhere in this codebase for --select) from
/// the GIF at `path`. Returns a clear, bounds-checked error - naming the
/// GIF's actual frame count - if `frame_number` doesn't exist, the same
/// way --select reports an out-of-range --index.
pub fn extract_gif_frame(path: &Path, frame_number: usize) -> Result<image::RgbaImage, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Could not read '{}': {e}", path.display()))?;
    extract_gif_frame_from(
        BufReader::new(file),
        &path.display().to_string(),
        frame_number,
    )
}

/// `extract_gif_frame` for a GIF that is already in memory (read from
/// standard input, say).
pub fn extract_gif_frame_from_bytes(
    bytes: &[u8],
    frame_number: usize,
) -> Result<image::RgbaImage, String> {
    extract_gif_frame_from(Cursor::new(bytes), "standard input", frame_number)
}

/// The shared work of both: `name` is how the GIF is called in messages.
fn extract_gif_frame_from<R: BufRead + Seek>(
    reader: R,
    name: &str,
    frame_number: usize,
) -> Result<image::RgbaImage, String> {
    if frame_number == 0 {
        return Err(
            "--gif-frame must be 1 or greater (frames are numbered starting at 1).".to_string(),
        );
    }

    let decoder =
        GifDecoder::new(reader).map_err(|e| format!("Could not read '{name}' as a GIF: {e}"))?;

    // A single pass over the frames: decoding happens once per frame
    // regardless, so counting the total (for a helpful error message if
    // frame_number is out of range) costs nothing extra on top of finding
    // the requested one.
    let mut selected: Option<image::RgbaImage> = None;
    let mut total = 0usize;
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame = frame.map_err(|e| format!("Could not decode a frame in '{name}': {e}"))?;
        total += 1;
        if index + 1 == frame_number {
            selected = Some(frame.into_buffer());
        }
    }

    selected.ok_or_else(|| {
        format!(
            "--gif-frame {frame_number} is out of range - '{name}' has {total} frame(s), so valid values are 1..{total}."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::GifEncoder;
    use image::{Frame, Rgba, RgbaImage};

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    /// Writes a GIF with one 8x8 solid-color frame per given color.
    fn write_gif(path: &Path, colors: &[[u8; 3]]) {
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = GifEncoder::new(file);
        for c in colors {
            let frame = RgbaImage::from_pixel(8, 8, Rgba([c[0], c[1], c[2], 255]));
            encoder.encode_frame(Frame::new(frame)).unwrap();
        }
    }

    fn write_png(path: &Path) {
        RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]))
            .save_with_format(path, image::ImageFormat::Png)
            .unwrap();
    }

    /// GIF palette quantization may shift a color slightly, so compare the
    /// dominant channel instead of exact values.
    fn dominant_channel(img: &RgbaImage) -> usize {
        let p = img.get_pixel(4, 4);
        (0..3).max_by_key(|&c| p[c]).unwrap()
    }

    // --- is_gif ------------------------------------------------------------------

    #[test]
    fn a_gif_file_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.gif");
        write_gif(&path, &[RED]);
        assert_eq!(is_gif(&path), Ok(true));
    }

    #[test]
    fn a_png_is_not_a_gif() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        write_png(&path);
        assert_eq!(is_gif(&path), Ok(false));
    }

    #[test]
    fn detection_uses_the_content_not_the_extension() {
        let dir = tempfile::tempdir().unwrap();
        let png_named_gif = dir.path().join("fake.gif");
        write_png(&png_named_gif);
        assert_eq!(is_gif(&png_named_gif), Ok(false));

        let gif_named_png = dir.path().join("real.png");
        write_gif(&gif_named_png, &[RED]);
        assert_eq!(is_gif(&gif_named_png), Ok(true));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = is_gif(&dir.path().join("missing.gif")).unwrap_err();
        assert!(err.contains("Could not read"), "unexpected message: {err}");
    }

    // --- count_gif_frames --------------------------------------------------------

    #[test]
    fn frame_count_matches_the_number_of_frames() {
        let dir = tempfile::tempdir().unwrap();
        for count in [1usize, 2, 5] {
            let path = dir.path().join(format!("{count}.gif"));
            write_gif(&path, &vec![RED; count]);
            assert_eq!(count_gif_frames(&path), Ok(count));
        }
    }

    #[test]
    fn counting_frames_of_a_non_gif_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        write_png(&path);
        assert!(count_gif_frames(&path).unwrap_err().contains("as a GIF"));
    }

    // --- extract_gif_frame -------------------------------------------------------

    #[test]
    fn each_frame_can_be_selected_by_its_one_based_number() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("three.gif");
        write_gif(&path, &[RED, GREEN, BLUE]);

        assert_eq!(dominant_channel(&extract_gif_frame(&path, 1).unwrap()), 0);
        assert_eq!(dominant_channel(&extract_gif_frame(&path, 2).unwrap()), 1);
        assert_eq!(dominant_channel(&extract_gif_frame(&path, 3).unwrap()), 2);
    }

    #[test]
    fn extracted_frame_has_the_gif_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("one.gif");
        write_gif(&path, &[RED]);
        assert_eq!(extract_gif_frame(&path, 1).unwrap().dimensions(), (8, 8));
    }

    #[test]
    fn frame_zero_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("one.gif");
        write_gif(&path, &[RED]);
        let err = extract_gif_frame(&path, 0).unwrap_err();
        assert!(err.contains("1 or greater"), "unexpected message: {err}");
    }

    #[test]
    fn a_frame_beyond_the_end_reports_the_real_frame_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("three.gif");
        write_gif(&path, &[RED, GREEN, BLUE]);
        let err = extract_gif_frame(&path, 4).unwrap_err();
        assert!(err.contains("out of range"), "unexpected message: {err}");
        assert!(err.contains("3 frame(s)"), "unexpected message: {err}");
        assert!(err.contains("1..3"), "unexpected message: {err}");
    }

    #[test]
    fn extracting_from_a_non_gif_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        write_png(&path);
        assert!(extract_gif_frame(&path, 1).is_err());
    }

    #[test]
    fn extracting_from_a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(extract_gif_frame(&dir.path().join("none.gif"), 1).is_err());
    }
}
