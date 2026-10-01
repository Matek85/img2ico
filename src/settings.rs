// Turns the three possible sources of a setting - the command line, a
// config file and img2ico's own built-in defaults - into the one set of
// values a run actually uses, and back into a `Settings` snapshot for
// --out-toml.

use crate::chroma_key::{DEFAULT_FEATHER, DEFAULT_FIND_MIN_SIZE};
use crate::cli::{Args, OutputFormat, SizePreset};
use crate::config::{Settings, settings_to_toml, write_config};
use crate::util::delete_source_files;
use std::path::{Path, PathBuf};

/// The value of each "tuning" setting for THIS run, after combining the
/// command line, the config file (config.rs) and the built-in defaults.
/// Every field mirrors one from `Settings` - see that struct's doc comment
/// for which settings are config-file-eligible in the first place (and why
/// some, like the mode or the input file, deliberately aren't).
///
/// Resolution order for every field is the same: an explicit command-line
/// flag wins outright; otherwise the config file's value is used (if any);
/// otherwise img2ico's own built-in default applies. For plain on/off
/// flags (no explicit "off" is possible on the command line, only
/// "given" or "not given"), this is implemented as a simple OR: the
/// setting ends up on if EITHER the command line or the config file turns
/// it on.
///
/// Text values are borrowed from the `Args` and `Settings` they came from
/// (hence the lifetime) instead of being copied: nothing here needs to
/// outlive them, and the one place that needs owned values -
/// --out-toml's snapshot in `to_settings` - makes its own copy.
pub struct ResolvedSettings<'a> {
    pub preset: Option<SizePreset>,
    pub chroma_key: Option<&'a str>,
    pub tolerance: u8,
    pub feather: u8,
    pub seeds: &'a [String],
    pub find: Option<&'a str>,
    pub find_min_size: usize,
    pub auto_apply: bool,
    pub replace_color: Option<&'a str>,
    pub grayscale: bool,
    pub padding: u8,
    pub gif_frame: usize,
    pub silent: bool,
    pub output_format: Option<OutputFormat>,
    pub delete_source: bool,
    pub force: bool,
    pub keep_going: bool,
    pub skip_existing: bool,
    pub recursive: bool,
    pub include: &'a [String],
    pub exclude: &'a [String],
    pub combine: bool,
    pub index: Option<&'a str>,
    pub sizes: Option<&'a str>,
    /// --verbose. Unlike everything above it has no settings-file
    /// equivalent and isn't part of the --out-toml snapshot: it only
    /// controls how much this particular run says about itself.
    pub verbose: bool,
}

impl<'a> ResolvedSettings<'a> {
    /// Merges the command-line `args` with a loaded (or default, if no
    /// config file applied) `settings`, following the "CLI wins, then
    /// config file, then built-in default" priority described on the
    /// struct itself.
    pub fn resolve(args: &'a Args, settings: &'a Settings) -> Self {
        Self {
            preset: args.preset.or(settings.preset),
            chroma_key: args
                .chroma_key
                .as_deref()
                .or(settings.chroma_key.as_deref()),
            tolerance: args.tolerance.or(settings.tolerance).unwrap_or(20),
            feather: args
                .feather
                .or(settings.feather)
                .unwrap_or(DEFAULT_FEATHER)
                .min(100),
            // Seeds from the command line replace the config file's seeds
            // entirely rather than adding to them.
            seeds: if args.seeds.is_empty() {
                &settings.seeds
            } else {
                &args.seeds
            },
            find: args.find.as_deref().or(settings.find.as_deref()),
            find_min_size: args
                .find_min_size
                .or(settings.find_min_size)
                .unwrap_or(DEFAULT_FIND_MIN_SIZE),
            auto_apply: args.auto_apply || settings.auto_apply,
            replace_color: args
                .replace_color
                .as_deref()
                .or(settings.replace_color.as_deref()),
            grayscale: args.grayscale || settings.grayscale,
            padding: args.padding.or(settings.padding).unwrap_or(0),
            gif_frame: args.gif_frame.or(settings.gif_frame).unwrap_or(1),
            silent: args.silent || settings.silent,
            output_format: args.output_format.or(settings.output_format),
            delete_source: args.delete_source || settings.delete_source,
            force: args.force || settings.force,
            keep_going: args.keep_going || settings.keep_going,
            skip_existing: args.skip_existing || settings.skip_existing,
            recursive: args.recursive || settings.recursive,
            // Like seeds: a list from the command line replaces the file's.
            include: if args.include.is_empty() {
                &settings.include
            } else {
                &args.include
            },
            exclude: if args.exclude.is_empty() {
                &settings.exclude
            } else {
                &args.exclude
            },
            combine: args.combine || settings.combine,
            index: args.index.as_deref().or(settings.index.as_deref()),
            sizes: args.sizes.as_deref().or(settings.sizes.as_deref()),
            verbose: args.verbose,
        }
    }

