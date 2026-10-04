//! The browser-facing side of img2ico: a few functions and one class that the
//! web page calls (from a Web Worker) with plain bytes and text, and that
//! return plain bytes and text. No logic lives here - it only translates
//! between JavaScript and img2ico-core.

use image::RgbaImage;
use img2ico_core::archive::{ZipArchive, write_zip};
use img2ico_core::convert::{
    Background, BackgroundMode, DEFAULT_SIZES, Format, NoNotes, Options, Sizes, analyze_raster,
    convert_raster, convert_vector, render_raster_original, render_vector_original,
};
use img2ico_core::diag;
use img2ico_core::export::{ImageKind, encode_image};
use img2ico_core::favicon::{head_snippet, manifest};
use img2ico_core::icon::{
    alpha_share, alpha_summary, entry_png, merge, missing_windows_sizes, read_dir, read_entry,
    select, unique_file_name, write_dir,
};
use img2ico_core::layout::{CropRect, FitMode, Layout, MAX_CORNER_RADIUS};
use img2ico_core::msg;
use img2ico_core::source::{
    Artwork, GifFrame, decode_gif_frames_from_bytes, decode_source_bytes,
    extract_gif_frame_from_bytes, is_gif_bytes,
};
use img2ico_core::validate::{Severity, validate_bytes};
use img2ico_core::vector::{VectorImage, is_svg};
use serde_json::{Map, Value};
use wasm_bindgen::prelude::*;

/// The most pixels a picture may have in the browser: 40 million, a
/// 6300 x 6300 image. A page has far less memory to work with than a program
/// does, and the conversion keeps a few copies of the picture.
const WEB_MAX_PIXELS: u64 = 40_000_000;

/// Remembers the code and values of every message, so the page can be handed them.
#[wasm_bindgen(start)]
fn start() {
    img2ico_core::msg::remember_messages();
}

/// A message as the page gets it: `{ code, params, text }`. `params` hold text, or - where a value
/// is itself a message (the reason inside "could not read ...") - the same structure again.
/// A sentence that has no code of its own comes as `{ code: "other", params: {}, text }`.
fn structured(text: &str) -> Value {
    let Some(known) = msg::lookup(text) else {
        return serde_json::json!({ "code": "other", "params": {}, "text": text });
    };
    let params: Map<String, Value> = known
        .params
        .iter()
        .map(|(name, value)| {
            let shown = if msg::lookup(value).is_some() {
                structured(value)
            } else {
                Value::String(value.clone())
            };
            ((*name).to_string(), shown)
        })
        .collect();
    serde_json::json!({ "code": known.code, "params": params, "text": text })
}

/// The error the page gets for a message: its structure as JSON text.
fn failure(message: String) -> JsError {
    JsError::new(&structured(&message).to_string())
}

/// The version of the engine, shown in the page footer.
#[wasm_bindgen]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Checks the bytes of an .ico file and returns the result as JSON text:
/// `{ valid, bytes, images: [...], errors: [...], warnings: [...] }`, the same
/// shape as `img2ico --validate --json` without the file path.
#[wasm_bindgen]
pub fn validate_ico(bytes: &[u8]) -> String {
    let report = validate_bytes(bytes);
    let findings = |severity| -> Vec<serde_json::Value> {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| {
                serde_json::json!({
                    "image": f.entry,
                    "message": f.message,
                    "info": structured(&f.message),
                })
            })
            .collect()
    };
    let images: Vec<serde_json::Value> = report
        .entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "index": e.index,
                "width": e.width,
                "height": e.height,
                "bits_per_pixel": e.bits_per_pixel,
                "format": e.format.name(),
                "offset": e.offset,
                "size": e.size,
            })
        })
        .collect();
    serde_json::json!({
        "valid": report.is_valid(),
        "bytes": report.file_size,
        "images": images,
        "errors": findings(Severity::Error),
        "warnings": findings(Severity::Warning),
    })
    .to_string()
}

/// The website icon package: a ZIP of the files a site needs, and the lines
/// for its `<head>`.
#[wasm_bindgen]
pub struct FaviconPack {
    zip: Vec<u8>,
    snippet: String,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl FaviconPack {
    /// The ZIP.
    pub fn zip(&self) -> Vec<u8> {
        self.zip.clone()
    }

    /// The lines to paste into the page's `<head>`.
    pub fn snippet(&self) -> String {
        self.snippet.clone()
    }

    /// The warnings raised while making the pictures, as JSON text (a list of messages, see `structured`).
    pub fn warnings(&self) -> String {
        Value::Array(self.warnings.iter().map(|w| structured(w)).collect()).to_string()
    }
}

/// The lines for a page's `<head>` that go with the website icon package.
#[wasm_bindgen]
pub fn favicon_snippet(has_svg: bool, theme_color: &str) -> String {
    head_snippet(has_svg, theme_color)
}

/// One image of an icon file, decoded to pixels.
#[wasm_bindgen]
pub struct IconPixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl IconPixels {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Four bytes per pixel (red, green, blue, alpha - not premultiplied), row
    /// by row from the top left.
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
}

/// Decodes image number `index` (from 0) of an .ico file to its exact pixels.
#[wasm_bindgen]
pub fn icon_pixels(bytes: &[u8], index: usize) -> Result<IconPixels, JsError> {
    read_entry(bytes, index)
        .map(|image| IconPixels {
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        })
        .map_err(failure)
}

/// Describes the images of an .ico file as JSON text:
/// `{ images: [{ index, width, height, bits_per_pixel, format, bytes, alpha,
/// non_opaque_share }], missing_windows_sizes: [...] }`.
#[wasm_bindgen]
pub fn icon_describe(bytes: &[u8]) -> Result<String, JsError> {
    describe_icon(bytes).map_err(failure)
}

/// Image number `index` (from 0) of an .ico file as the bytes of a PNG file.
#[wasm_bindgen]
pub fn icon_extract_png(bytes: &[u8], index: usize) -> Result<Vec<u8>, JsError> {
    extract_png(bytes, index).map_err(failure)
}

/// A new .ico file with only the images at `indices` (in that order).
#[wasm_bindgen]
pub fn icon_select(bytes: &[u8], indices: &[u32]) -> Result<Vec<u8>, JsError> {
    select_images(bytes, indices).map_err(failure)
}

/// A ZIP file that has been opened: its pictures can be listed and unpacked
/// one at a time.
#[wasm_bindgen]
pub struct ZipReader {
    archive: ZipArchive,
}

#[wasm_bindgen]
impl ZipReader {
    /// Reads the table of contents of the ZIP in `bytes`.
    pub fn open(bytes: &[u8]) -> Result<ZipReader, JsError> {
        ZipArchive::open(bytes.to_vec())
            .map(|archive| ZipReader { archive })
            .map_err(failure)
    }

    /// The files in the ZIP as JSON text: `[{ index, name, size }]`. Folders,
    /// macOS resource forks and hidden files are left out.
    pub fn files(&self) -> String {
        let files: Vec<Value> = self
            .archive
            .files()
            .into_iter()
            .map(|f| serde_json::json!({ "index": f.index, "name": f.name, "size": f.size }))
            .collect();
        Value::from(files).to_string()
    }

