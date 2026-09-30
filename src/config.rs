// Everything related to the optional TOML settings files: loading them
// (--config, an auto-discovered "img2ico.toml" in the current directory,
// and a per-user file), layering them on top of each other, and writing
// the settings back out (--out-toml).

use crate::cli::{OutputFormat, SizePreset};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
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
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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

impl Settings {
    /// Layers two sets of settings: every value `self` has an opinion about
    /// wins, and `fallback` fills in whatever `self` leaves open. This is
    /// how a project's "img2ico.toml" (higher priority) sits on top of the
    /// per-user file (lower priority) - setting by setting, not "all or
    /// nothing".
    ///
    /// The rules match how the command line is resolved against a file
    /// (see settings.rs): an absent value falls through, an on/off flag is
    /// on if EITHER layer turns it on (a file can't say "explicitly off"),
    /// and a non-empty list of seeds replaces the lower layer's list
    /// instead of adding to it.
    ///
    /// Written without `..` on purpose: if a field is ever added to
    /// `Settings`, this stops compiling until it says how that field
    /// layers - so a new setting can't silently be ignored here.
    pub fn layered_over(self, fallback: Settings) -> Settings {
        Settings {
            sizes: self.sizes.or(fallback.sizes),
            preset: self.preset.or(fallback.preset),
            chroma_key: self.chroma_key.or(fallback.chroma_key),
            tolerance: self.tolerance.or(fallback.tolerance),
            seeds: if self.seeds.is_empty() {
                fallback.seeds
            } else {
                self.seeds
            },
            find: self.find.or(fallback.find),
            find_min_size: self.find_min_size.or(fallback.find_min_size),
            auto_apply: self.auto_apply || fallback.auto_apply,
            replace_color: self.replace_color.or(fallback.replace_color),
            grayscale: self.grayscale || fallback.grayscale,
            padding: self.padding.or(fallback.padding),
            gif_frame: self.gif_frame.or(fallback.gif_frame),
            output_format: self.output_format.or(fallback.output_format),
            delete_source: self.delete_source || fallback.delete_source,
            force: self.force || fallback.force,
            combine: self.combine || fallback.combine,
            index: self.index.or(fallback.index),
            silent: self.silent || fallback.silent,
        }
    }
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

/// Reads one settings file: parses it as TOML into `Settings`, warning
/// about any setting name img2ico doesn't recognize.
///
/// `cli_silent` is the RAW --silent flag as given on the command line
/// (NOT the fully resolved value, which isn't known yet at this point -
/// resolving it requires the config files that are still being loaded right
/// here). It only controls whether THIS function's own warnings (an
/// unrecognized setting name) are printed; every warning elsewhere in the
/// program instead respects the fully resolved --silent once it's known.
fn read_settings_file(path: &Path, cli_silent: bool) -> Result<Settings, String> {
    let text = std::fs::read_to_string(path)
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

    toml::from_str(&text).map_err(|e| {
        format!(
            "Could not parse config file '{}' as TOML: {e}",
            path.display()
        )
    })
}

/// Resolves and loads the PROJECT config file for this run, if any:
/// - `explicit_path` (from --config) always wins outright when given -
///   the automatic lookup below is skipped entirely in that case, the two
///   are never combined.
/// - Otherwise, looks for "img2ico.toml" in the current directory and
///   uses it automatically if present.
/// - If neither applies, returns `Ok(None)` - not an error, just "no
///   project config file for this run".
///
/// Returns the parsed `Settings` together with the path that was
/// actually used, so the caller can print an info message about it -
/// img2ico never silently changes its own behavior because of a file
/// sitting in the current directory without saying so out loud.
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

    let settings = read_settings_file(&path, cli_silent)?;
    Ok(Some((settings, path)))
}

/// The operating systems whose conventions for a per-user config location
/// differ. A parameter (instead of `cfg!` inside the lookup) so all three
/// can be tested on any machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Other,
}

impl Platform {
    /// The platform this binary was built for.
    pub const CURRENT: Platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Other
    };
}

/// Where the per-user settings file lives on `platform`, given a way to
/// look up environment variables (a parameter so tests don't have to touch
/// the real environment). `None` if the location can't be determined, e.g.
/// no home directory is set - then there is simply no user file.
///
/// The locations follow each platform's own convention:
/// - Windows: `%APPDATA%\img2ico\config.toml`
/// - macOS: `~/Library/Application Support/img2ico/config.toml`
/// - elsewhere: `$XDG_CONFIG_HOME/img2ico/config.toml`, or
///   `~/.config/img2ico/config.toml` if that variable isn't set. (The XDG
///   specification says a relative `XDG_CONFIG_HOME` must be ignored.)
///
/// An environment variable that is set but empty counts as not set.
pub fn user_config_path_for(
    platform: Platform,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };

    let base = match platform {
        Platform::Windows => var("APPDATA")?,
        Platform::MacOs => var("HOME")?.join("Library").join("Application Support"),
        Platform::Other => match var("XDG_CONFIG_HOME").filter(|dir| dir.is_absolute()) {
            Some(dir) => dir,
            None => var("HOME")?.join(".config"),
        },
    };
    Some(base.join("img2ico").join("config.toml"))
}