    /// Prints one diagnostic line to standard error, but only with
    /// --verbose. Standard error (not standard output) so the normal
    /// output of a run - what a script might read - stays exactly as it is.
    pub fn note(&self, message: impl std::fmt::Display) {
        if self.verbose {
            eprintln!("verbose: {message}");
        }
    }

    /// With --verbose: shows the settings this run actually uses - what
    /// the command line, the settings files and the built-in defaults
    /// resolved to - in the same TOML form --out-toml would write.
    pub fn print_effective_settings(&self) -> Result<(), String> {
        if !self.verbose {
            return Ok(());
        }
        let toml = settings_to_toml(&self.to_settings())?;
        self.note("effective settings (command line, settings files and defaults combined):");
        for line in toml.lines() {
            self.note(format_args!("  {line}"));
        }
        Ok(())
    }

    /// Turns the resolved settings back into an owned `Settings` value,
    /// for --out-toml to write out as a snapshot of what was actually used
    /// for this run.
    pub fn to_settings(&self) -> Settings {
        Settings {
            sizes: self.sizes.map(str::to_owned),
            preset: self.preset,
            chroma_key: self.chroma_key.map(str::to_owned),
            tolerance: Some(self.tolerance),
            feather: Some(self.feather),
            seeds: self.seeds.to_vec(),
            find: self.find.map(str::to_owned),
            find_min_size: Some(self.find_min_size),
            auto_apply: self.auto_apply,
            replace_color: self.replace_color.map(str::to_owned),
            grayscale: self.grayscale,
            padding: Some(self.padding),
            gif_frame: Some(self.gif_frame),
            output_format: self.output_format,
            delete_source: self.delete_source,
            force: self.force,
            keep_going: self.keep_going,
            skip_existing: self.skip_existing,
            recursive: self.recursive,
            include: self.include.to_vec(),
            exclude: self.exclude.to_vec(),
            combine: self.combine,
            index: self.index.map(str::to_owned),
            silent: self.silent,
        }
    }
}

/// Writes `resolved` out to `path` as TOML, if `--out-toml` was given -
/// shared by every mode that supports it (merge/extract/select/normal
/// conversion). A no-op if `out_toml` is `None`. Called right at the end
/// of each mode, so it only runs after everything else about the run
/// already succeeded.
pub fn maybe_write_out_toml(
    out_toml: Option<&Path>,
    resolved: &ResolvedSettings,
) -> Result<(), String> {
    if let Some(path) = out_toml {
        write_config(&resolved.to_settings(), path)?;
        if !resolved.silent {
            say!("Settings written to '{}'.", path.display());
        }
    }
    Ok(())
}

/// The shared last step of every mode, run only once everything else
/// about it succeeded: delete the source files if --delete-source is on
/// (never the `output` file itself - see util::delete_source_files), then
/// write --out-toml's settings snapshot if requested.
pub fn finish_run(
    args: &Args,
    resolved: &ResolvedSettings,
    sources: &[PathBuf],
    output: Option<&Path>,
) -> Result<(), String> {
    delete_sources_if_requested(resolved, sources, output);
    maybe_write_out_toml(args.out_toml.as_deref(), resolved)
}