    /// Unpacks the file with this `index` (as listed by `files`).
    pub fn read(&self, index: usize) -> Result<Vec<u8>, JsError> {
        self.archive.read(index).map_err(failure)
    }
}

/// Collects files and writes them into a ZIP.
#[wasm_bindgen]
pub struct ZipBuilder {
    files: Vec<(String, Vec<u8>)>,
}

#[wasm_bindgen]
impl ZipBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ZipBuilder {
        ZipBuilder { files: Vec::new() }
    }

    pub fn add(&mut self, name: &str, bytes: &[u8]) {
        self.files.push((name.to_string(), bytes.to_vec()));
    }

    pub fn finish(&self) -> Result<Vec<u8>, JsError> {
        write_zip(&self.files).map_err(failure)
    }
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Every image of an .ico file as a PNG, in a ZIP, named `<stem>_<w>x<h>.png`.
#[wasm_bindgen]
pub fn icon_png_zip(bytes: &[u8], stem: &str) -> Result<Vec<u8>, JsError> {
    png_zip(bytes, stem).map_err(failure)
}

/// `icon_png_zip` without the JavaScript error type.
pub fn png_zip(bytes: &[u8], stem: &str) -> Result<Vec<u8>, String> {
    let dir = read_dir(bytes, "the icon file")?;
    if dir.entries().is_empty() {
        return Err(img2ico_core::msg!(
            "icon.no_images",
            "The icon file has no images."
        ));
    }
    let mut names: Vec<String> = Vec::new();
    let mut files = Vec::new();
    for entry in dir.entries() {
        let name = unique_file_name(&names, stem, entry.width(), entry.height(), "png");
        files.push((name.clone(), entry_png(entry, "the icon file")?));
        names.push(name);
    }
    write_zip(&files)
}

/// The images of an .ico file as files of another kind (`png`, `jpg`, `webp`, `bmp`, `tiff`), in a ZIP, named
/// `<stem>_<w>x<h>.<ending>`. A kind without transparency (jpg, bmp) lays the images on `background` (`#rrggbb`).
#[wasm_bindgen]
pub fn icon_image_zip(
    bytes: &[u8],
    stem: &str,
    kind: &str,
    background: &str,
) -> Result<Vec<u8>, JsError> {
    image_zip(bytes, stem, kind, background).map_err(failure)
}

/// `icon_image_zip` without the JavaScript error type.
pub fn image_zip(
    bytes: &[u8],
    stem: &str,
    kind: &str,
    background: &str,
) -> Result<Vec<u8>, String> {
    let kind = image_kind(kind)?;
    let background = background_color(background)?;
    let dir = read_dir(bytes, "the icon file")?;
    if dir.entries().is_empty() {
        return Err(img2ico_core::msg!(
            "icon.no_images",
            "The icon file has no images."
        ));
    }
    let mut names: Vec<String> = Vec::new();
    let mut files = Vec::new();
    for entry in dir.entries() {
        let name = unique_file_name(
            &names,
            stem,
            entry.width(),
            entry.height(),
            kind.extension(),
        );
        files.push((name.clone(), entry_image(entry, kind, background)?));
        names.push(name);
    }
    write_zip(&files)
}

/// One image of an .ico file (the one at `index`) as a file of another kind (see `icon_image_zip`).
#[wasm_bindgen]
pub fn icon_image(
    bytes: &[u8],
    index: usize,
    kind: &str,
    background: &str,
) -> Result<Vec<u8>, JsError> {
    image_of_icon(bytes, index, kind, background).map_err(failure)
}

/// `icon_image` without the JavaScript error type.
pub fn image_of_icon(
    bytes: &[u8],
    index: usize,
    kind: &str,
    background: &str,
) -> Result<Vec<u8>, String> {
    let kind = image_kind(kind)?;
    let background = background_color(background)?;
    let dir = read_dir(bytes, "the icon file")?;
    let entry = dir.entries().get(index).ok_or_else(|| {
        img2ico_core::msg!(
            "icon.no_such_image",
            "The icon file has {count} image(s); there is no image number {index}.",
            count = dir.entries().len(),
            index = index
        )
    })?;
    entry_image(entry, kind, background)
}

fn entry_image(
    entry: &ico::IconDirEntry,
    kind: ImageKind,
    background: [u8; 3],
) -> Result<Vec<u8>, String> {
    let png = entry_png(entry, "the icon file")?;
    if kind == ImageKind::Png {
        return Ok(png);
    }
    let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    encode_image(&image, kind, background)
}

fn image_kind(text: &str) -> Result<ImageKind, String> {
    ImageKind::parse(text).ok_or_else(|| {
        img2ico_core::msg!(
            "export.unknown_type",
            "{kind} is not an image type this page can make.",
            kind = text
        )
    })
}

fn background_color(text: &str) -> Result<[u8; 3], String> {
    img2ico_core::chroma_key::parse_hex_color(text)
        .map_err(|e| format!("Invalid background color: {e}"))
}

/// Collects several .ico files and merges them into one. If two files hold the
/// same size, the first wins and the others are reported as warnings.
#[wasm_bindgen]
pub struct Merger {
    files: Vec<(String, ico::IconDir)>,
}

#[wasm_bindgen]
impl Merger {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Merger {
        Merger { files: Vec::new() }
    }

    /// Adds an .ico file. `name` is how it is called in messages.
    pub fn add(&mut self, bytes: &[u8], name: &str) -> Result<(), JsError> {
        let dir = read_dir(bytes, name).map_err(failure)?;
        self.files.push((name.to_string(), dir));
        Ok(())
    }

    /// The merged file; `warnings` of the result say which images were left out.
    pub fn merge(&self) -> Result<Output, JsError> {
        self.merge_files().map_err(failure)
    }
}

impl Default for Merger {
    fn default() -> Self {
        Self::new()
    }
}

impl Merger {
    /// `merge` without the JavaScript error type, so it can be tested.
    pub fn merge_files(&self) -> Result<Output, String> {
        if self.files.len() < 2 {
            return Err("Merging needs at least two icon files.".to_string());
        }
        let sources: Vec<(&str, &ico::IconDir)> = self
            .files
            .iter()
            .map(|(name, dir)| (name.as_str(), dir))
            .collect();
        let (merged, skipped) = merge(&sources)?;
        Ok(Output {
            sizes: merged.entries().iter().map(|e| e.width()).collect(),
            bytes: write_dir(&merged)?,
            warnings: skipped
                .iter()
                .map(|image| {
                    img2ico_core::msg!(
                        "icon.merge_skipped",
                        "Skipped {width}x{height} from '{source}': that size is already in the merged file.",
                        width = image.width,
                        height = image.height,
                        source = image.source
                    )
                })
                .collect(),
        })
    }
}

/// `icon_describe` without the JavaScript error type.
pub fn describe_icon(bytes: &[u8]) -> Result<String, String> {
    let dir = read_dir(bytes, "the icon file")?;
    let present: std::collections::HashSet<u32> = dir.entries().iter().map(|e| e.width()).collect();
    let images: Vec<Value> = dir
        .entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            serde_json::json!({
                "index": index,
                "width": entry.width(),
                "height": entry.height(),
                "bits_per_pixel": entry.bits_per_pixel(),
                "format": if entry.is_png() { "PNG" } else { "BMP" },
                "bytes": entry.data().len(),
                "alpha": alpha_summary(entry),
                "non_opaque_share": alpha_share(entry),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "images": images,
        "missing_windows_sizes": missing_windows_sizes(&present),
    })
    .to_string())
}

