// Small, general-purpose helpers that don't belong to any one feature:
// parsing a "--seed" value, the shared overwrite-protection check, and
// safe source-file deletion for --delete-source.

use std::path::{Path, PathBuf};

/// Parses a "--seed" value in the format "x,y" (e.g. "200,50") into a
/// coordinate pair. Returns an understandable error message if the format
/// doesn't match.
pub fn parse_seed(input: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.len() != 2 {
        return Err(format!(
            "'{input}' is not a valid seed point (expected format is 'x,y', e.g. '200,50')"
        ));
    }
    let x = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("'{}' is not a valid x coordinate", parts[0]))?;
    let y = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("'{}' is not a valid y coordinate", parts[1]))?;
    Ok((x, y))
}

/// Refuses to silently overwrite an existing file: returns an error
/// (naming --force as the way around it) if `path` already exists and
/// `force` is false. A no-op if `force` is true, or if nothing exists at
/// `path` yet, so callers can just unconditionally call this right before
/// writing.
pub fn check_overwrite(path: &Path, force: bool) -> Result<(), String> {
    if !force && path.exists() {
        return Err(format!(
            "'{}' already exists. Use --force to overwrite it.",
            path.display()
        ));
    }
    Ok(())
}

/// Checks whether two paths point at the same file on disk, resolving
/// symlinks/relative components first (`/a/./b.png` and `/a/b.png` should
/// count as "the same file" even though they're different strings).
/// Falls back to plain path equality if either path can't be resolved
/// (e.g. because it doesn't exist) - a non-existent path can't be "the
/// same file" as anything by resolution anyway, so plain comparison is a
/// reasonable fallback.
fn paths_refer_to_same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Deletes the given source files as part of --delete-source, skipping
/// (with a warning) any file that is also the resolved output path -
/// that would otherwise delete the just-written result instead of a
/// source file. `output_path` is `None` for extract mode, where the
/// output is a directory and can never collide with a source file path.
///
/// A failed deletion (e.g. permission denied) only prints a warning and
/// does not turn the overall command into a failure: the actual
/// conversion/merge/extraction the user asked for already succeeded by
/// the time this runs, so a cleanup problem afterward shouldn't make the
/// whole command look like it failed.
pub fn delete_source_files(paths: &[PathBuf], output_path: Option<&Path>, silent: bool) {
    for path in paths {
        if let Some(output_path) = output_path
            && paths_refer_to_same_file(path, output_path)
        {
            if !silent {
                eprintln!(
                    "Not deleting '{}': it's also the output path.",
                    path.display()
                );
            }
            continue;
        }
        match std::fs::remove_file(path) {
            Ok(()) => println!("Deleted source file '{}'.", path.display()),
            Err(e) => {
                if !silent {
                    eprintln!("Warning: could not delete '{}': {e}", path.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // --- parse_seed ------------------------------------------------------------

    #[test]
    fn parse_seed_accepts_a_plain_pair() {
        assert_eq!(parse_seed("200,50"), Ok((200, 50)));
        assert_eq!(parse_seed("0,0"), Ok((0, 0)));
    }

    #[test]
    fn parse_seed_trims_whitespace_around_the_numbers() {
        assert_eq!(parse_seed(" 10 , 20 "), Ok((10, 20)));
    }

    #[test]
    fn parse_seed_rejects_the_wrong_number_of_parts() {
        assert!(parse_seed("").is_err());
        assert!(parse_seed("5").is_err());
        assert!(parse_seed("1,2,3").is_err());
        assert!(parse_seed("1,").is_err());
        assert!(parse_seed(",2").is_err());
    }

    #[test]
    fn parse_seed_rejects_non_numeric_and_negative_values() {
        assert!(parse_seed("a,1").is_err());
        assert!(parse_seed("1,b").is_err());
        assert!(parse_seed("-1,2").is_err());
        assert!(parse_seed("1.5,2").is_err());
    }

    #[test]
    fn parse_seed_rejects_values_beyond_u32() {
        assert!(parse_seed("4294967296,0").is_err());
    }

    #[test]
    fn parse_seed_error_names_the_bad_coordinate() {
        assert!(parse_seed("x,1").unwrap_err().contains("x coordinate"));
        assert!(parse_seed("1,y").unwrap_err().contains("y coordinate"));
    }

    // --- check_overwrite -------------------------------------------------------

    #[test]
    fn overwrite_check_passes_for_a_path_that_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(check_overwrite(&dir.path().join("new.ico"), false), Ok(()));
    }

    #[test]
    fn overwrite_check_refuses_an_existing_file_and_mentions_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.ico");
        std::fs::write(&path, b"x").unwrap();
        let err = check_overwrite(&path, false).unwrap_err();
        assert!(err.contains("--force"), "unexpected message: {err}");
        assert!(err.contains("existing.ico"), "unexpected message: {err}");
    }

    #[test]
    fn overwrite_check_allows_an_existing_file_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.ico");
        std::fs::write(&path, b"x").unwrap();
        assert_eq!(check_overwrite(&path, true), Ok(()));
    }

    // --- paths_refer_to_same_file ----------------------------------------------

    #[test]
    fn same_file_is_recognized_through_different_spellings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.png");
        std::fs::write(&file, b"x").unwrap();
        let roundabout = dir.path().join(".").join("a.png");
        assert!(paths_refer_to_same_file(&file, &roundabout));
    }

    #[test]
    fn different_files_are_not_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        assert!(!paths_refer_to_same_file(&a, &b));
    }

    #[test]
    fn nonexistent_paths_fall_back_to_plain_comparison() {
        let a = Path::new("does/not/exist.png");
        assert!(paths_refer_to_same_file(a, a));
        assert!(!paths_refer_to_same_file(a, Path::new("does/not/exist2.png")));
    }

    // --- delete_source_files ---------------------------------------------------

    #[test]
    fn delete_source_removes_the_given_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        delete_source_files(&[a.clone(), b.clone()], None, true);
        assert!(!a.exists() && !b.exists());
    }

    #[test]
    fn delete_source_never_deletes_the_output_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("in.png");
        let output = dir.path().join("out.ico");
        std::fs::write(&source, b"x").unwrap();
        std::fs::write(&output, b"x").unwrap();
        // The output is listed among the "sources" (as happens for --merge
        // when -o points at one of the inputs).
        delete_source_files(&[source.clone(), output.clone()], Some(&output), true);
        assert!(!source.exists());
        assert!(output.exists(), "the output file must be protected");
    }

    #[test]
    fn delete_source_survives_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone.png");
        let present = dir.path().join("here.png");
        std::fs::write(&present, b"x").unwrap();
        delete_source_files(&[missing, present.clone()], None, true);
        assert!(!present.exists(), "the failure on one file must not stop the others");
    }

    proptest! {
        /// parse_seed must return a normal Ok/Err for absolutely any
        /// string - never panic - for the same reason as
        /// chroma_key::parse_hex_color: it takes raw command-line text
        /// (--seed) with no restrictions on what it can contain.
        #[test]
        fn parse_seed_never_panics(input in ".*") {
            let _ = parse_seed(&input);
        }

        /// Round-trip property: any coordinate pair, formatted as "x,y"
        /// the way --seed expects it, must parse back to exactly that
        /// same pair.
        #[test]
        fn parse_seed_roundtrip(x: u32, y: u32) {
            let input = format!("{x},{y}");
            prop_assert_eq!(parse_seed(&input), Ok((x, y)));
        }
    }
}