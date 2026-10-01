// Planning a conversion before any of it happens: which output file each
// input produces, whether two of them would clash, and whether the options
// given make sense for the inputs given. Nothing here writes anything -
// which is also what lets --dry-run reuse it as it is.

use crate::cli::Args;
use crate::select::{NamePattern, Source};
use crate::settings::ResolvedSettings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One conversion: which image to read and where the icon goes.
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    /// Create the output's folder first (--keep-structure can need folders
    /// that do not exist yet).
    pub make_dirs: bool,
}

/// How the icon for an input is named and where it is put.
pub struct Naming<'a> {
    /// The output folder (-o in a batch); without one, every icon goes next
    /// to its input.
    pub folder: Option<&'a Path>,
    /// Rebuild the input's subfolders below `folder` (--keep-structure).
    pub keep_structure: bool,
    /// The --name pattern, if any; otherwise the input's name is kept.
    pub pattern: Option<&'a NamePattern>,
    pub use_icns: bool,
}

impl Naming<'_> {
    /// The extension of the icons: `ico` or `icns`.
    pub fn extension(&self) -> &'static str {
        if self.use_icns { "icns" } else { "ico" }
    }

    /// Where the icon for `source` goes.
    pub fn output_for(&self, source: &Source) -> Result<PathBuf, String> {
        let extension = self.extension();
        let stem = match self.pattern {
            Some(pattern) => pattern.render(&source.path, extension)?,
            None => source
                .path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("icon")
                .to_string(),
        };
        let name = format!("{stem}.{extension}");

        let directory = match self.folder {
            Some(folder) if self.keep_structure => match source.relative.parent() {
                Some(below) => folder.join(below),
                None => folder.to_path_buf(),
            },
            Some(folder) => folder.to_path_buf(),
            None => source
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
        };
        Ok(directory.join(name))
    }
}

/// The one job of a single-file run: `-o` (if given) is the output FILE,
/// used as it is; without it the icon is named by `pattern` or after the input
/// and put next to it.
pub fn single_job(
    explicit: Option<&Path>,
    input: &Path,
    pattern: Option<&NamePattern>,
    use_icns: bool,
) -> Result<Job, String> {
    let output = match explicit {
        Some(_) if pattern.is_some() => {
            return Err(
                "--name chooses the icon's file name, but -o already names the output file. Use one of them."
                    .to_string(),
            );
        }
        Some(path) => path.to_path_buf(),
        None => {
            let source = Source {
                path: input.to_path_buf(),
                relative: input.file_name().map(PathBuf::from).unwrap_or_default(),
            };
            Naming {
                folder: None,
                keep_structure: false,
                pattern,
                use_icns,
            }
            .output_for(&source)?
        }
    };
    Ok(Job {
        input: input.to_path_buf(),
        output,
        make_dirs: false,
    })
}

/// A form of `path` in which two paths naming the same file compare equal:
/// lowercased where the file system ignores case (Windows, macOS).
fn collision_key(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(any(windows, target_os = "macos")) {
        text.to_lowercase()
    } else {
        text
    }
}

/// Pairs every input with its output path - and refuses the whole batch,
/// before anything is written, if two inputs would end up as the same
/// output file (say `logo.png` and `logo.jpg`, both becoming `logo.ico`, or
/// `a/logo.png` and `b/logo.png` collected into one folder): one would
/// silently overwrite the other.
pub fn plan_jobs(sources: Vec<Source>, naming: &Naming) -> Result<Vec<Job>, String> {
    let make_dirs = naming.keep_structure && naming.folder.is_some();
    let mut claimed: HashMap<String, PathBuf> = HashMap::new();
    let mut jobs = Vec::with_capacity(sources.len());
    for source in sources {
        let output = naming.output_for(&source)?;
        let key = collision_key(&output);
        if let Some(first) = claimed.get(&key) {
            return Err(format!(
                "'{}' and '{}' would both be written to '{}'. Rename one of them, or convert them separately - or use --keep-structure (to keep their folders apart) or --name (for example \"{{stem}}-{{ext}}\", to tell them apart) to give them different output names.",
                first.display(),
                source.path.display(),
                output.display()
            ));
        }
        claimed.insert(key, source.path.clone());
        jobs.push(Job {
            input: source.path,
            output,
            make_dirs,
        });
    }
    Ok(jobs)
}

