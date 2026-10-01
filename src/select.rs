// Choosing WHICH files a batch converts and WHAT the icons are called:
//   - finding the images inside the folders given as input (optionally in all
//     their subfolders), narrowed by --include / --exclude patterns, and
//   - the --name pattern that builds an output file name from the input's.
// Neither touches the file system beyond reading folders, so both are easy
// to test on their own.

use globset::{GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The file extensions (compared case-insensitively) that a folder given as
/// input contributes - the formats the normal conversion accepts.
const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];

/// Whether `path` has one of the extensions in `IMAGE_EXTENSIONS`.
pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

/// One image to convert, and where it sits relative to the folder it was
/// found in (just its file name for a file given directly) - which is what
/// --keep-structure rebuilds under the output folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub path: PathBuf,
    pub relative: PathBuf,
}

/// One --include / --exclude pattern.
struct Pattern {
    matcher: GlobMatcher,
    /// A pattern containing a "/" is matched against the path below the
    /// folder (`sub/logo.png`); one without is matched against the file name
    /// alone (`logo.png`), wherever in the tree the file sits.
    by_path: bool,
}

impl Pattern {
    fn parse(text: &str, flag: &str) -> Result<Self, String> {
        // Case-insensitive on purpose: the extension check above is too, and
        // "*.png" ought to find "LOGO.PNG" on every platform alike.
        let glob = GlobBuilder::new(text)
            .case_insensitive(true)
            .literal_separator(true)
            .build()
            .map_err(|e| format!("Invalid {flag} pattern '{text}': {}", e.kind()))?;
        Ok(Self {
            matcher: glob.compile_matcher(),
            by_path: text.contains('/'),
        })
    }

    fn matches(&self, relative: &Path) -> bool {
        if self.by_path {
            // Forward slashes on every platform, so one pattern works everywhere.
            let text = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            self.matcher.is_match(text)
        } else {
            relative
                .file_name()
                .is_some_and(|name| self.matcher.is_match(name))
        }
    }
}

/// The --include / --exclude patterns of a run.
pub struct Filter {
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
}

impl Filter {
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self, String> {
        let parse_all = |texts: &[String], flag: &str| {
            texts
                .iter()
                .map(|text| Pattern::parse(text, flag))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(Self {
            include: parse_all(include, "--include")?,
            exclude: parse_all(exclude, "--exclude")?,
        })
    }

    /// A file is taken if it matches at least one --include pattern (when
    /// there are any) and no --exclude pattern.
    pub fn accepts(&self, relative: &Path) -> bool {
        let included = self.include.is_empty() || self.include.iter().any(|p| p.matches(relative));
        included && !self.exclude.iter().any(|p| p.matches(relative))
    }
}

/// Turns the inputs into the list of image files to convert: a file stands
/// for itself, a folder for the supported image files inside it - directly,
/// or in all its subfolders with `recursive` - narrowed by `filter`. The
/// order is reproducible: folder by folder, by name. A folder that ends up
/// with no file at all is an error rather than a silent no-op.
///
/// The filter applies to what a folder contributes only. A file named on the
/// command line is named on purpose, so it is never filtered away.
pub fn expand_inputs(
    inputs: &[PathBuf],
    recursive: bool,
    filter: &Filter,
) -> Result<Vec<Source>, String> {
    let mut sources = Vec::new();
    for input in inputs {
        if !input.is_dir() {
            sources.push(Source {
                path: input.clone(),
                relative: input.file_name().map(PathBuf::from).unwrap_or_default(),
            });
            continue;
        }

        // Fail with a readable message if the folder itself can't be opened;
        // trouble further down the tree just leaves that part out.
        std::fs::read_dir(input)
            .map_err(|e| format!("Could not read folder '{}': {e}", input.display()))?;

        let mut images = 0usize;
        let mut has_subfolder = false;
        let mut found = Vec::new();
        let walk = WalkDir::new(input)
            .min_depth(1)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .sort_by_file_name();
        for entry in walk.into_iter().filter_map(Result::ok) {
            has_subfolder |= entry.file_type().is_dir();
            // path().is_file() (not file_type()) so a link to a file counts.
            if !entry.path().is_file() || !is_supported_image(entry.path()) {
                continue;
            }
            images += 1;
            let relative = entry
                .path()
                .strip_prefix(input)
                .unwrap_or(entry.path())
                .to_path_buf();
            if filter.accepts(&relative) {
                found.push(Source {
                    path: entry.path().to_path_buf(),
                    relative,
                });
            }
        }

        if found.is_empty() {
            return Err(if images > 0 {
                format!(
                    "None of the {images} image(s) in folder '{}' match the --include/--exclude patterns.",
                    input.display()
                )
            } else if !recursive && has_subfolder {
                format!(
                    "No supported images (PNG, JPG, BMP, GIF) found directly in folder '{}'. It has subfolders - add --recursive to search them too.",
                    input.display()
                )
            } else {
                format!(
                    "No supported images (PNG, JPG, BMP, GIF) found in folder '{}'.",
                    input.display()
                )
            });
        }
        sources.extend(found);
    }
    Ok(sources)
}

/// The variables a --name pattern may use.
const NAME_VARIABLES: &str = "{stem}, {ext} and {format}";

/// One piece of a --name pattern.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Stem,
    Ext,
    Format,
}