/// Deletes `sources` if --delete-source is on (never the `output` file
/// itself - see util::delete_source_files). A no-op otherwise. The caller
/// decides WHEN it is safe to call this - after everything it was asked to
/// do has succeeded.
pub fn delete_sources_if_requested(
    resolved: &ResolvedSettings,
    sources: &[PathBuf],
    output: Option<&Path>,
) {
    if resolved.delete_source {
        delete_source_files(sources, output, resolved.silent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(extra: &[&str]) -> Args {
        let mut full = vec!["img2ico", "in.png"];
        full.extend_from_slice(extra);
        Args::parse_from(full)
    }

    // --- Built-in defaults -------------------------------------------------------

    #[test]
    fn without_cli_or_config_the_built_in_defaults_apply() {
        let (cli, file) = (args(&[]), Settings::default());
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert_eq!(resolved.tolerance, 20);
        assert_eq!(resolved.feather, DEFAULT_FEATHER);
        assert_eq!(resolved.padding, 0);
        assert_eq!(resolved.gif_frame, 1);
        assert_eq!(resolved.find_min_size, DEFAULT_FIND_MIN_SIZE);
        assert_eq!(resolved.preset, None);
        assert_eq!(resolved.sizes, None);
        assert_eq!(resolved.chroma_key, None);
        assert!(resolved.seeds.is_empty());
        assert!(!resolved.grayscale && !resolved.force && !resolved.silent);
        assert!(!resolved.delete_source && !resolved.combine && !resolved.auto_apply);
    }

    // --- Config file values ------------------------------------------------------

    #[test]
    fn config_values_are_used_when_the_command_line_says_nothing() {
        let file = Settings {
            tolerance: Some(45),
            padding: Some(10),
            gif_frame: Some(4),
            find_min_size: Some(2),
            sizes: Some("16,64".to_string()),
            preset: Some(SizePreset::Minimal),
            chroma_key: Some("#00FF00".to_string()),
            replace_color: Some("#000000".to_string()),
            find: Some("#FF00FF".to_string()),
            index: Some("1,2".to_string()),
            output_format: Some(OutputFormat::Icns),
            seeds: vec!["1,2".to_string()],
            ..Settings::default()
        };
        let cli = args(&[]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert_eq!(resolved.tolerance, 45);
        assert_eq!(resolved.padding, 10);
        assert_eq!(resolved.gif_frame, 4);
        assert_eq!(resolved.find_min_size, 2);
        assert_eq!(resolved.sizes, Some("16,64"));
        assert_eq!(resolved.preset, Some(SizePreset::Minimal));
        assert_eq!(resolved.chroma_key, Some("#00FF00"));
        assert_eq!(resolved.replace_color, Some("#000000"));
        assert_eq!(resolved.find, Some("#FF00FF"));
        assert_eq!(resolved.index, Some("1,2"));
        assert_eq!(resolved.output_format, Some(OutputFormat::Icns));
        assert_eq!(resolved.seeds, ["1,2"]);
    }

    // --- Priority: command line wins ---------------------------------------------

    #[test]
    fn command_line_values_win_over_the_config_file() {
        let file = Settings {
            tolerance: Some(45),
            padding: Some(10),
            gif_frame: Some(4),
            sizes: Some("16,64".to_string()),
            preset: Some(SizePreset::Minimal),
            chroma_key: Some("#00FF00".to_string()),
            output_format: Some(OutputFormat::Icns),
            ..Settings::default()
        };
        let cli = args(&[
            "--tolerance",
            "5",
            "--padding",
            "0",
            "--gif-frame",
            "2",
            "--sizes",
            "32",
            "--preset",
            "favicon",
            "--chroma-key",
            "#FF0000",
            "--output-format",
            "ico",
        ]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert_eq!(resolved.tolerance, 5);
        assert_eq!(
            resolved.padding, 0,
            "an explicit 0 must not fall back to the config value"
        );
        assert_eq!(resolved.gif_frame, 2);
        assert_eq!(resolved.sizes, Some("32"));
        assert_eq!(resolved.preset, Some(SizePreset::Favicon));
        assert_eq!(resolved.chroma_key, Some("#FF0000"));
        assert_eq!(resolved.output_format, Some(OutputFormat::Ico));
    }

    #[test]
    fn command_line_seeds_replace_config_seeds_instead_of_adding_to_them() {
        let file = Settings {
            seeds: vec!["1,1".to_string(), "2,2".to_string()],
            ..Settings::default()
        };
        let cli = args(&["--seed", "9,9"]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert_eq!(resolved.seeds, ["9,9"]);
    }

    // --- On/off flags are OR-ed --------------------------------------------------

    #[test]
    fn a_flag_is_on_if_either_the_command_line_or_the_config_turns_it_on() {
        let file = Settings {
            grayscale: true,
            force: true,
            silent: true,
            delete_source: true,
            combine: true,
            auto_apply: true,
            keep_going: true,
            skip_existing: true,
            ..Settings::default()
        };
        let cli = args(&[]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert!(resolved.grayscale && resolved.force && resolved.silent);
        assert!(resolved.delete_source && resolved.combine && resolved.auto_apply);
        assert!(resolved.keep_going && resolved.skip_existing);

        let cli = args(&[
            "--grayscale",
            "--force",
            "--silent",
            "--delete-source",
            "--combine",
            "--auto-apply",
            "--keep-going",
        ]);
        let file = Settings::default();
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert!(resolved.grayscale && resolved.force && resolved.silent);
        assert!(resolved.delete_source && resolved.combine && resolved.auto_apply);
        assert!(resolved.keep_going && !resolved.skip_existing);

        // --skip-existing contradicts --force on the command line, so it is
        // checked on its own.
        let cli = args(&["--skip-existing"]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert!(resolved.skip_existing && !resolved.force && !resolved.keep_going);
    }

    // --- Folder selection ---------------------------------------------------------

    #[test]
    fn folder_selection_comes_from_the_file_unless_the_command_line_says_otherwise() {
        let file = Settings {
            recursive: true,
            include: vec!["*.png".to_string()],
            exclude: vec!["*_old*".to_string(), "backup/**".to_string()],
            ..Settings::default()
        };
        let cli = args(&[]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert!(resolved.recursive);
        assert_eq!(resolved.include, ["*.png"]);
        assert_eq!(resolved.exclude, ["*_old*", "backup/**"]);

        // A list on the command line replaces the file's list - it does not add to it.
        let cli = args(&["--include", "*.gif"]);
        let resolved = ResolvedSettings::resolve(&cli, &file);
        assert_eq!(resolved.include, ["*.gif"]);
        assert_eq!(resolved.exclude, ["*_old*", "backup/**"], "untouched");

        let cli = args(&["-r"]);
        let none = Settings::default();
        let resolved = ResolvedSettings::resolve(&cli, &none);
        assert!(resolved.recursive && resolved.include.is_empty());
    }

    #[test]
    fn folder_selection_is_part_of_the_settings_snapshot() {
        let cli = args(&["-r", "--include", "*.png", "--exclude", "old/**"]);
        let file = Settings::default();
        let snapshot = ResolvedSettings::resolve(&cli, &file).to_settings();
        assert!(snapshot.recursive);
        assert_eq!(snapshot.include, ["*.png"]);
        assert_eq!(snapshot.exclude, ["old/**"]);
    }

    #[test]
    fn feather_follows_the_usual_priority_and_is_capped_at_100() {
        let file = Settings {
            feather: Some(30),
            ..Settings::default()
        };
        assert_eq!(ResolvedSettings::resolve(&args(&[]), &file).feather, 30);
        let cli = args(&["--feather", "10"]);
        assert_eq!(ResolvedSettings::resolve(&cli, &file).feather, 10);
        let cli = args(&["--feather", "0"]);
        assert_eq!(
            ResolvedSettings::resolve(&cli, &file).feather,
            0,
            "an explicit 0 must not fall back to the file"
        );
        let cli = args(&["--feather", "200"]);
        assert_eq!(ResolvedSettings::resolve(&cli, &file).feather, 100);
    }

    // --- Snapshot for --out-toml -------------------------------------------------

    #[test]
    fn to_settings_records_the_resolved_values_including_defaults() {
        let (cli, file) = (
            args(&["--padding", "7", "--grayscale"]),
            Settings::default(),
        );
        let snapshot = ResolvedSettings::resolve(&cli, &file).to_settings();
        assert_eq!(snapshot.padding, Some(7));
        assert_eq!(
            snapshot.tolerance,
            Some(20),
            "defaults are written out explicitly"
        );
        assert_eq!(snapshot.gif_frame, Some(1));
        assert_eq!(snapshot.find_min_size, Some(DEFAULT_FIND_MIN_SIZE));
        assert!(snapshot.grayscale);
    }

    #[test]
    fn a_snapshot_resolves_to_the_same_settings_again() {
        let (first_cli, empty) = (
            args(&[
                "--tolerance",
                "33",
                "--seed",
                "4,5",
                "--preset",
                "windows",
                "--silent",
            ]),
            Settings::default(),
        );
        let first = ResolvedSettings::resolve(&first_cli, &empty);
        let snapshot = first.to_settings();

        let second_cli = args(&[]);
        let second = ResolvedSettings::resolve(&second_cli, &snapshot);
        assert_eq!(second.tolerance, first.tolerance);
        assert_eq!(second.seeds, first.seeds);
        assert_eq!(second.preset, first.preset);
        assert_eq!(second.silent, first.silent);
        assert_eq!(second.padding, first.padding);
    }

    // --- --verbose -------------------------------------------------------------

    #[test]
    fn verbose_comes_from_the_command_line_only() {
        let (off_cli, on_cli, file) = (args(&[]), args(&["-v"]), Settings::default());
        assert!(!ResolvedSettings::resolve(&off_cli, &file).verbose);
        assert!(ResolvedSettings::resolve(&on_cli, &file).verbose);
    }

    #[test]
    fn verbose_is_not_part_of_the_settings_snapshot() {
        let (cli, file) = (args(&["--verbose"]), Settings::default());
        let toml = settings_to_toml(&ResolvedSettings::resolve(&cli, &file).to_settings()).unwrap();
        assert!(!toml.contains("verbose"), "{toml}");
    }

    // --- maybe_write_out_toml ----------------------------------------------------

    #[test]
    fn out_toml_is_only_written_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let (cli, file) = (args(&["--silent"]), Settings::default());
        let resolved = ResolvedSettings::resolve(&cli, &file);

        maybe_write_out_toml(None, &resolved).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

        let path = dir.path().join("snapshot.toml");
        maybe_write_out_toml(Some(&path), &resolved).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("tolerance = 20"), "{text}");
    }
}
