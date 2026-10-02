//! The browser-facing side of img2ico: a few functions and one class that the
//! web page calls (from a Web Worker) with plain bytes and text, and that
//! return plain bytes and text. No logic lives here - it only translates
//! between JavaScript and img2ico-core.

use image::RgbaImage;
use img2ico_core::convert::{
    Background, BackgroundMode, DEFAULT_SIZES, Format, NoNotes, Options, Sizes, analyze_raster,
    convert_raster, convert_vector,
};
use img2ico_core::diag;
use img2ico_core::layout::{CropRect, FitMode, Layout, MAX_CORNER_RADIUS};
use img2ico_core::source::{
    Artwork, decode_source_bytes, extract_gif_frame_from_bytes, is_gif_bytes,
};
use img2ico_core::validate::{Severity, validate_bytes};
use img2ico_core::vector::VectorImage;
use serde_json::{Map, Value};
use wasm_bindgen::prelude::*;

/// The most pixels a picture may have in the browser: 40 million, a
/// 6300 x 6300 image. A page has far less memory to work with than a program
/// does, and the conversion keeps a few copies of the picture.
const WEB_MAX_PIXELS: u64 = 40_000_000;

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
            .map(|f| serde_json::json!({ "image": f.entry, "message": f.message }))
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

/// A picture that has been opened and can be converted again and again with
/// different options, without decoding it each time.
#[wasm_bindgen]
pub struct Source {
    picture: Picture,
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
            .map(|picture| Source { picture })
            .map_err(|message| JsError::new(&message))
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

    /// Makes the icon file the options (JSON text, see `parse_options`) ask for.
    pub fn convert(&self, options: &str) -> Result<Output, JsError> {
        self.convert_with(options)
            .map_err(|message| JsError::new(&message))
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

    /// The warnings raised while converting, as JSON text (a list of strings).
    pub fn warnings(&self) -> String {
        Value::from(self.warnings.clone()).to_string()
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

    /// `convert` without the JavaScript error type, so it can be tested.
    pub fn convert_with(&self, options_json: &str) -> Result<Output, String> {
        let options = parse_options(options_json)?;
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
    match decode_source_bytes(bytes, name, WEB_MAX_PIXELS)
        .map_err(|e| format!("Could not read '{name}': {e}"))?
    {
        Artwork::Raster(image) => Ok(Picture::Raster(image.into_rgba8())),
        Artwork::Vector(drawing) => Ok(Picture::Vector(drawing)),
    }
}

/// What the page may ask for, as JSON:
///
/// ```text
/// {
///   "format": "ico" | "icns",                 default "ico"
///   "sizes": [16, 32, ...] | "auto",           default 16,32,48,64,128,256
///   "padding": 0-100, "cornerRadius": 0-50,    default 0
///   "fit": "contain" | "cover",                default "contain"
///   "grayscale": bool, "trim": bool,           default false
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
            "crop",
            "background",
        ],
        "option",
    )?;

    let format = match text(&map, "format")?.as_deref() {
        None | Some("ico") => Format::Ico,
        Some("icns") => Format::Icns,
        Some(other) => {
            return Err(format!(
                "Unknown format '{other}' (use \"ico\" or \"icns\")."
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
        crop,
        trim: flag(&map, "trim")?,
        // A browser page has no threads to hand out.
        threads: 1,
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

    fn source(bytes: &[u8]) -> Source {
        Source {
            picture: open_source(bytes, "test.png", 1).unwrap(),
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
        let warnings: Vec<String> = serde_json::from_str(&output.warnings()).unwrap();
        assert!(!warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn an_svg_is_a_drawing() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="#336699"/></svg>"##;
        let source = Source {
            picture: open_source(svg, "a.svg", 1).unwrap(),
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
            (r#"{"format": "png"}"#, "Unknown format"),
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
