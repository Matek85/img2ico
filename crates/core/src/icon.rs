// Reading the pictures inside an .ico file back out as pixels - to look at
// them exactly as stored (the page's pixel inspector), and the groundwork for
// extracting and merging icons.

use std::io::Cursor;

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
    let dir = ico::IconDir::read(Cursor::new(bytes))
        .map_err(|e| format!("Could not read the icon file: {e}"))?;
    Ok(dir.entries().len())
}

/// Decodes image number `index` (counted from 0, in the order of the file's
/// directory) of the .ico file in `bytes`.
pub fn read_entry(bytes: &[u8], index: usize) -> Result<IconImage, String> {
    let dir = ico::IconDir::read(Cursor::new(bytes))
        .map_err(|e| format!("Could not read the icon file: {e}"))?;
    let entry = dir.entries().get(index).ok_or_else(|| {
        format!(
            "The icon file has {} image(s); there is no image number {index}.",
            dir.entries().len()
        )
    })?;
    let image = entry
        .decode()
        .map_err(|e| format!("Could not decode image {index}: {e}"))?;
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