/// The per-user settings file location on this machine - see
/// `user_config_path_for`.
pub fn user_config_path() -> Option<PathBuf> {
    user_config_path_for(Platform::CURRENT, |name| std::env::var_os(name))
}

/// Everything the settings files contributed to this run.
#[derive(Debug, Default, PartialEq)]
pub struct LoadedConfig {
    /// The project file's settings layered over the user file's settings.
    pub settings: Settings,
    /// The files actually used, highest priority first - so the caller can
    /// announce each of them.
    pub sources: Vec<PathBuf>,
}

/// Loads all settings files that apply to this run and layers them. From
/// highest to lowest priority:
/// 1. the project file: `explicit_path` (--config) if given, otherwise an
///    "img2ico.toml" in the current directory (see `load_config`);
/// 2. the per-user file at `user_path`, if it exists.
///
/// A file that doesn't exist simply contributes nothing; one that exists
/// but can't be read or parsed is an error naming the file. Whatever
/// neither file sets falls through to the built-in defaults.
pub fn load_layered(
    explicit_path: Option<&Path>,
    user_path: Option<&Path>,
    cli_silent: bool,
) -> Result<LoadedConfig, String> {
    let mut loaded = LoadedConfig::default();

    let project = load_config(explicit_path, cli_silent)?;
    let user = match user_path {
        Some(path) if path.is_file() => Some((read_settings_file(path, cli_silent)?, path)),
        _ => None,
    };

    if let Some((settings, path)) = project {
        loaded.settings = settings;
        loaded.sources.push(path);
    }
    if let Some((settings, path)) = user {
        loaded.settings = loaded.settings.layered_over(settings);
        loaded.sources.push(path.to_path_buf());
    }
    Ok(loaded)
}

/// The given settings as the TOML text a settings file would contain -
/// shared by --out-toml (which writes it to a file) and --verbose (which
/// shows it).
pub fn settings_to_toml(settings: &Settings) -> Result<String, String> {
    toml::to_string_pretty(settings)
        .map_err(|e| format!("Could not turn the current settings into TOML: {e}"))
}

