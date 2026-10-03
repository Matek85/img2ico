// SVG (and gzip-compressed .svgz) as a source: a vector drawing is not
// scaled down from one big picture like a raster image, it is rendered anew
// at every icon size. That gives the sharpest small icons there are - the
// strokes of a 16x16 icon are drawn as 16x16 pixels, not averaged out of a
// 1024x1024 bitmap.
//
// Parsing and drawing are done by the pure-Rust `resvg` family (usvg parses,
// resvg draws). Two things worth knowing about what it is allowed to do:
//   - No network and no files: only images embedded in the SVG itself (data:
//     URIs) are drawn. Nothing the SVG links to is ever fetched or read.
//   - Text is drawn with the fonts installed on THIS computer, so an SVG that
//     contains <text> can look different on another machine. Paths, shapes
//     and gradients are drawn identically everywhere.

use crate::layout::{FitMode, Layout, content_edge, round_corners};
use image::RgbaImage;
use resvg::{tiny_skia, usvg};

/// The first bytes of a gzip stream - what an .svgz file starts with.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// How far into a file to look for the `<svg` tag when deciding whether it is
/// an SVG (an XML declaration, a doctype and comments may come first).
const SNIFF_LIMIT: usize = 4096;

/// Whether `bytes` look like an SVG document (or a gzip-compressed one): the
/// XML text contains an `<svg` tag near its start. The decision is made on
/// the content, like for every other format, so a `logo.xml` or a file without
/// extension is found too.
pub fn is_svg(bytes: &[u8]) -> bool {
    // A gzip stream is taken for an .svgz; if it is not one, parsing says so.
    bytes.starts_with(&GZIP_MAGIC) || looks_like_svg_text(bytes)
}

/// Whether the leading text is XML with an `<svg` element.
fn looks_like_svg_text(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(SNIFF_LIMIT)];
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(head);
    // Quick check first: it has to be text that starts like markup.
    if head.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'<') {
        return false;
    }
    head.windows(4).any(|window| window == b"<svg")
}

/// A parsed SVG, ready to be drawn at any size.
pub struct VectorImage {
    tree: Box<usvg::Tree>,
}

impl VectorImage {
    /// Parses SVG (or .svgz) bytes. `name` is how the file is called in
    /// messages.
    pub fn parse(bytes: &[u8], name: &str) -> Result<Self, String> {
        #[cfg_attr(not(feature = "system-fonts"), allow(unused_mut))]
        let mut options = usvg::Options::default();
        // Fonts for <text>: the ones installed on this computer. Looking
        // through them takes a few hundred milliseconds, so it is done only
        // when the drawing has text at all (a compressed one is not looked
        // into, it gets the fonts to be safe). Without the "system-fonts"
        // feature (the web version: a browser has no fonts to read) text is
        // laid out with no fonts at all, so it does not show.
        #[cfg(feature = "system-fonts")]
        if bytes.starts_with(&GZIP_MAGIC) || bytes.windows(5).any(|w| w == b"<text") {
            options.fontdb_mut().load_system_fonts();
        }
        // Embedded images (data: URIs) are drawn; everything the file points
        // to outside itself is not - there is no `resources_dir` to look in.
        let tree = usvg::Tree::from_data(bytes, &options).map_err(|e| {
            crate::msg!(
                "svg.unreadable",
                "Could not read '{name}' as an SVG file: {e}",
                name = name,
                e = e
            )
        })?;
        let size = tree.size();
        if !(size.width() > 0.0 && size.height() > 0.0) {
            return Err(crate::msg!(
                "svg.no_size",
                "The SVG '{name}' has no size.",
                name = name
            ));
        }
        Ok(Self {
            tree: Box::new(tree),
        })
    }

    /// The drawing's own size in SVG units (usually pixels).
    pub fn size(&self) -> (f32, f32) {
        let size = self.tree.size();
        (size.width(), size.height())
    }

    /// The area of the drawing that actually has something in it, in SVG
    /// units: (x, y, width, height). What --trim fits to.
    fn content_area(&self) -> Option<(f32, f32, f32, f32)> {
        let bounds = self.tree.root().abs_layer_bounding_box();
        Some((bounds.x(), bounds.y(), bounds.width(), bounds.height()))
    }

