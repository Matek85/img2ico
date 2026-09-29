// Everything related to the optional TOML settings file: loading it
// (--config, or auto-discovered "img2ico.toml" in the current directory)
// and writing one back out (--out-toml).

use crate::cli::{OutputFormat, SizePreset};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The filename looked for in the current directory when --config isn't
/// given explicitly - see load_config() below.
const AUTO_CONFIG_FILENAME: &str = "img2ico.toml";

/// Every "tuning" setting that can come from a config file, in addition
/// to the command line. Deliberately does NOT include the mode flags
/// (--merge/--inspect/--extract/--select), the input file(s), or the
/// output path (-o) - defaulting those rarely makes sense, since they're
/// exactly what changes from run to run, not a standing preference.
///
/// Every field is optional (or, for plain bool/Vec fields, defaults to
/// "off"/empty via `#[serde(default)]`): an absent field simply means "no
/// opinion from this source", falling through to the command line and
/// then to img2ico's own built-in default. See where each field of this
/// struct is used in main.rs's run() for the exact fallback chain.
///
/// `#[serde(rename_all = "kebab-case")]` makes the TOML keys match the
/// long command-line flag names exactly (e.g. `chroma_key` here becomes
/// `chroma-key` in the file, matching `--chroma-key`) - so anything you
/// already know from `--help` carries over directly to the file, without
/// a second naming scheme to learn.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    pub sizes: Option<String>,
    pub preset: Option<SizePreset>,
    pub chroma_key: Option<String>,
    pub tolerance: Option<u8>,
    #[serde(default)]
    pub seeds: Vec<String>,
    pub find: Option<String>,
    pub find_min_size: Option<usize>,
    #[serde(default)]
    pub auto_apply: bool,
    pub replace_color: Option<String>,
    #[serde(default)]
    pub grayscale: bool,
    pub padding: Option<u8>,
    pub gif_frame: Option<usize>,
    pub output_format: Option<OutputFormat>,
    #[serde(default)]
    pub delete_source: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub combine: bool,
    pub index: Option<String>,
    #[serde(default)]
    pub silent: bool,
}

/// Every valid top-level key a settings file can contain - the exact
/// kebab-case TOML names, matching `Settings`' own
/// `#[serde(rename_all = "kebab-case")]`. Used only to warn about a
/// likely typo (see `load_config` below); it has no effect on what's
/// actually accepted, since serde already silently ignores unknown
/// fields when deserializing into `Settings` regardless of this list.
/// Keep this in sync by hand whenever a field is added to or removed
/// from `Settings` - the same way cli.rs's --config doc comment lists
/// them too.
const KNOWN_SETTINGS_KEYS: &[&str] = &[
    "sizes",
    "preset",
    "chroma-key",
    "tolerance",
    "seeds",
    "find",
    "find-min-size",
    "auto-apply",
    "replace-color",
    "grayscale",
    "padding",
    "gif-frame",
    "output-format",
    "delete-source",
    "force",
    "combine",
    "index",
    "silent",
];

/// Resolves and loads the config file for this run, if any:
/// - `explicit_path` (from --config) always wins outright when given -
///   the automatic lookup below is skipped entirely in that case, the two
///   are never combined.
/// - Otherwise, looks for "img2ico.toml" in the current directory and
///   uses it automatically if present.
/// - If neither applies, returns `Ok(None)` - not an error, just "no
///   config file for this run", and img2ico proceeds with pure
///   command-line + built-in defaults exactly as before this feature
///   existed.
///
/// Returns the parsed `Settings` together with the path that was
/// actually used, so the caller can print an info message about it -
/// img2ico never silently changes its own behavior because of a file
/// sitting in the current directory without saying so out loud.
///
/// `cli_silent` is the RAW --silent flag as given on the command line
/// (NOT the fully resolved value, which isn't known yet at this point -
/// resolving it requires the config file that's still being loaded right
/// here). It only controls whether THIS function's own warnings (an
/// unrecognized setting name) are printed; every warning elsewhere in the
/// program instead respects the fully resolved --silent once it's known.
pub fn load_config(
    explicit_path: Option<&Path>,
    cli_silent: bool,
) -> Result<Option<(Settings, PathBuf)>, String> {
    let path: PathBuf = match explicit_path {
        Some(p) => p.to_path_buf(),
        None => {
            let candidate = PathBuf::from(AUTO_CONFIG_FILENAME);
            if !candidate.is_file() {
                return Ok(None);
            }
            candidate
        }
    };

    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read config file '{}': {e}", path.display()))?;

    // Warn about any top-level key that isn't one of img2ico's own
    // settings - almost always a typo (e.g. "toleranse" instead of
    // "tolerance"), silently doing nothing otherwise with no indication
    // anything was misspelled. Best-effort: if the file doesn't even
    // parse as a generic TOML table, this is simply skipped - the real
    // parse error from the typed parse a few lines below reports that
    // properly anyway.
    if !cli_silent {
        if let Ok(table) = toml::from_str::<toml::value::Table>(&text) {
            for key in table.keys() {
                if !KNOWN_SETTINGS_KEYS.contains(&key.as_str()) {
                    eprintln!(
                        "Warning: unknown setting '{key}' in '{}' - ignored. Check for a typo, or see `img2ico --help` for the exact setting names.",
                        path.display()
                    );
                }
            }
        }
    }

    let settings: Settings = toml::from_str(&text)
        .map_err(|e| format!("Could not parse config file '{}' as TOML: {e}", path.display()))?;

    Ok(Some((settings, path)))
}

/// Writes the given settings out as a TOML file - used by --out-toml to
/// save a snapshot of the settings actually used for a run, for reuse
/// later via --config, or by placing it as "img2ico.toml" in a project
/// folder for automatic pickup.
pub fn write_config(settings: &Settings, path: &Path) -> Result<(), String> {
    let text = toml::to_string_pretty(settings)
        .map_err(|e| format!("Could not turn the current settings into TOML: {e}"))?;
    std::fs::write(path, text)
        .map_err(|e| format!("Could not write settings file '{}': {e}", path.display()))
}