/// A parsed --name pattern: literal text mixed with the variables `{stem}`
/// (the input's name without extension), `{ext}` (the input's extension,
/// lowercase) and `{format}` (`ico` or `icns`). The icon's own extension is
/// always added after it. `{{` and `}}` stand for literal braces.
#[derive(Debug, Clone, PartialEq)]
pub struct NamePattern(Vec<Piece>);

impl NamePattern {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() {
            return Err("--name must not be empty.".to_string());
        }
        if text.contains(['/', '\\']) {
            return Err(
                "--name is a file name, not a path - use -o (and --keep-structure) to choose the folder."
                    .to_string(),
            );
        }

        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '{' => {
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(c) => name.push(c),
                            None => {
                                return Err(format!(
                                    "--name pattern '{text}' has a '{{' without a closing '}}' (write '{{{{' for a literal brace)."
                                ));
                            }
                        }
                    }
                    let piece = match name.as_str() {
                        "stem" => Piece::Stem,
                        "ext" => Piece::Ext,
                        "format" => Piece::Format,
                        other => {
                            return Err(format!(
                                "Unknown variable '{{{other}}}' in --name pattern '{text}' - the known ones are {NAME_VARIABLES}."
                            ));
                        }
                    };
                    if !literal.is_empty() {
                        pieces.push(Piece::Text(std::mem::take(&mut literal)));
                    }
                    pieces.push(piece);
                }
                '}' => {
                    return Err(format!(
                        "--name pattern '{text}' has a '}}' without an opening '{{' (write '}}}}' for a literal brace)."
                    ));
                }
                c => literal.push(c),
            }
        }
        if !literal.is_empty() {
            pieces.push(Piece::Text(literal));
        }
        Ok(Self(pieces))
    }

    /// The file name stem for `input`, which becomes `<stem>.<icon extension>`.
    pub fn render(&self, input: &Path, format: &str) -> Result<String, String> {
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("icon");
        let ext = input
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        let mut name = String::new();
        for piece in &self.0 {
            match piece {
                Piece::Text(text) => name.push_str(text),
                Piece::Stem => name.push_str(stem),
                Piece::Ext => name.push_str(&ext),
                Piece::Format => name.push_str(format),
            }
        }
        if name.is_empty() {
            return Err(format!(
                "The --name pattern gives an empty file name for '{}'.",
                input.display()
            ));
        }
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    /// The found files as paths relative to the folder, with forward slashes.
    fn rel(sources: &[Source]) -> Vec<String> {
        sources
            .iter()
            .map(|s| s.relative.to_string_lossy().replace('\\', "/"))
            .collect()
    }

    fn no_filter() -> Filter {
        Filter::new(&[], &[]).unwrap()
    }

    fn filter(include: &[&str], exclude: &[&str]) -> Filter {
        let owned = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Filter::new(&owned(include), &owned(exclude)).unwrap()
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "a.png",
            "b.JPG",
            "notes.txt",
            "sub/c.png",
            "sub/c_old.png",
            "sub/deep/d.gif",
            "other/e.bmp",
        ] {
            touch(&dir.path().join(name));
        }
        dir
    }

    // --- is_supported_image ----------------------------------------------------

    #[test]
    fn supported_images_are_recognized_by_extension_ignoring_case() {
        for yes in [
            "a.png", "a.PNG", "a.jpg", "a.JPEG", "a.bmp", "a.gif", "d/a.Png",
        ] {
            assert!(is_supported_image(Path::new(yes)), "{yes}");
        }
        for no in ["a.ico", "a.txt", "a.pngx", "png", "a", "a."] {
            assert!(!is_supported_image(Path::new(no)), "{no}");
        }
    }

    // --- Folders and recursion -------------------------------------------------

    #[test]
    fn a_folder_contributes_only_its_own_images_without_recursion() {
        let dir = tree();
        let found = expand_inputs(&[dir.path().to_path_buf()], false, &no_filter()).unwrap();
        assert_eq!(rel(&found), ["a.png", "b.JPG"]);
    }

    #[test]
    fn recursion_adds_every_subfolder_in_name_order() {
        let dir = tree();
        let found = expand_inputs(&[dir.path().to_path_buf()], true, &no_filter()).unwrap();
        assert_eq!(
            rel(&found),
            [
                "a.png",
                "b.JPG",
                "other/e.bmp",
                "sub/c.png",
                "sub/c_old.png",
                "sub/deep/d.gif"
            ]
        );
        // The relative path is what --keep-structure rebuilds.
        assert_eq!(found[2].path, dir.path().join("other").join("e.bmp"));
    }

    #[test]
    fn a_file_given_directly_is_never_filtered_and_has_just_its_name() {
        let found = expand_inputs(
            &[PathBuf::from("art/logo.png")],
            false,
            &filter(&["*.jpg"], &["logo*"]),
        )
        .unwrap();
        assert_eq!(rel(&found), ["logo.png"]);
        assert_eq!(found[0].path, PathBuf::from("art/logo.png"));
    }

    #[test]
    fn an_empty_folder_names_the_subfolder_hint_only_when_it_applies() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("sub/x.png"));
        let err = expand_inputs(&[dir.path().to_path_buf()], false, &no_filter()).unwrap_err();
        assert!(err.contains("--recursive"), "{err}");

        let plain = tempfile::tempdir().unwrap();
        touch(&plain.path().join("notes.txt"));
        let err = expand_inputs(&[plain.path().to_path_buf()], false, &no_filter()).unwrap_err();
        assert!(err.contains("No supported images"), "{err}");
        assert!(!err.contains("--recursive"), "{err}");
    }

    #[test]
    fn filters_that_remove_everything_are_reported_as_such() {
        let dir = tree();
        let err = expand_inputs(&[dir.path().to_path_buf()], true, &filter(&["*.tiff"], &[]))
            .unwrap_err();
        assert!(err.contains("match the --include/--exclude"), "{err}");
    }

    // --- Filters ----------------------------------------------------------------

    #[test]
    fn include_keeps_only_matching_names_in_any_folder() {
        let dir = tree();
        let found =
            expand_inputs(&[dir.path().to_path_buf()], true, &filter(&["*.png"], &[])).unwrap();
        assert_eq!(rel(&found), ["a.png", "sub/c.png", "sub/c_old.png"]);
    }

    #[test]
    fn exclude_drops_matching_names_and_wins_over_include() {
        let dir = tree();
        let found = expand_inputs(
            &[dir.path().to_path_buf()],
            true,
            &filter(&["*.png"], &["*_old*"]),
        )
        .unwrap();
        assert_eq!(rel(&found), ["a.png", "sub/c.png"]);
    }

    #[test]
    fn several_include_patterns_are_alternatives() {
        let dir = tree();
        let found = expand_inputs(
            &[dir.path().to_path_buf()],
            true,
            &filter(&["a.*", "*.gif"], &[]),
        )
        .unwrap();
        assert_eq!(rel(&found), ["a.png", "sub/deep/d.gif"]);
    }

    #[test]
    fn matching_ignores_case() {
        let dir = tree();
        let found =
            expand_inputs(&[dir.path().to_path_buf()], false, &filter(&["*.jpg"], &[])).unwrap();
        assert_eq!(rel(&found), ["b.JPG"]);
    }

    #[test]
    fn a_pattern_with_a_slash_matches_the_path_below_the_folder() {
        let dir = tree();
        let found =
            expand_inputs(&[dir.path().to_path_buf()], true, &filter(&[], &["sub/**"])).unwrap();
        assert_eq!(rel(&found), ["a.png", "b.JPG", "other/e.bmp"]);

        // "*" does not cross folders in a path pattern, "**" does.
        let found = expand_inputs(
            &[dir.path().to_path_buf()],
            true,
            &filter(&["sub/*.png"], &[]),
        )
        .unwrap();
        assert_eq!(rel(&found), ["sub/c.png", "sub/c_old.png"]);
    }

    #[test]
    fn a_broken_pattern_is_an_error_naming_the_flag() {
        let owned = vec!["[abc".to_string()];
        let err = Filter::new(&owned, &[]).err().unwrap();
        assert!(err.contains("--include") && err.contains("[abc"), "{err}");
        let err = Filter::new(&[], &owned).err().unwrap();
        assert!(err.contains("--exclude"), "{err}");
    }

    // --- Name patterns ----------------------------------------------------------

    fn render(pattern: &str, input: &str, format: &str) -> String {
        NamePattern::parse(pattern)
            .unwrap()
            .render(Path::new(input), format)
            .unwrap()
    }

    #[test]
    fn variables_are_replaced() {
        assert_eq!(render("{stem}-app", "art/logo.png", "ico"), "logo-app");
        assert_eq!(render("{stem}_{ext}", "logo.PNG", "ico"), "logo_png");
        assert_eq!(render("{stem}.{format}", "logo.png", "icns"), "logo.icns");
        assert_eq!(render("fixed", "logo.png", "ico"), "fixed");
        assert_eq!(render("{stem}{stem}", "ab.png", "ico"), "abab");
    }

    #[test]
    fn doubled_braces_are_literal() {
        assert_eq!(render("{{{stem}}}", "logo.png", "ico"), "{logo}");
    }

    #[test]
    fn bad_patterns_are_refused_with_a_clear_message() {
        let err = |text: &str| NamePattern::parse(text).unwrap_err();
        assert!(err("{name}").contains("Unknown variable '{name}'"));
        assert!(err("{name}").contains("{stem}"));
        assert!(err("a{stem").contains("without a closing"));
        assert!(err("a}b").contains("without an opening"));
        assert!(err("").contains("empty"));
        assert!(err("a/{stem}").contains("not a path"));
        assert!(err("a\\{stem}").contains("not a path"));
    }

    #[test]
    fn a_pattern_that_renders_empty_is_an_error() {
        let pattern = NamePattern::parse("{ext}").unwrap();
        let err = pattern.render(Path::new("noextension"), "ico").unwrap_err();
        assert!(err.contains("empty file name"), "{err}");
    }
}
