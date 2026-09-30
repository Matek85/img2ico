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
    if !cli_silent && let Ok(table) = toml::from_str::<toml::value::Table>(&text) {
        for key in table.keys() {
            if !KNOWN_SETTINGS_KEYS.contains(&key.as_str()) {
                eprintln!(
                    "Warning: unknown setting '{key}' in '{}' - ignored. Check for a typo, or see `img2ico --help` for the exact setting names.",
                    path.display()
                );
            }
        }
    }

    let settings: Settings = toml::from_str(&text).map_err(|e| {
        format!(
            "Could not parse config file '{}' as TOML: {e}",
            path.display()
        )
    })?;

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
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn load(text: &str) -> Result<Settings, String> {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "cfg.toml", text);
        load_config(Some(&path), true).map(|loaded| loaded.expect("explicit path always loads").0)
    }

    /// A `Settings` with every single field set to a non-default value.
    fn fully_populated() -> Settings {
        Settings {
            sizes: Some("16,32".to_string()),
            preset: Some(SizePreset::Favicon),
            chroma_key: Some("#00FF00".to_string()),
            tolerance: Some(35),
            seeds: vec!["1,2".to_string(), "3,4".to_string()],
            find: Some("#FF00FF".to_string()),
            find_min_size: Some(5),
            auto_apply: true,
            replace_color: Some("#000000".to_string()),
            grayscale: true,
            padding: Some(12),
            gif_frame: Some(3),
            output_format: Some(OutputFormat::Icns),
            delete_source: true,
            force: true,
            combine: true,
            index: Some("0,2".to_string()),
            silent: true,
        }
    }

    // --- Loading ---------------------------------------------------------------

    #[test]
    fn empty_file_gives_default_settings() {
        let settings = load("").unwrap();
        assert_eq!(settings.tolerance, None);
        assert!(settings.seeds.is_empty());
        assert!(!settings.force);
    }

    #[test]
    fn keys_use_the_kebab_case_names_of_the_command_line_flags() {
        let settings = load(
            r##"
            chroma-key = "#00FF00"
            find-min-size = 4
            replace-color = "#111111"
            output-format = "icns"
            gif-frame = 2
            delete-source = true
            auto-apply = true
            "##,
        )
        .unwrap();
        assert_eq!(settings.chroma_key.as_deref(), Some("#00FF00"));
        assert_eq!(settings.find_min_size, Some(4));
        assert_eq!(settings.replace_color.as_deref(), Some("#111111"));
        assert_eq!(settings.output_format, Some(OutputFormat::Icns));
        assert_eq!(settings.gif_frame, Some(2));
        assert!(settings.delete_source);
        assert!(settings.auto_apply);
    }

    #[test]
    fn snake_case_keys_are_not_accepted_as_setting_names() {
        let settings = load("chroma_key = \"#00FF00\"").unwrap();
        assert_eq!(settings.chroma_key, None);
    }

    #[test]
    fn presets_and_formats_are_lowercase_words() {
        let settings = load("preset = \"favicon\"\noutput-format = \"ico\"").unwrap();
        assert_eq!(settings.preset, Some(SizePreset::Favicon));
        assert_eq!(settings.output_format, Some(OutputFormat::Ico));
    }

    #[test]
    fn seeds_are_read_as_a_list() {
        let settings = load("seeds = [\"10,20\", \"30,40\"]").unwrap();
        assert_eq!(settings.seeds, vec!["10,20", "30,40"]);
    }

    #[test]
    fn unknown_settings_are_ignored_and_the_rest_still_applies() {
        let settings = load("toleranse = 5\ntolerance = 33").unwrap();
        assert_eq!(settings.tolerance, Some(33));
    }

    #[test]
    fn invalid_toml_is_an_error_naming_the_file() {
        let err = load("this is = = not toml").unwrap_err();
        assert!(err.contains("Could not parse"), "unexpected message: {err}");
        assert!(err.contains("cfg.toml"), "unexpected message: {err}");
    }

    #[test]
    fn a_value_of_the_wrong_type_is_an_error() {
        assert!(load("tolerance = \"high\"").is_err());
        assert!(load("force = \"yes\"").is_err());
    }

    #[test]
    fn a_tolerance_beyond_u8_is_an_error() {
        assert!(load("tolerance = 300").is_err());
        assert!(load("tolerance = -1").is_err());
    }

    #[test]
    fn an_unknown_preset_or_format_is_an_error() {
        assert!(load("preset = \"huge\"").is_err());
        assert!(load("output-format = \"tiff\"").is_err());
    }

    #[test]
    fn a_missing_explicit_config_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_config(Some(&dir.path().join("nope.toml")), true).unwrap_err();
        assert!(
            err.contains("Could not read config file"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn load_returns_the_path_it_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "mine.toml", "padding = 5");
        let (settings, used) = load_config(Some(&path), true).unwrap().unwrap();
        assert_eq!(settings.padding, Some(5));
        assert_eq!(used, path);
    }

    // --- Writing and round trips -----------------------------------------------

    #[test]
    fn written_settings_load_back_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.toml");
        write_config(&fully_populated(), &path).unwrap();

        let (loaded, _) = load_config(Some(&path), true).unwrap().unwrap();
        let expected = fully_populated();
        assert_eq!(loaded.sizes, expected.sizes);
        assert_eq!(loaded.preset, expected.preset);
        assert_eq!(loaded.chroma_key, expected.chroma_key);
        assert_eq!(loaded.tolerance, expected.tolerance);
        assert_eq!(loaded.seeds, expected.seeds);
        assert_eq!(loaded.find, expected.find);
        assert_eq!(loaded.find_min_size, expected.find_min_size);
        assert_eq!(loaded.auto_apply, expected.auto_apply);
        assert_eq!(loaded.replace_color, expected.replace_color);
        assert_eq!(loaded.grayscale, expected.grayscale);
        assert_eq!(loaded.padding, expected.padding);
        assert_eq!(loaded.gif_frame, expected.gif_frame);
        assert_eq!(loaded.output_format, expected.output_format);
        assert_eq!(loaded.delete_source, expected.delete_source);
        assert_eq!(loaded.force, expected.force);
        assert_eq!(loaded.combine, expected.combine);
        assert_eq!(loaded.index, expected.index);
        assert_eq!(loaded.silent, expected.silent);
    }

    #[test]
    fn writing_twice_through_a_load_gives_identical_files() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.toml");
        let second = dir.path().join("second.toml");
        write_config(&fully_populated(), &first).unwrap();
        let (loaded, _) = load_config(Some(&first), true).unwrap().unwrap();
        write_config(&loaded, &second).unwrap();
        assert_eq!(
            std::fs::read_to_string(first).unwrap(),
            std::fs::read_to_string(second).unwrap()
        );
    }

    #[test]
    fn written_file_uses_kebab_case_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.toml");
        write_config(&fully_populated(), &path).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("chroma-key = "), "{text}");
        assert!(text.contains("output-format = "), "{text}");
        assert!(text.contains("preset = "), "{text}");
        assert!(!text.contains('_'), "no snake_case keys expected: {text}");
    }

    #[test]
    fn writing_to_an_unwritable_location_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-such-dir").join("out.toml");
        let err = write_config(&Settings::default(), &path).unwrap_err();
        assert!(
            err.contains("Could not write settings file"),
            "unexpected message: {err}"
        );
    }

    // --- Drift guard -------------------------------------------------------------

    #[test]
    fn known_keys_list_matches_the_settings_struct_exactly() {
        let text = toml::to_string(&fully_populated()).unwrap();
        let table: toml::value::Table = toml::from_str(&text).unwrap();
        let actual: BTreeSet<&str> = table.keys().map(String::as_str).collect();
        let listed: BTreeSet<&str> = KNOWN_SETTINGS_KEYS.iter().copied().collect();
        assert_eq!(
            actual, listed,
            "KNOWN_SETTINGS_KEYS is out of sync with the Settings struct"
        );
    }
}