/// `icon_extract_png` without the JavaScript error type.
pub fn extract_png(bytes: &[u8], index: usize) -> Result<Vec<u8>, String> {
    let dir = read_dir(bytes, "the icon file")?;
    let entry = dir.entries().get(index).ok_or_else(|| {
        img2ico_core::msg!(
            "icon.no_such_image",
            "The icon file has {count} image(s); there is no image number {index}.",
            count = dir.entries().len(),
            index = index
        )
    })?;
    entry_png(entry, "the icon file")
}

/// `icon_select` without the JavaScript error type.
pub fn select_images(bytes: &[u8], indices: &[u32]) -> Result<Vec<u8>, String> {
    if indices.is_empty() {
        return Err(img2ico_core::msg!(
            "select.none_chosen",
            "Choose at least one image."
        ));
    }
    let dir = read_dir(bytes, "the icon file")?;
    let indices: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
    write_dir(&select(&dir, &indices, "the icon file")?)
}

/// The frames of an animated GIF, held in memory so the page can show them one
/// after another and choose one (counted from 0 here).
#[wasm_bindgen]
pub struct GifFrames {
    frames: Vec<GifFrame>,
}

#[wasm_bindgen]
impl GifFrames {
    /// Decodes every frame of the GIF in `bytes`.
    pub fn open(bytes: &[u8], name: &str) -> Result<GifFrames, JsError> {
        decode_gif_frames_from_bytes(bytes, name, WEB_MAX_PIXELS)
            .map(|frames| GifFrames { frames })
            .map_err(failure)
    }

    /// How many frames the GIF has.
    pub fn count(&self) -> usize {
        self.frames.len()
    }

    /// The width in pixels, the same for every frame.
    pub fn width(&self) -> u32 {
        self.frames.first().map_or(0, |frame| frame.image.width())
    }

    /// The height in pixels, the same for every frame.
    pub fn height(&self) -> u32 {
        self.frames.first().map_or(0, |frame| frame.image.height())
    }

    /// How long frame `index` is shown, in milliseconds.
    pub fn delay_ms(&self, index: usize) -> u32 {
        self.frames.get(index).map_or(100, |frame| frame.delay_ms)
    }

    /// Frame `index` as a PNG, to show it.
    pub fn frame_png(&self, index: usize) -> Result<Vec<u8>, JsError> {
        gif_frame_png(&self.frames, index).map_err(failure)
    }
}

/// `GifFrames::frame_png` without the JavaScript error type.
pub fn gif_frame_png(frames: &[GifFrame], index: usize) -> Result<Vec<u8>, String> {
    use image::{DynamicImage, ImageFormat};
    use std::io::Cursor;
    let frame = frames.get(index).ok_or_else(|| {
        img2ico_core::msg!(
            "gif.no_such_frame",
            "The GIF has {count} frame(s); there is no frame number {number}.",
            count = frames.len(),
            number = index + 1
        )
    })?;
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(frame.image.clone())
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}

/// A picture that has been opened and can be converted again and again with
/// different options, without decoding it each time.
#[wasm_bindgen]
pub struct Source {
    picture: Picture,
    /// The text of an SVG, kept for the website icon package.
    svg: Option<Vec<u8>>,
}

pub enum Picture {
    Raster(RgbaImage),
    Vector(VectorImage),
}

/// The finished icon file and what came with it.
#[wasm_bindgen]
pub struct Output {
    bytes: Vec<u8>,
    sizes: Vec<u32>,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl Source {
    /// Opens the bytes of an image (PNG, JPEG, GIF, WebP, BMP, TIFF, .icns or
    /// SVG). `name` is how the file is called in messages; for a GIF,
    /// `gif_frame` picks the frame (counted from 1).
    pub fn open(bytes: &[u8], name: &str, gif_frame: usize) -> Result<Source, JsError> {
        open_source(bytes, name, gif_frame)
            .map(|picture| Source {
                svg: (matches!(picture, Picture::Vector(_))
                    && is_svg(bytes)
                    && !bytes.starts_with(&[0x1f, 0x8b]))
                .then(|| bytes.to_vec()),
                picture,
            })
            .map_err(failure)
    }

    /// The picture that is frame `index` (from 0) of an animated GIF that was
    /// read with `GifFrames::open`.
    pub fn from_gif_frame(frames: &GifFrames, index: usize) -> Result<Source, JsError> {
        frames
            .frames
            .get(index)
            .map(|frame| Source {
                picture: Picture::Raster(frame.image.clone()),
                svg: None,
            })
            .ok_or_else(|| JsError::new("That frame does not exist."))
    }

    /// The width in pixels (for an SVG: its width in units, rounded).
    pub fn width(&self) -> u32 {
        self.dimensions().0
    }

    /// The height in pixels (for an SVG: its height in units, rounded).
    pub fn height(&self) -> u32 {
        self.dimensions().1
    }

    /// Whether this is a drawing (SVG), which is drawn anew at every size.
    pub fn is_vector(&self) -> bool {
        matches!(self.picture, Picture::Vector(_))
    }

    /// The picture mirrored (if asked) and turned clockwise by `degrees` as a PNG, for the crop view: at most `max_edge`
    /// pixels on its longer side (0: whole size). Its pixels are placed exactly as in the icon, so
    /// the page shows the crop frame on what the icon is made from.
    pub fn rotated_preview(
        &self,
        degrees: i32,
        flip_horizontal: bool,
        flip_vertical: bool,
        max_edge: u32,
    ) -> Result<Vec<u8>, JsError> {
        self.preview_with(degrees, flip_horizontal, flip_vertical, max_edge)
            .map_err(failure)
    }

    /// Makes the website icon package from the options (JSON text, see
    /// `parse_options`; the sizes and the format are decided here) and
    /// `meta`: `{ "name", "themeColor", "appleBackground" }`.
    pub fn favicon_pack(&self, options: &str, meta: &str) -> Result<FaviconPack, JsError> {
        self.pack_with(options, meta).map_err(failure)
    }

    /// The picture as it was edited (the options, JSON text, see `parse_options`: the background removed, mirrored,
    /// turned, cropped, trimmed, black and white; not the margin, corners or sizes of an icon), in its own size,
    /// as an image file of the kind `png`, `jpg`, `webp`, `bmp` or `tiff`. `flatten` in the options is the color
    /// jpg and bmp lay it on (white if there is none). The output's sizes are its width and height.
    pub fn image(&self, options: &str, kind: &str) -> Result<Output, JsError> {
        self.image_with(options, kind).map_err(failure)
    }

    /// Makes the icon file the options (JSON text, see `parse_options`) ask for.
    pub fn convert(&self, options: &str) -> Result<Output, JsError> {
        self.convert_with(options).map_err(failure)
    }
}

#[wasm_bindgen]
impl Output {
    /// The icon file.
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// The edge lengths in the file, in order.
    pub fn sizes(&self) -> Vec<u32> {
        self.sizes.clone()
    }

