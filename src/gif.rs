// Support for detecting animated GIF input and selecting a specific frame
// from it. image::open() - used everywhere else in this project for
// loading a source image - only ever returns a GIF's FIRST frame (the
// standard, non-animation-aware decode path), with no way to ask for a
// different one and no visibility into how many frames exist at all.
// This module fills that one gap; everything else about a GIF (or any
// other format) still goes through the normal image::open() path.

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
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

/// Decodes every frame of the GIF at `path` and returns the total count -
/// used by --inspect to report it. GIF files don't store a frame count
/// up front (a GIF is just a sequence of image blocks, one after
/// another, until the file ends) - the only way to know how many frames
/// exist is to actually walk through decoding all of them, which is
/// exactly what this does.
pub fn count_gif_frames(path: &Path) -> Result<usize, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Could not read '{}': {e}", path.display()))?;
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
    if frame_number == 0 {
        return Err(
            "--gif-frame must be 1 or greater (frames are numbered starting at 1).".to_string(),
        );
    }

    let file =
        std::fs::File::open(path).map_err(|e| format!("Could not read '{}': {e}", path.display()))?;
    let decoder = GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("Could not read '{}' as a GIF: {e}", path.display()))?;

    // A single pass over the frames: decoding happens once per frame
    // regardless, so counting the total (for a helpful error message if
    // frame_number is out of range) costs nothing extra on top of finding
    // the requested one.
    let mut selected: Option<image::RgbaImage> = None;
    let mut total = 0usize;
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame =
            frame.map_err(|e| format!("Could not decode a frame in '{}': {e}", path.display()))?;
        total += 1;
        if index + 1 == frame_number {
            selected = Some(frame.into_buffer());
        }
    }

    selected.ok_or_else(|| {
        format!(
            "--gif-frame {frame_number} is out of range - '{}' has {total} frame(s), so valid values are 1..{total}.",
            path.display()
        )
    })
}