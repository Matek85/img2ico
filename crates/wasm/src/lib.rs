//! The browser-facing side of img2ico: a few functions that the web page
//! calls (from a Web Worker) with plain bytes and that return plain text or
//! bytes. No logic lives here - it only translates between JavaScript and
//! img2ico-core.

use img2ico_core::validate::{Severity, validate_bytes};
use wasm_bindgen::prelude::*;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