    /// The warnings raised while converting, as JSON text (a list of messages, see `structured`).
    pub fn warnings(&self) -> String {
        Value::Array(self.warnings.iter().map(|w| structured(w)).collect()).to_string()
    }
}

impl Source {
    fn dimensions(&self) -> (u32, u32) {
        match &self.picture {
            Picture::Raster(image) => image.dimensions(),
            Picture::Vector(drawing) => {
                let (w, h) = drawing.size();
                (w.round() as u32, h.round() as u32)
            }
        }
    }

    /// `rotated_preview` without the JavaScript error type.
    pub fn preview_with(
        &self,
        degrees: i32,
        flip_horizontal: bool,
        flip_vertical: bool,
        max_edge: u32,
    ) -> Result<Vec<u8>, String> {
        use image::{DynamicImage, ImageFormat, imageops};
        let Picture::Raster(image) = &self.picture else {
            return Err(img2ico_core::msg!(
                "rotate.vector",
                "A drawing (SVG) cannot be turned: it is drawn anew at every size."
            ));
        };
        let (width, height) = image.dimensions();
        let (canvas_w, canvas_h) = img2ico_core::rotate::rotated_size(width, height, degrees);
        let longest = canvas_w.max(canvas_h);
        let mut base = if max_edge > 0 && longest > max_edge {
            let scale = f64::from(max_edge) / f64::from(longest);
            imageops::resize(
                image,
                ((f64::from(width) * scale).round() as u32).max(1),
                ((f64::from(height) * scale).round() as u32).max(1),
                imageops::FilterType::Triangle,
            )
        } else {
            image.clone()
        };
        img2ico_core::rotate::flip(&mut base, flip_horizontal, flip_vertical);
        let turned = img2ico_core::rotate::rotate(&base, degrees)?;
        let mut bytes = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(turned)
            .write_to(&mut bytes, ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(bytes.into_inner())
    }

    /// `convert` without the JavaScript error type, so it can be tested.
    pub fn convert_with(&self, options_json: &str) -> Result<Output, String> {
        self.run(&parse_options(options_json)?)
    }

    /// `image` without the JavaScript error type, so it can be tested.
    pub fn image_with(&self, options_json: &str, kind: &str) -> Result<Output, String> {
        let kind = image_kind(kind)?;
        let options = parse_options(options_json)?;
        // The color a kind without transparency lays the picture on; the other kinds keep the transparency.
        let background = options.flatten.unwrap_or([255, 255, 255]);
        let options = Options {
            flatten: None,
            ..options
        };
        let (result, warnings) = diag::collect(|| match &self.picture {
            Picture::Raster(image) => {
                let analysis = analyze_raster(image, &options, &NoNotes)?;
                render_raster_original(image.clone(), &options, &analysis, &NoNotes)
            }
            Picture::Vector(drawing) => render_vector_original(drawing, &options, &NoNotes),
        });
        let picture = result?;
        let (width, height) = picture.dimensions();
        Ok(Output {
            bytes: encode_image(&picture, kind, background)?,
            sizes: vec![width, height],
            warnings,
        })
    }

    /// `favicon_pack` without the JavaScript error type.
    pub fn pack_with(&self, options_json: &str, meta_json: &str) -> Result<FaviconPack, String> {
        let base = parse_options(options_json)?;
        let meta: Value = serde_json::from_str(meta_json)
            .map_err(|e| format!("The package settings are not valid JSON: {e}"))?;
        let text = |key: &str, default: &str| -> String {
            meta.get(key)
                .and_then(Value::as_str)
                .unwrap_or(default)
                .to_string()
        };
        let name = text("name", "");
        let theme = color_text(&text("themeColor", "#ffffff"))?;
        let apple_input = text("appleBackground", "#ffffff");
        let apple_color = img2ico_core::chroma_key::parse_hex_color(&apple_input)
            .map_err(|e| format!("Invalid color for the Apple icon: {e}"))?;
        let apple = color_text(&apple_input)?;

        let variant = |format: Format, size: Vec<u32>, flatten: Option<[u8; 3]>| Options {
            format,
            sizes: Sizes::Fixed(size),
            flatten,
            ..base.clone()
        };
        let png = |size: u32, flatten: Option<[u8; 3]>| variant(Format::Png, vec![size], flatten);

        let jobs: Vec<(&str, Options)> = vec![
            ("favicon.ico", variant(Format::Ico, vec![16, 32, 48], None)),
            ("favicon-16x16.png", png(16, None)),
            ("favicon-32x32.png", png(32, None)),
            ("apple-touch-icon.png", png(180, Some(apple_color))),
            ("icon-192.png", png(192, None)),
            ("icon-512.png", png(512, None)),
        ];
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        for (file_name, options) in jobs {
            let output = self.run(&options)?;
            for warning in output.warnings {
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
            files.push((file_name.to_string(), output.bytes));
        }
        files.push((
            "site.webmanifest".to_string(),
            manifest(&name, &theme, &apple).into_bytes(),
        ));
        if let Some(svg) = &self.svg {
            files.push(("favicon.svg".to_string(), svg.clone()));
        }
        let snippet = head_snippet(self.svg.is_some(), &theme);
        files.push((
            "head-snippet.html".to_string(),
            snippet.clone().into_bytes(),
        ));

        Ok(FaviconPack {
            zip: write_zip(&files)?,
            snippet,
            warnings,
        })
    }

    /// Converts with options that are already parsed.
    pub fn run(&self, options: &Options) -> Result<Output, String> {
        let options = options.clone();
        let (result, warnings) = diag::collect(|| match &self.picture {
            Picture::Raster(image) => {
                let analysis = analyze_raster(image, &options, &NoNotes)?;
                convert_raster(image.clone(), &options, &analysis, &NoNotes)
            }
            Picture::Vector(drawing) => convert_vector(drawing, &options, &NoNotes),
        });
        let converted = result?;
        Ok(Output {
            bytes: converted.bytes,
            sizes: converted.sizes,
            warnings,
        })
    }
}

/// Opens the bytes as a picture - the part of `Source::open` that can be
/// tested without JavaScript.
pub fn open_source(bytes: &[u8], name: &str, gif_frame: usize) -> Result<Picture, String> {
    if is_gif_bytes(bytes) {
        return extract_gif_frame_from_bytes(bytes, name, gif_frame.max(1), WEB_MAX_PIXELS)
            .map(Picture::Raster);
    }
    match decode_source_bytes(bytes, name, WEB_MAX_PIXELS).map_err(|e| {
        img2ico_core::msg!(
            "source.unreadable",
            "Could not read '{name}': {e}",
            name = name,
            e = e
        )
    })? {
        Artwork::Raster(image) => Ok(Picture::Raster(image.into_rgba8())),
        Artwork::Vector(drawing) => Ok(Picture::Vector(drawing)),
    }
}

/// What the page may ask for, as JSON:
///
/// ```text
/// {
///   "format": "ico" | "icns" | "png",         default "ico"; a png has one size
///   "sizes": [16, 32, ...] | "auto",           default 16,32,48,64,128,256
///   "padding": 0-100, "cornerRadius": 0-50,    default 0
///   "fit": "contain" | "cover",                default "contain"
///   "grayscale": bool, "trim": bool,           default false
///   "flipHorizontal": bool, "flipVertical": bool   mirror the picture first (not for an SVG); default false
///   "rotate": 0-359,                           degrees clockwise, before the crop; default 0
///   "flatten": "#rrggbb",                     lay the icon on this color (no transparency)
///   "crop": { "x", "y", "width", "height" },   default none
///   "background": {                            default none
///     "spec": "auto" | "#rrggbb",
///     "find": bool,                            also look for enclosed regions
///     "tolerance": 0-255, "feather": 0-100,
///     "replacement": "#rrggbb",
///     "seeds": [[x, y], ...]
///   }
/// }
/// ```
///
/// Anything else is an error, so a misspelled option does not silently do
/// nothing.
pub fn parse_options(json: &str) -> Result<Options, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| format!("The options are not valid JSON: {e}"))?;
    let Value::Object(map) = value else {
        return Err("The options must be a JSON object.".to_string());
    };
    check_keys(
        &map,
        &[
            "format",
            "sizes",
            "padding",
            "cornerRadius",
            "fit",
            "grayscale",
            "trim",
            "flipHorizontal",
            "flipVertical",
            "rotate",
            "crop",
            "background",
            "flatten",
        ],
        "option",
    )?;