/// Rejects options that make no sense for a batch.
pub fn check_batch_options(
    output: Option<&Path>,
    resolved: &ResolvedSettings,
) -> Result<(), String> {
    // A --find preview prints a report for ONE image. Several of them
    // would be an undifferentiated pile of reports, and the preview isn't
    // a conversion, so it doesn't belong in one.
    if resolved.find.is_some() && !resolved.auto_apply {
        return Err(
            "--find without --auto-apply prints a report for one image and can't be used with several inputs or a folder. Run it for each file separately, or add --auto-apply."
                .to_string(),
        );
    }

    // In a batch -o names a FOLDER. Catch the two easy mix-ups instead of
    // quietly creating a folder called "icon.ico" or failing obscurely.
    if let Some(output) = output {
        if output.exists() && !output.is_dir() {
            return Err(format!(
                "With several inputs (or a folder) -o names an output folder, but '{}' is a file.",
                output.display()
            ));
        }
        let looks_like_icon_file = output.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ico") || extension.eq_ignore_ascii_case("icns")
        });
        if !output.exists() && looks_like_icon_file {
            return Err(format!(
                "With several inputs (or a folder) -o names an output folder, but '{}' looks like a file name. To convert one file to that name, give just that one input.",
                output.display()
            ));
        }
    }
    Ok(())
}