/// Writes the given settings out as a TOML file - used by --out-toml to
/// save a snapshot of the settings actually used for a run, for reuse
/// later via --config, or by placing it as "img2ico.toml" in a project
/// folder for automatic pickup.
pub fn write_config(settings: &Settings, path: &Path) -> Result<(), String> {
    let text = settings_to_toml(settings)?;
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

    // --- Layering ----------------------------------------------------------------

    #[test]
    fn layering_prefers_the_higher_layer_and_fills_gaps_from_the_lower() {
        let high = Settings {
            tolerance: Some(40),
            sizes: Some("16".to_string()),
            ..Settings::default()
        };
        let low = Settings {
            tolerance: Some(10),
            padding: Some(5),
            preset: Some(SizePreset::Favicon),
            ..Settings::default()
        };
        let merged = high.layered_over(low);
        assert_eq!(merged.tolerance, Some(40), "higher layer wins");
        assert_eq!(merged.sizes.as_deref(), Some("16"));
        assert_eq!(merged.padding, Some(5), "gap filled from the lower layer");
        assert_eq!(merged.preset, Some(SizePreset::Favicon));
    }

    #[test]
    fn layering_with_an_empty_layer_changes_nothing() {
        assert_eq!(
            fully_populated().layered_over(Settings::default()),
            fully_populated()
        );
        assert_eq!(
            Settings::default().layered_over(fully_populated()),
            fully_populated()
        );
    }

    #[test]
    fn layered_flags_are_on_if_either_layer_turns_them_on() {
        let high = Settings {
            force: true,
            ..Settings::default()
        };
        let low = Settings {
            grayscale: true,
            silent: true,
            ..Settings::default()
        };
        let merged = high.layered_over(low);
        assert!(merged.force && merged.grayscale && merged.silent);
        assert!(!merged.delete_source && !merged.combine && !merged.auto_apply);
    }

    #[test]
    fn non_empty_seeds_replace_the_lower_layers_seeds() {
        let with_seeds = |seeds: &[&str]| Settings {
            seeds: seeds.iter().map(|s| s.to_string()).collect(),
            ..Settings::default()
        };
        let merged = with_seeds(&["1,1"]).layered_over(with_seeds(&["2,2", "3,3"]));
        assert_eq!(merged.seeds, ["1,1"]);
        let merged = with_seeds(&[]).layered_over(with_seeds(&["2,2", "3,3"]));
        assert_eq!(merged.seeds, ["2,2", "3,3"]);
    }

    // --- Per-user config location ---------------------------------------------------

    fn env_with(pairs: &[(&str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> + use<> {
        let vars: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone().into_os_string()))
            .collect();
        move |name| {
            vars.iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    fn config_under(base: &Path) -> PathBuf {
        base.join("img2ico").join("config.toml")
    }

    #[test]
    fn windows_uses_appdata() {
        let appdata = PathBuf::from("AppDataRoaming");
        let path =
            user_config_path_for(Platform::Windows, env_with(&[("APPDATA", appdata.clone())]));
        assert_eq!(path, Some(config_under(&appdata)));
    }

    #[test]
    fn windows_without_appdata_has_no_user_file() {
        assert_eq!(user_config_path_for(Platform::Windows, env_with(&[])), None);
        assert_eq!(
            user_config_path_for(Platform::Windows, env_with(&[("APPDATA", PathBuf::new())])),
            None,
            "an empty variable counts as not set"
        );
    }

    #[test]
    fn macos_uses_application_support_under_home() {
        let home = PathBuf::from("Users").join("me");
        let path = user_config_path_for(Platform::MacOs, env_with(&[("HOME", home.clone())]));
        let expected = config_under(&home.join("Library").join("Application Support"));
        assert_eq!(path, Some(expected));
        assert_eq!(user_config_path_for(Platform::MacOs, env_with(&[])), None);
    }

    #[test]
    fn other_platforms_prefer_an_absolute_xdg_config_home() {
        let xdg = std::env::temp_dir(); // absolute on every platform
        let env = env_with(&[
            ("XDG_CONFIG_HOME", xdg.clone()),
            ("HOME", PathBuf::from("home")),
        ]);
        assert_eq!(
            user_config_path_for(Platform::Other, env),
            Some(config_under(&xdg))
        );
    }

    #[test]
    fn other_platforms_fall_back_to_dot_config_under_home() {
        let home = PathBuf::from("home").join("me");
        let expected = Some(config_under(&home.join(".config")));

        let only_home = env_with(&[("HOME", home.clone())]);
        assert_eq!(user_config_path_for(Platform::Other, only_home), expected);

        // A relative XDG_CONFIG_HOME must be ignored (XDG specification).
        let relative = env_with(&[
            ("XDG_CONFIG_HOME", PathBuf::from("relative").join("dir")),
            ("HOME", home),
        ]);
        assert_eq!(user_config_path_for(Platform::Other, relative), expected);
    }

    #[test]
    fn other_platforms_without_any_variable_have_no_user_file() {
        assert_eq!(user_config_path_for(Platform::Other, env_with(&[])), None);
    }

    // --- Loading and layering files -----------------------------------------------------

    #[test]
    fn a_project_file_is_layered_over_the_user_file() {
        let dir = tempfile::tempdir().unwrap();
        let project = write_file(
            dir.path(),
            "project.toml",
            "tolerance = 40\nsizes = \"16\"\n",
        );
        let user = write_file(dir.path(), "user.toml", "tolerance = 10\npadding = 5\n");

        let loaded = load_layered(Some(&project), Some(&user), true).unwrap();
        assert_eq!(loaded.settings.tolerance, Some(40), "project wins");
        assert_eq!(loaded.settings.sizes.as_deref(), Some("16"));
        assert_eq!(loaded.settings.padding, Some(5), "user fills the gap");
        assert_eq!(
            loaded.sources,
            vec![project, user],
            "highest priority first"
        );
    }

    #[test]
    fn a_missing_user_file_contributes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let project = write_file(dir.path(), "project.toml", "padding = 7\n");
        let absent = dir.path().join("no-such").join("config.toml");

        let loaded = load_layered(Some(&project), Some(&absent), true).unwrap();
        assert_eq!(loaded.settings.padding, Some(7));
        assert_eq!(loaded.sources, vec![project]);

        let loaded = load_layered(Some(&loaded.sources[0]), None, true).unwrap();
        assert_eq!(loaded.settings.padding, Some(7));
    }

    #[test]
    fn a_broken_user_file_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let project = write_file(dir.path(), "project.toml", "padding = 7\n");
        let user = write_file(dir.path(), "user.toml", "tolerance = \"high\"\n");

        let err = load_layered(Some(&project), Some(&user), true).unwrap_err();
        assert!(err.contains("Could not parse config file"), "{err}");
        assert!(err.contains("user.toml"), "{err}");
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