    let format = match text(&map, "format")?.as_deref() {
        None | Some("ico") => Format::Ico,
        Some("icns") => Format::Icns,
        Some("png") => Format::Png,
        Some(other) => {
            return Err(format!(
                "Unknown format '{other}' (use \"ico\", \"icns\" or \"png\")."
            ));
        }
    };
    let sizes = match map.get("sizes") {
        None => Sizes::Fixed(DEFAULT_SIZES.to_vec()),
        Some(Value::String(s)) if s == "auto" => Sizes::Auto(DEFAULT_SIZES.to_vec()),
        Some(Value::Array(list)) => Sizes::Fixed(
            list.iter()
                .map(|v| whole(v, "sizes", 1, u64::from(u32::MAX)).map(|n| n as u32))
                .collect::<Result<_, _>>()?,
        ),
        Some(_) => return Err("\"sizes\" must be a list of numbers or \"auto\".".to_string()),
    };
    if format == Format::Ico && matches!(&sizes, Sizes::Fixed(list) if list.is_empty()) {
        return Err("At least one size must be given.".to_string());
    }
    let fit = match text(&map, "fit")?.as_deref() {
        None | Some("contain") => FitMode::Contain,
        Some("cover") => FitMode::Cover,
        Some(other) => {
            return Err(format!(
                "Unknown fit '{other}' (use \"contain\" or \"cover\")."
            ));
        }
    };
    let layout = Layout {
        fit,
        padding: number(&map, "padding", 0, 100, 0)? as u8,
        corner_radius: number(&map, "cornerRadius", 0, u64::from(MAX_CORNER_RADIUS), 0)? as u8,
    };

    let crop = match map.get("crop") {
        None | Some(Value::Null) => None,
        Some(Value::Object(rect)) => {
            check_keys(rect, &["x", "y", "width", "height"], "crop field")?;
            Some(CropRect {
                x: number(rect, "x", 0, u64::from(u32::MAX), 0)? as u32,
                y: number(rect, "y", 0, u64::from(u32::MAX), 0)? as u32,
                width: required(rect, "width", 1, u64::from(u32::MAX))? as u32,
                height: required(rect, "height", 1, u64::from(u32::MAX))? as u32,
            })
        }
        Some(_) => return Err("\"crop\" must be an object with x, y, width and height.".into()),
    };

    let background = match map.get("background") {
        None | Some(Value::Null) => None,
        Some(Value::Object(bg)) => Some(parse_background(bg)?),
        Some(_) => return Err("\"background\" must be an object.".to_string()),
    };

    Ok(Options {
        format,
        sizes,
        layout,
        background,
        grayscale: flag(&map, "grayscale")?,
        flip_horizontal: flag(&map, "flipHorizontal")?,
        flip_vertical: flag(&map, "flipVertical")?,
        rotate: number(&map, "rotate", 0, 359, 0)? as i32,
        crop,
        trim: flag(&map, "trim")?,
        // A browser page has no threads to hand out.
        threads: 1,
        flatten: match text(&map, "flatten")? {
            None => None,
            Some(color) => Some(
                img2ico_core::chroma_key::parse_hex_color(&color)
                    .map_err(|e| format!("Invalid flatten color: {e}"))?,
            ),
        },
        // The warnings are collected and shown by the page, not printed.
        silent: false,
    })
}

fn parse_background(bg: &Map<String, Value>) -> Result<Background, String> {
    check_keys(
        bg,
        &[
            "spec",
            "find",
            "tolerance",
            "feather",
            "replacement",
            "seeds",
        ],
        "background field",
    )?;
    let spec = text(bg, "spec")?.ok_or("The background needs a \"spec\" (a color or \"auto\").")?;
    let mode = if flag(bg, "find")? {
        BackgroundMode::Find {
            min_size: img2ico_core::chroma_key::DEFAULT_FIND_MIN_SIZE,
        }
    } else {
        BackgroundMode::ChromaKey
    };
    let replacement = match text(bg, "replacement")? {
        None => None,
        Some(color) => Some(
            img2ico_core::chroma_key::parse_hex_color(&color)
                .map_err(|e| format!("Invalid replacement color: {e}"))?,
        ),
    };
    let seeds = match bg.get("seeds") {
        None => Vec::new(),
        Some(Value::Array(list)) => list
            .iter()
            .map(|point| match point {
                Value::Array(pair) if pair.len() == 2 => Ok((
                    whole(&pair[0], "seeds", 0, u64::from(u32::MAX))? as u32,
                    whole(&pair[1], "seeds", 0, u64::from(u32::MAX))? as u32,
                )),
                _ => Err("Each seed must be a pair [x, y].".to_string()),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("\"seeds\" must be a list of [x, y] pairs.".to_string()),
    };
    Ok(Background {
        mode,
        spec,
        tolerance: number(bg, "tolerance", 0, 255, 20)? as u8,
        feather: number(bg, "feather", 0, 100, 50)? as u8,
        seeds,
        replacement,
    })
}

/// A color given as `#rrggbb` (or `rrggbb`), as `#rrggbb` text.
fn color_text(text: &str) -> Result<String, String> {
    let [r, g, b] = img2ico_core::chroma_key::parse_hex_color(text)
        .map_err(|e| format!("Invalid color '{text}': {e}"))?;
    Ok(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn check_keys(map: &Map<String, Value>, known: &[&str], what: &str) -> Result<(), String> {
    match map.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(format!("Unknown {what} \"{key}\".")),
        None => Ok(()),
    }
}

fn text(map: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("\"{key}\" must be text.")),
    }
}

fn flag(map: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("\"{key}\" must be true or false.")),
    }
}

fn whole(value: &Value, key: &str, min: u64, max: u64) -> Result<u64, String> {
    match value.as_u64() {
        Some(n) if (min..=max).contains(&n) => Ok(n),
        _ => Err(format!(
            "\"{key}\" must be a whole number from {min} to {max}."
        )),
    }
}