    /// Draws the SVG into a `size` x `size` square the way `layout` says:
    /// fitted inside the area left by the padding or (with `FitMode::Cover`)
    /// filling it, centered, on a transparent background, with the corners
    /// rounded if asked. With `trim` the drawing's empty margin is left out,
    /// so its content, not its page, is what gets fitted. This is the same
    /// layout a raster image gets (see layout.rs), but drawn sharp at this
    /// size instead of scaled.
    pub fn render(&self, size: u32, layout: &Layout, trim: bool) -> RgbaImage {
        let (page_w, page_h) = self.size();
        let (source_x, source_y, source_w, source_h) = if trim {
            self.content_area().unwrap_or((0.0, 0.0, page_w, page_h))
        } else {
            (0.0, 0.0, page_w, page_h)
        };

        let content = content_edge(size, layout.padding);
        let (scale, placed_w, placed_h) = match layout.fit {
            FitMode::Contain => {
                let scale = (content / source_w).min(content / source_h);
                (scale, source_w * scale, source_h * scale)
            }
            FitMode::Cover => {
                let edge = content.round().max(1.0);
                let scale = (edge / source_w).max(edge / source_h);
                (scale, edge, edge)
            }
        };

        // The picture is drawn into its own pixmap - the placed rectangle,
        // rounded up to whole pixels - and then laid on the canvas.
        let (pixmap_w, pixmap_h) = (
            (placed_w.round() as u32).max(1),
            (placed_h.round() as u32).max(1),
        );
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_w, pixmap_h)
            .expect("a pixmap of at least 1x1 pixel is valid");
        // Centered in the pixmap: for cover the overhang is cut off by its edges.
        let offset_x = (pixmap_w as f32 - source_w * scale) / 2.0;
        let offset_y = (pixmap_h as f32 - source_h * scale) / 2.0;
        let transform = tiny_skia::Transform::from_translate(-source_x, -source_y)
            .post_scale(scale, scale)
            .post_translate(offset_x, offset_y);
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());

        // tiny-skia keeps premultiplied alpha; icons are straight alpha.
        let mut placed = RgbaImage::from_raw(pixmap_w, pixmap_h, pixmap.take_demultiplied())
            .expect("a pixmap has width x height x 4 bytes");
        round_corners(&mut placed, 0, 0, pixmap_w, pixmap_h, layout.corner_radius);

        let mut canvas = RgbaImage::new(size, size);
        image::imageops::overlay(
            &mut canvas,
            &placed,
            i64::from(size.saturating_sub(pixmap_w) / 2),
            i64::from(size.saturating_sub(pixmap_h) / 2),
        );
        canvas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(padding: u8) -> Layout {
        Layout {
            padding,
            ..Layout::default()
        }
    }

    const TWO_SQUARES: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="50" height="50" fill="#ff0000"/><rect x="50" width="50" height="50" fill="#00ff00"/></svg>"##;

    #[test]
    fn svg_is_recognized_from_the_content() {
        assert!(is_svg(TWO_SQUARES.as_bytes()));
        assert!(is_svg(
            b"\xEF\xBB\xBF  \n<?xml version=\"1.0\"?>\n<!-- c -->\n<svg xmlns=\"x\"/>"
        ));
        assert!(is_svg(&[0x1f, 0x8b, 8, 0]), "a gzip stream may be an .svgz");
        assert!(!is_svg(b"\x89PNG\r\n"));
        assert!(!is_svg(b"<html><body>no vector here</body></html>"));
        assert!(!is_svg(b"plain text with <svg inside"));
        assert!(!is_svg(b""));
    }

    #[test]
    fn the_size_is_the_drawings_own() {
        let drawing = VectorImage::parse(TWO_SQUARES.as_bytes(), "t.svg").unwrap();
        assert_eq!(drawing.size(), (100.0, 50.0));
    }

    #[test]
    fn a_square_render_fits_the_drawing_and_leaves_transparent_bars() {
        let drawing = VectorImage::parse(TWO_SQUARES.as_bytes(), "t.svg").unwrap();
        let image = drawing.render(32, &plain(0), false);
        assert_eq!(image.dimensions(), (32, 32));
        // 100x50 fitted into 32x32: 32 wide, 16 high, centered vertically.
        assert_eq!(image.get_pixel(4, 16).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(28, 16).0, [0, 255, 0, 255]);
        assert_eq!(image.get_pixel(16, 2).0[3], 0, "above the drawing");
        assert_eq!(image.get_pixel(16, 30).0[3], 0, "below the drawing");
    }

    #[test]
    fn padding_leaves_a_margin_on_all_sides() {
        let drawing = VectorImage::parse(TWO_SQUARES.as_bytes(), "t.svg").unwrap();
        let image = drawing.render(100, &plain(20), false);
        // 80 wide, 40 high, centered: x 10..90, y 30..70.
        assert_eq!(image.get_pixel(5, 50).0[3], 0);
        assert_eq!(image.get_pixel(15, 50).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(50, 25).0[3], 0);
    }

    #[test]
    fn small_sizes_are_drawn_not_scaled_down() {
        // A 2 units wide line in a 16 unit drawing, drawn at 16x16: its edges
        // fall exactly on pixel borders, so there is no blur at all.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect x="7" width="2" height="16" fill="#000"/></svg>"##;
        let drawing = VectorImage::parse(svg.as_bytes(), "t.svg").unwrap();
        let image = drawing.render(16, &plain(0), false);
        assert_eq!(image.get_pixel(7, 8).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(8, 8).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(6, 8).0[3], 0, "crisp edge, no blur");
        assert_eq!(image.get_pixel(9, 8).0[3], 0, "crisp edge, no blur");
    }

    #[test]
    fn broken_drawings_are_errors_naming_the_file() {
        assert!(VectorImage::parse(b"<svg", "t.svg").is_err());
        assert!(VectorImage::parse(b"not xml", "t.svg").is_err());
        let err = VectorImage::parse(b"\x1f\x8bnot gzip", "x.svgz")
            .err()
            .unwrap();
        assert!(err.contains("x.svgz"), "{err}");
    }

    #[test]
    fn a_link_to_an_outside_file_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 255, 255]))
            .save(dir.path().join("secret.png"))
            .unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="8" height="8"><image width="8" height="8" xlink:href="secret.png"/></svg>"#;
        let drawing = VectorImage::parse(svg.as_bytes(), "a.svg").unwrap();
        assert_eq!(
            drawing.render(8, &plain(0), false).get_pixel(4, 4).0[3],
            0,
            "nothing is drawn"
        );
    }
}