/// Rejects the folder options where they cannot do anything: they select
/// files inside folders (so some input has to be one), and --keep-structure
/// places them below an output folder (so -o has to be given).
pub fn check_folder_options(args: &Args) -> Result<(), String> {
    let any_folder = args.input.iter().any(|input| input.is_dir());
    if !any_folder {
        let used: Vec<&str> = [
            (args.recursive, "--recursive"),
            (!args.include.is_empty(), "--include"),
            (!args.exclude.is_empty(), "--exclude"),
            (args.keep_structure, "--keep-structure"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        if !used.is_empty() {
            return Err(format!(
                "{} only make sense when an input is a folder.",
                used.join(", ")
            ));
        }
    }
    if args.keep_structure && args.output.is_none() {
        return Err(
            "--keep-structure needs -o: without an output folder every icon already lands next to its input."
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use clap::Parser;

    fn source(path: &str) -> Source {
        let path = PathBuf::from(path);
        Source {
            relative: path.file_name().map(PathBuf::from).unwrap(),
            path,
        }
    }

    /// A source found `relative` below some folder.
    fn found(folder: &str, relative: &str) -> Source {
        Source {
            path: Path::new(folder).join(relative),
            relative: PathBuf::from(relative),
        }
    }

    fn plain(folder: Option<&Path>, use_icns: bool) -> Naming<'_> {
        Naming {
            folder,
            keep_structure: false,
            pattern: None,
            use_icns,
        }
    }

    fn with_resolved<T>(flags: &[&str], check: impl FnOnce(&ResolvedSettings) -> T) -> T {
        let mut full = vec!["img2ico", "in.png"];
        full.extend_from_slice(flags);
        let (cli, file) = (Args::parse_from(full), Settings::default());
        check(&ResolvedSettings::resolve(&cli, &file))
    }

    fn args(extra: &[&str]) -> Args {
        let mut full = vec!["img2ico"];
        full.extend_from_slice(extra);
        Args::parse_from(full)
    }

    // --- Output names -----------------------------------------------------------

    #[test]
    fn outputs_go_next_to_the_input_or_into_the_folder() {
        let input = source("art/logo.png");
        assert_eq!(
            plain(None, false).output_for(&input).unwrap(),
            Path::new("art").join("logo.ico")
        );
        assert_eq!(
            plain(Some(Path::new("out")), true)
                .output_for(&input)
                .unwrap(),
            Path::new("out").join("logo.icns")
        );
        assert_eq!(
            plain(None, false).output_for(&source("logo.png")).unwrap(),
            Path::new("logo.ico")
        );
    }

    #[test]
    fn dots_in_the_file_name_are_kept() {
        let out = plain(Some(Path::new("out")), false)
            .output_for(&source("logo.v2.final.png"))
            .unwrap();
        assert_eq!(out, Path::new("out").join("logo.v2.final.ico"));
    }

    #[test]
    fn an_input_without_an_extension_still_gets_one() {
        let out = plain(None, false).output_for(&source("logo")).unwrap();
        assert_eq!(out, Path::new("logo.ico"));
    }

    #[test]
    fn keep_structure_rebuilds_the_path_below_the_input_folder() {
        let naming = Naming {
            keep_structure: true,
            ..plain(Some(Path::new("out")), false)
        };
        assert_eq!(
            naming.output_for(&found("assets", "ui/save.png")).unwrap(),
            Path::new("out").join("ui").join("save.ico")
        );
        assert_eq!(
            naming.output_for(&found("assets", "top.png")).unwrap(),
            Path::new("out").join("top.ico")
        );
        // Without -o everything stays next to its input anyway.
        let next_to = Naming {
            keep_structure: true,
            ..plain(None, false)
        };
        assert_eq!(
            next_to.output_for(&found("assets", "ui/save.png")).unwrap(),
            Path::new("assets").join("ui").join("save.ico")
        );
    }

    #[test]
    fn a_name_pattern_replaces_the_stem() {
        let pattern = NamePattern::parse("{stem}-app").unwrap();
        let naming = Naming {
            pattern: Some(&pattern),
            ..plain(Some(Path::new("out")), false)
        };
        assert_eq!(
            naming.output_for(&source("art/logo.png")).unwrap(),
            Path::new("out").join("logo-app.ico")
        );

        let with_format = NamePattern::parse("{stem}_{format}").unwrap();
        let naming = Naming {
            pattern: Some(&with_format),
            ..plain(None, true)
        };
        assert_eq!(
            naming.output_for(&source("logo.png")).unwrap(),
            Path::new("logo_icns.icns")
        );
    }

    #[test]
    fn a_single_job_uses_the_explicit_output_unchanged() {
        let explicit = Path::new("out/custom.name");
        let job = single_job(Some(explicit), Path::new("in.png"), None, true).unwrap();
        assert_eq!(job.output, explicit);
        assert!(!job.make_dirs);
    }

    #[test]
    fn a_single_job_without_output_swaps_the_extension() {
        let input = Path::new("art/logo.png");
        assert_eq!(
            single_job(None, input, None, false).unwrap().output,
            Path::new("art/logo.ico")
        );
        assert_eq!(
            single_job(None, input, None, true).unwrap().output,
            Path::new("art/logo.icns")
        );
    }

    #[test]
    fn a_single_job_follows_a_name_pattern_but_not_together_with_an_output_file() {
        let pattern = NamePattern::parse("{stem}-app").unwrap();
        let job = single_job(None, Path::new("art/logo.png"), Some(&pattern), false).unwrap();
        assert_eq!(job.output, Path::new("art/logo-app.ico"));

        let err = single_job(
            Some(Path::new("x.ico")),
            Path::new("logo.png"),
            Some(&pattern),
            false,
        )
        .map(|_| ())
        .unwrap_err();
        assert!(err.contains("--name") && err.contains("-o"), "{err}");
    }

    // --- Planning a batch -------------------------------------------------------

    #[test]
    fn jobs_pair_every_input_with_its_output() {
        let files = vec![source("a.png"), source("b.jpg")];
        let jobs = plan_jobs(files, &plain(Some(Path::new("out")), false)).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].output, Path::new("out").join("a.ico"));
        assert_eq!(jobs[1].output, Path::new("out").join("b.ico"));
        assert!(!jobs[0].make_dirs);
    }

    #[test]
    fn two_inputs_that_would_share_an_output_are_refused() {
        let files = vec![source("logo.png"), source("logo.jpg")];
        let err = plan_jobs(files, &plain(None, false))
            .map(|_| ())
            .unwrap_err();
        assert!(
            err.contains("logo.png") && err.contains("logo.jpg"),
            "{err}"
        );
        assert!(err.contains("would both be written to"), "{err}");
        assert!(
            err.contains("--keep-structure") && err.contains("--name"),
            "{err}"
        );

        let same_twice = vec![source("a.png"), source("a.png")];
        assert!(plan_jobs(same_twice, &plain(None, false)).is_err());
    }

    #[test]
    fn the_same_name_in_different_places_is_not_a_collision() {
        let files = || vec![source("one/logo.png"), source("two/logo.png")];
        assert!(
            plan_jobs(files(), &plain(None, false)).is_ok(),
            "next to their inputs they differ"
        );
        assert!(
            plan_jobs(files(), &plain(Some(Path::new("out")), false)).is_err(),
            "in one folder they clash"
        );
    }

    #[test]
    fn keep_structure_keeps_same_named_files_in_different_folders_apart() {
        let files = vec![
            found("assets", "one/logo.png"),
            found("assets", "two/logo.png"),
        ];
        let naming = Naming {
            keep_structure: true,
            ..plain(Some(Path::new("out")), false)
        };
        let jobs = plan_jobs(files, &naming).unwrap();
        assert_eq!(
            jobs[0].output,
            Path::new("out").join("one").join("logo.ico")
        );
        assert_eq!(
            jobs[1].output,
            Path::new("out").join("two").join("logo.ico")
        );
        assert!(jobs.iter().all(|job| job.make_dirs));
    }

    #[test]
    fn a_name_pattern_can_tell_apart_files_that_differ_in_extension() {
        let files = || vec![source("logo.png"), source("logo.jpg")];
        assert!(plan_jobs(files(), &plain(None, false)).is_err());

        let pattern = NamePattern::parse("{stem}-{ext}").unwrap();
        let naming = Naming {
            pattern: Some(&pattern),
            ..plain(None, false)
        };
        let jobs = plan_jobs(files(), &naming).unwrap();
        assert_eq!(jobs[0].output, Path::new("logo-png.ico"));
        assert_eq!(jobs[1].output, Path::new("logo-jpg.ico"));
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn where_the_file_system_ignores_case_differing_case_is_a_collision() {
        let files = vec![source("Logo.png"), source("logo.png")];
        assert!(plan_jobs(files, &plain(Some(Path::new("out")), false)).is_err());
    }

    #[test]
    fn the_extension_follows_the_output_format() {
        let jobs = plan_jobs(vec![source("a.png")], &plain(None, true)).unwrap();
        assert_eq!(jobs[0].output, Path::new("a.icns"));
    }

    // --- Option checks ----------------------------------------------------------

    #[test]
    fn a_find_preview_cannot_run_as_a_batch() {
        let err =
            with_resolved(&["--find", "00FF00"], |r| check_batch_options(None, r)).unwrap_err();
        assert!(err.contains("--find without --auto-apply"), "{err}");
        assert!(
            with_resolved(&["--find", "00FF00", "--auto-apply"], |r| {
                check_batch_options(None, r)
            })
            .is_ok()
        );
    }

    #[test]
    fn in_a_batch_output_must_be_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.ico");
        std::fs::write(&file, b"x").unwrap();
        let check = |out: &Path| with_resolved(&[], |r| check_batch_options(Some(out), r));

        assert!(check(dir.path()).is_ok(), "an existing folder");
        assert!(
            check(&dir.path().join("new-folder")).is_ok(),
            "a folder to be created"
        );
        assert!(check(&file).unwrap_err().contains("is a file"));
        let looks_like_file = dir.path().join("out.ico");
        assert!(
            check(&looks_like_file)
                .unwrap_err()
                .contains("looks like a file name")
        );
        assert!(
            check(&dir.path().join("OUT.ICNS"))
                .unwrap_err()
                .contains("looks like a file name")
        );
    }

    #[test]
    fn folder_options_need_a_folder_input() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_str().unwrap();

        let err = check_folder_options(&args(&["a.png", "-r", "--exclude", "x"])).unwrap_err();
        assert!(
            err.contains("--recursive") && err.contains("--exclude"),
            "{err}"
        );
        assert!(!err.contains("--include"), "{err}");
        assert!(check_folder_options(&args(&["a.png", "--include", "*.png"])).is_err());

        assert!(check_folder_options(&args(&[folder, "-r", "--include", "*.png"])).is_ok());
        assert!(check_folder_options(&args(&["a.png"])).is_ok());
    }

    #[test]
    fn keep_structure_needs_an_output_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_str().unwrap();
        let err = check_folder_options(&args(&[folder, "--keep-structure"])).unwrap_err();
        assert!(err.contains("--keep-structure needs -o"), "{err}");
        assert!(check_folder_options(&args(&[folder, "--keep-structure", "-o", "out"])).is_ok());
    }
}