fn number(
    map: &Map<String, Value>,
    key: &str,
    min: u64,
    max: u64,
    default: u64,
) -> Result<u64, String> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => whole(value, key, min, max),
    }
}

fn required(map: &Map<String, Value>, key: &str, min: u64, max: u64) -> Result<u64, String> {
    match map.get(key) {
        None => Err(format!("\"{key}\" is missing.")),
        Some(value) => whole(value, key, min, max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba};
    use std::io::Cursor;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, y| {
            if (x + y) % 7 < 3 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 255])
            }
        });
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn the_frames_of_a_gif_can_be_shown_and_one_of_them_opened() {
        use image::codecs::gif::GifEncoder;
        let mut gif = Cursor::new(Vec::new());
        {
            let mut encoder = GifEncoder::new(&mut gif);
            for color in [[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]] {
                encoder
                    .encode_frame(image::Frame::new(RgbaImage::from_pixel(5, 3, Rgba(color))))
                    .unwrap();
            }
        }
        let frames = decode_gif_frames_from_bytes(&gif.into_inner(), "a.gif", 0).unwrap();
        assert_eq!(frames.len(), 3);
        let png = gif_frame_png(&frames, 1).unwrap();
        let second = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(second.dimensions(), (5, 3));
        assert_eq!(second.get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert!(gif_frame_png(&frames, 3).unwrap_err().contains("3 frame"));
    }

    fn source(bytes: &[u8]) -> Source {
        Source {
            picture: open_source(bytes, "test.png", 1).unwrap(),
            svg: None,
        }
    }

    #[test]
    fn an_empty_file_is_reported_as_invalid_json() {
        let json: serde_json::Value = serde_json::from_str(&validate_ico(&[])).unwrap();
        assert_eq!(json["valid"], false);
        assert!(!json["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn the_engine_has_a_version() {
        assert!(!engine_version().is_empty());
    }

    #[test]
    fn a_picture_is_opened_and_converted_with_the_default_options() {
        let source = source(&png(64, 40));
        assert_eq!((source.width(), source.height()), (64, 40));
        assert!(!source.is_vector());
        let output = source.convert_with("{}").unwrap();
        assert_eq!(output.sizes, vec![16, 32, 48, 64, 128, 256]);
        assert!(validate_bytes(&output.bytes).is_valid());
    }

    #[test]
    fn the_options_decide_the_sizes_and_the_format() {
        let source = source(&png(64, 64));
        let ico = source
            .convert_with(r#"{"sizes": [16, 48], "padding": 10, "cornerRadius": 20}"#)
            .unwrap();
        assert_eq!(ico.sizes, vec![16, 48]);
        let icns = source.convert_with(r##"{"format": "icns"}"##).unwrap();
        assert!(icns.bytes.starts_with(b"icns"));
    }

    #[test]
    fn a_background_can_be_removed() {
        let source = source(&png(32, 32));
        let output = source
            .convert_with(
                r##"{"sizes": [32], "background": {"spec": "#00ff00", "tolerance": 10}}"##,
            )
            .unwrap();
        let dir = ico::IconDir::read(Cursor::new(&output.bytes)).unwrap();
        let rgba = dir.entries()[0].decode().unwrap();
        assert!(
            rgba.rgba_data().chunks(4).any(|p| p[3] == 0),
            "some transparent"
        );
    }

    #[test]
    fn warnings_come_back_with_the_result() {
        // A 20-pixel picture scaled to 256 is a stretch.
        let output = source(&png(20, 20))
            .convert_with(r#"{"sizes": [256]}"#)
            .unwrap();
        let warnings: Vec<Value> = serde_json::from_str(&output.warnings()).unwrap();
        assert!(!warnings.is_empty(), "{warnings:?}");
        assert!(warnings[0]["text"].as_str().unwrap().contains("upscaled"));
    }

    #[test]
    fn a_message_comes_back_as_a_code_with_its_values() {
        msg::remember_messages();
        // a 20-pixel picture scaled to 256 is a stretch
        let output = source(&png(20, 20))
            .convert_with(r#"{"sizes": [256]}"#)
            .unwrap();
        let warnings: Vec<Value> = serde_json::from_str(&output.warnings()).unwrap();
        assert_eq!(warnings[0]["code"], "resize.upscaled");
        assert_eq!(warnings[0]["params"]["source_width"], "20");
        assert_eq!(warnings[0]["params"]["sizes"], "[256]");
    }

    #[test]
    fn a_reason_inside_a_message_is_a_message_too_and_unknown_text_is_other() {
        msg::remember_messages();
        let error = open_source(&[0u8; 8], "x.png", 1).err().unwrap();
        let shown = structured(&error);
        assert_eq!(shown["code"], "source.unreadable");
        assert_eq!(shown["params"]["name"], "x.png");
        // the reason is a message of its own: a file that is no picture at all
        assert_eq!(shown["params"]["e"]["code"], "source.format_unknown");
        assert_eq!(structured("something else")["code"], "other");

        // a limit error nested in a "could not read" is a message of its own
        let big = img2ico_core::source::check_pixel_limit(9000, 9000, 40_000_000).unwrap_err();
        let outer = img2ico_core::msg!(
            "source.unreadable",
            "Could not read '{name}': {e}",
            name = "big.png",
            e = big
        );
        assert_eq!(
            structured(&outer)["params"]["e"]["code"],
            "source.too_many_pixels"
        );
        assert_eq!(structured(&outer)["params"]["e"]["params"]["width"], "9000");
    }

    #[test]
    fn the_pixels_of_an_icon_image_come_back_exactly() {
        let output = source(&png(32, 32))
            .convert_with(r#"{"sizes": [16, 32]}"#)
            .unwrap();
        let image = icon_pixels(&output.bytes, 0).ok().unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        assert_eq!(image.rgba().len(), 16 * 16 * 4);
        assert_eq!(icon_pixels(&output.bytes, 1).ok().unwrap().width(), 32);
    }

    fn icon_of(sizes: &[u32]) -> Vec<u8> {
        source(&png(64, 64))
            .convert_with(&format!("{{\"sizes\": {sizes:?}}}"))
            .unwrap()
            .bytes
    }

    #[test]
    fn an_icon_file_is_described() {
        let json: Value =
            serde_json::from_str(&describe_icon(&icon_of(&[16, 32])).unwrap()).unwrap();
        assert_eq!(json["images"].as_array().unwrap().len(), 2);
        assert_eq!(json["images"][1]["width"], 32);
        assert_eq!(json["images"][0]["format"], "PNG");
        assert!(
            json["missing_windows_sizes"]
                .as_array()
                .unwrap()
                .contains(&Value::from(48))
        );
        assert!(describe_icon(b"nope").is_err());
    }

    #[test]
    fn an_image_is_taken_out_as_png_or_selected() {
        let bytes = icon_of(&[16, 32, 48]);
        assert!(
            extract_png(&bytes, 1)
                .unwrap()
                .starts_with(&[0x89, b'P', b'N', b'G'])
        );
        assert!(
            extract_png(&bytes, 3)
                .unwrap_err()
                .contains("no image number 3")
        );
        let picked = select_images(&bytes, &[2, 0]).unwrap();
        assert_eq!(read_dir(&picked, "x").unwrap().entries().len(), 2);
        assert!(select_images(&bytes, &[]).is_err());
        assert!(
            select_images(&bytes, &[7])
                .unwrap_err()
                .contains("out of range")
        );
    }

    #[test]
    fn icon_files_are_merged_and_duplicates_reported() {
        let mut merger = Merger::new();
        assert!(merger.merge_files().is_err(), "nothing to merge yet");
        merger.files.push((
            "a.ico".to_string(),
            read_dir(&icon_of(&[16, 32]), "a").unwrap(),
        ));
        merger.files.push((
            "b.ico".to_string(),
            read_dir(&icon_of(&[32, 48]), "b").unwrap(),
        ));
        let merged = merger.merge_files().unwrap();
        assert_eq!(merged.sizes, vec![16, 32, 48]);
        assert_eq!(merged.warnings.len(), 1);
        assert!(merged.warnings[0].contains("32x32 from 'b.ico'"));
        assert!(validate_bytes(&merged.bytes).is_valid());
    }

    #[test]
    fn the_images_of_an_icon_come_as_a_zip_of_pngs() {
        let bytes = icon_of(&[16, 48]);
        let zip = ZipArchive::open(png_zip(&bytes, "logo").unwrap()).unwrap();
        let names: Vec<String> = zip.files().into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["logo_16x16.png", "logo_48x48.png"]);
        let first = zip.read(zip.files()[0].index).unwrap();
        assert!(first.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(png_zip(b"nope", "x").is_err());
    }

    #[test]
    fn the_images_of_an_icon_come_as_a_zip_of_any_kind() {
        let bytes = icon_of(&[16, 48]);
        let zip = ZipArchive::open(image_zip(&bytes, "logo", "JPG", "#ffffff").unwrap()).unwrap();
        let names: Vec<String> = zip.files().into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["logo_16x16.jpg", "logo_48x48.jpg"]);
        let first = zip.read(zip.files()[0].index).unwrap();
        assert!(
            first.starts_with(&[0xFF, 0xD8, 0xFF]),
            "a JPEG starts with its marker"
        );
        let webp = image_of_icon(&bytes, 1, "webp", "#ffffff").unwrap();
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(
            image::GenericImageView::dimensions(&image::load_from_memory(&webp).unwrap()),
            (48, 48)
        );
        assert!(
            image_zip(&bytes, "logo", "gif", "#ffffff")
                .unwrap_err()
                .contains("not an image type")
        );
        assert!(image_of_icon(&bytes, 5, "png", "#ffffff").is_err());
        assert!(image_zip(&bytes, "logo", "png", "nope").is_err());
    }

    #[test]
    fn the_picture_can_be_saved_in_its_own_size_after_the_edits() {
        let source = source(&png(60, 40));
        // Cropped to 30 x 20, turned a quarter (20 x 30), margin and sizes of the icon do not matter.
        let output = source
            .image_with(
                r#"{"crop": {"x": 0, "y": 0, "width": 20, "height": 30}, "rotate": 90, "padding": 10, "sizes": [16]}"#,
                "png",
            )
            .unwrap();
        assert_eq!(output.sizes, vec![20, 30]);
        let image = image::load_from_memory(&output.bytes).unwrap();
        assert_eq!(image::GenericImageView::dimensions(&image), (20, 30));
        // A JPEG of it lies on the color of `flatten`.
        let jpeg = source
            .image_with(r##"{"flatten": "#102030"}"##, "jpg")
            .unwrap();
        assert!(jpeg.bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
        assert_eq!(jpeg.sizes, vec![60, 40]);
        assert!(
            source
                .image_with("{}", "svg")
                .err()
                .unwrap()
                .contains("not an image type")
        );
    }

    #[test]
    fn a_drawing_can_be_saved_as_a_picture_at_the_size_it_declares() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="#336699"/></svg>"##;
        let source = Source {
            picture: open_source(svg, "a.svg", 1).unwrap(),
            svg: Some(svg.to_vec()),
        };
        let output = source.image_with("{}", "png").unwrap();
        assert_eq!(output.sizes, vec![40, 20]);
        let image = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
        assert_eq!(image.get_pixel(20, 10).0, [0x33, 0x66, 0x99, 255]);
    }

    #[test]
    fn a_zip_is_built_and_read_through_the_bindings() {
        let mut builder = ZipBuilder::new();
        builder.add("a.png", &png(8, 8));
        builder.add("b.txt", b"hi");
        let reader = ZipReader {
            archive: ZipArchive::open(write_zip(&builder.files).unwrap()).unwrap(),
        };
        let listed: Vec<Value> = serde_json::from_str(&reader.files()).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[1]["name"], "b.txt");
        assert_eq!(reader.archive.read(1).unwrap(), b"hi");
    }

    #[test]
    fn the_website_icon_package_has_the_files_a_site_needs() {
        let source = source(&png(120, 80));
        let meta = r##"{"name": "Demo", "themeColor": "#336699", "appleBackground": "#ffffff"}"##;
        let pack = source.pack_with(r#"{"padding": 10}"#, meta).unwrap();
        let zip = ZipArchive::open(pack.zip()).unwrap();
        let names: Vec<String> = zip.files().into_iter().map(|f| f.name).collect();
        assert_eq!(
            names,
            vec![
                "favicon.ico",
                "favicon-16x16.png",
                "favicon-32x32.png",
                "apple-touch-icon.png",
                "icon-192.png",
                "icon-512.png",
                "site.webmanifest",
                "head-snippet.html",
            ]
        );
        let read = |name: &str| {
            let file = zip.files().into_iter().find(|f| f.name == name).unwrap();
            zip.read(file.index).unwrap()
        };
        assert_eq!(
            read_dir(&read("favicon.ico"), "x").unwrap().entries().len(),
            3
        );
        let apple = image::load_from_memory(&read("apple-touch-icon.png"))
            .unwrap()
            .to_rgba8();
        assert_eq!((apple.width(), apple.height()), (180, 180));
        assert!(
            apple.pixels().all(|p| p[3] == 255),
            "the Apple icon has no transparency"
        );
        let big = image::load_from_memory(&read("icon-512.png")).unwrap();
        assert_eq!((big.width(), big.height()), (512, 512));
        let manifest = String::from_utf8(read("site.webmanifest")).unwrap();
        assert!(manifest.contains("\"name\": \"Demo\"") && manifest.contains("#336699"));
        assert!(
            pack.snippet().contains("apple-touch-icon.png")
                && !pack.snippet().contains("favicon.svg")
        );
        assert!(!pack.warnings().is_empty() || pack.warnings() == "[]");
    }

    #[test]
    fn an_svg_goes_into_the_package_as_it_is() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="40" height="40" fill="#336699"/></svg>"##;
        let source = Source {
            picture: open_source(svg, "a.svg", 1).unwrap(),
            svg: Some(svg.to_vec()),
        };
        let pack = source.pack_with("{}", "{}").unwrap();
        let zip = ZipArchive::open(pack.zip()).unwrap();
        let file = zip
            .files()
            .into_iter()
            .find(|f| f.name == "favicon.svg")
            .unwrap();
        assert_eq!(zip.read(file.index).unwrap(), svg);
        assert!(pack.snippet().contains("favicon.svg"));
    }

    #[test]
    fn the_package_settings_are_checked() {
        let source = source(&png(32, 32));
        assert!(
            source
                .pack_with("{}", r#"{"themeColor": "blue-ish"}"#)
                .err()
                .unwrap()
                .contains("Invalid color")
        );
        assert!(
            source
                .pack_with("{}", r#"{"appleBackground": "nope"}"#)
                .err()
                .unwrap()
                .contains("Apple icon")
        );
        assert!(source.pack_with("{}", "not json").is_err());
    }

    #[test]
    fn a_png_and_a_flatten_color_can_be_asked_for_by_json() {
        let source = source(&png(32, 32));
        let output = source
            .convert_with(
                r##"{"format": "png", "sizes": [64], "flatten": "#ffcc00", "padding": 20}"##,
            )
            .unwrap();
        let image = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
        assert_eq!((image.width(), image.height()), (64, 64));
        assert_eq!(image.get_pixel(0, 0).0, [255, 204, 0, 255]);
        assert!(
            source
                .convert_with(r#"{"flatten": "x"}"#)
                .err()
                .unwrap()
                .contains("flatten")
        );
    }

    #[test]
    fn an_svg_is_a_drawing() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="#336699"/></svg>"##;
        let source = Source {
            picture: open_source(svg, "a.svg", 1).unwrap(),
            svg: Some(svg.to_vec()),
        };
        assert!(source.is_vector());
        assert_eq!((source.width(), source.height()), (40, 20));
        assert_eq!(
            source.convert_with(r#"{"sizes": [32]}"#).unwrap().sizes,
            vec![32]
        );
    }

    #[test]
    fn what_cannot_be_read_is_a_clear_error() {
        let error = open_source(b"not an image", "x.png", 1).err().unwrap();
        assert!(error.contains("Could not read 'x.png'"), "{error}");
    }

    #[test]
    fn bad_options_are_named() {
        let source = source(&png(16, 16));
        for (json, expected) in [
            ("not json", "not valid JSON"),
            ("[]", "must be a JSON object"),
            (r#"{"size": [16]}"#, "Unknown option \"size\""),
            (r#"{"sizes": []}"#, "At least one size"),
            (r#"{"sizes": [0]}"#, "from 1 to"),
            (r#"{"padding": 101}"#, "from 0 to 100"),
            (r#"{"cornerRadius": 51}"#, "from 0 to 50"),
            (r#"{"fit": "stretch"}"#, "Unknown fit"),
            (r#"{"format": "gif"}"#, "Unknown format"),
            (r#"{"grayscale": 1}"#, "true or false"),
            (r#"{"background": {}}"#, "needs a \"spec\""),
            (
                r#"{"background": {"spec": "red-ish"}}"#,
                "Invalid --chroma-key value",
            ),
            (r#"{"crop": {"x": 1}}"#, "\"width\" is missing"),
        ] {
            let error = source.convert_with(json).err().unwrap_or_default();
            assert!(error.contains(expected), "{json}: {error}");
        }
    }

    #[test]
    fn a_rotated_picture_is_cropped_on_its_turned_canvas() {
        // 40 x 10 turned by a quarter is 10 x 40: a crop of 10 x 40 fits only after the turn.
        let source = source(&png(40, 10));
        let crop = r#""crop": {"x": 0, "y": 0, "width": 10, "height": 40}"#;
        assert!(source.convert_with(&format!("{{{crop}}}")).is_err());
        let turned = source
            .convert_with(&format!(r#"{{"sizes": [16], "rotate": 90, {crop}}}"#))
            .unwrap();
        assert_eq!(turned.sizes, vec![16]);
        assert!(source.convert_with(r#"{"rotate": 360}"#).is_err());
    }

    #[test]
    fn the_turned_preview_has_the_canvas_of_the_turn_and_can_be_made_smaller() {
        let source = source(&png(400, 200));
        let full =
            image::load_from_memory(&source.preview_with(90, false, false, 0).unwrap()).unwrap();
        assert_eq!((full.width(), full.height()), (200, 400));
        let small =
            image::load_from_memory(&source.preview_with(30, false, false, 100).unwrap()).unwrap();
        // 400 x 200 turned by 30 degrees is 447 x 372 (rounded up); made to fit 100 on the long side.
        assert_eq!(small.width().max(small.height()), 100);
    }

    #[test]
    fn the_preview_is_mirrored_before_it_is_turned_and_the_option_reaches_the_icon() {
        // A 4 x 2 picture: red on the left, blue on the right.
        let mut picture = image::RgbaImage::new(4, 2);
        for (x, _y, pixel) in picture.enumerate_pixels_mut() {
            *pixel = if x < 2 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            };
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(picture)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let source = source(&bytes.into_inner());
        let shown = |h: bool, v: bool, degrees: i32| {
            image::load_from_memory(&source.preview_with(degrees, h, v, 0).unwrap())
                .unwrap()
                .to_rgba8()
        };
        assert_eq!(shown(false, false, 0).get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(shown(true, false, 0).get_pixel(0, 0).0, [0, 0, 255, 255]);
        // Top to bottom changes nothing for a picture that is the same in every row.
        assert_eq!(shown(false, true, 0).get_pixel(0, 0).0, [255, 0, 0, 255]);
        // Mirrored left to right, then turned a quarter: the blue half is now the top.
        let turned = shown(true, false, 90);
        assert_eq!(turned.dimensions(), (2, 4));
        assert_eq!(turned.get_pixel(0, 0).0, [0, 0, 255, 255]);
        // The icon is made from the mirrored picture too (a 4 x 2 picture fills the icon's width).
        let plain = source.convert_with(r#"{"sizes": [16]}"#).unwrap();
        let mirrored = source
            .convert_with(r#"{"sizes": [16], "flipHorizontal": true}"#)
            .unwrap();
        assert_ne!(plain.bytes, mirrored.bytes);
        assert!(source.convert_with(r#"{"flipHorizontal": "yes"}"#).is_err());
    }

    #[test]
    fn a_crop_outside_the_picture_is_an_error() {
        let error = source(&png(16, 16))
            .convert_with(r#"{"crop": {"x": 10, "y": 0, "width": 20, "height": 5}}"#)
            .err()
            .unwrap();
        assert!(!error.is_empty());
    }

    #[test]
    fn an_animated_gif_frame_can_be_chosen() {
        let mut bytes = Cursor::new(Vec::new());
        {
            use image::codecs::gif::GifEncoder;
            let mut encoder = GifEncoder::new(&mut bytes);
            for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
                encoder
                    .encode_frame(image::Frame::new(RgbaImage::from_pixel(8, 8, Rgba(color))))
                    .unwrap();
            }
        }
        let bytes = bytes.into_inner();
        let Picture::Raster(second) = open_source(&bytes, "a.gif", 2).unwrap() else {
            panic!("a GIF is a raster");
        };
        assert_eq!(second.get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert!(open_source(&bytes, "a.gif", 3).is_err());
    }
}
