// --validate: the command-line side of the check - which files, what is
// printed, the exit status. The check itself (the parser that finds what is
// wrong with an .ico file) lives in img2ico-core, so the web version uses
// exactly the same one.

use std::path::{Path, PathBuf};

use img2ico_core::validate::{Finding, Report, Severity, validate_bytes};

/// The files to check for the given inputs: a file stands for itself, a
/// folder for the .ico files directly inside it (in name order) - or, with
/// `recursive`, anywhere below it.
fn collect_files(inputs: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        if !input.is_dir() {
            files.push(input.clone());
            continue;
        }
        let walker = walkdir::WalkDir::new(input)
            .min_depth(1)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .sort_by_file_name();
        let before = files.len();
        for entry in walker.into_iter().filter_map(Result::ok) {
            let is_ico = entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("ico"));
            if entry.file_type().is_file() && is_ico {
                files.push(entry.into_path());
            }
        }
        if files.len() == before {
            return Err(format!(
                "No .ico files found {} folder '{}'.{}",
                if recursive {
                    "in or below"
                } else {
                    "directly in"
                },
                input.display(),
                if recursive {
                    ""
                } else {
                    " Add --recursive to search its subfolders too."
                }
            ));
        }
    }
    Ok(files)
}

/// Checks the file at `path`. A file that cannot be read at all is a result
/// like any other - one bad file must not hide the verdict on the rest.
fn check_file(path: &Path) -> Report {
    match std::fs::read(path) {
        Ok(bytes) => validate_bytes(&bytes),
        Err(e) => Report {
            file_size: 0,
            kind: 0,
            entries: Vec::new(),
            findings: vec![Finding {
                severity: Severity::Error,
                entry: None,
                message: format!("the file could not be read: {e}"),
            }],
        },
    }
}

/// The human-readable form of one file's result.
fn print_report(path: &Path, report: &Report) {
    let errors = report.count(Severity::Error);
    let warnings = report.count(Severity::Warning);
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    if report.is_valid() {
        let sizes: Vec<String> = report
            .entries
            .iter()
            .map(|e| format!("{}x{} {}", e.width, e.height, e.format.name()))
            .collect();
        let mut line = format!(
            "{}: valid - {} ({})",
            path.display(),
            plural(report.entries.len(), "image"),
            sizes.join(", ")
        );
        if warnings > 0 {
            line.push_str(&format!(", {}", plural(warnings, "warning")));
        }
        println!("{line}");
    } else {
        println!(
            "{}: INVALID - {}, {}",
            path.display(),
            plural(errors, "error"),
            plural(warnings, "warning")
        );
    }
    for finding in &report.findings {
        let label = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match finding.entry {
            Some(index) => println!("  {label}: image [{index}]: {}", finding.message),
            None => println!("  {label}: {}", finding.message),
        }
    }
}

/// The JSON form of one file's result.
fn report_json(path: &Path, report: &Report) -> serde_json::Value {
    let findings = |severity| -> Vec<serde_json::Value> {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| serde_json::json!({ "image": f.entry, "message": f.message }))
            .collect()
    };
    let entries: Vec<serde_json::Value> = report
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
        "path": path.display().to_string(),
        "valid": report.is_valid(),
        "bytes": report.file_size,
        "images": entries,
        "errors": findings(Severity::Error),
        "warnings": findings(Severity::Warning),
    })
}

/// `--validate`: checks every file (or folder of files) and prints what it
/// found. The error that is returned when any file is invalid is what turns
/// into the non-zero exit status a build script can fail on.
pub fn validate_files(inputs: &[PathBuf], recursive: bool, json: bool) -> Result<(), String> {
    let files = collect_files(inputs, recursive)?;
    let results: Vec<(PathBuf, Report)> = files
        .into_iter()
        .map(|path| {
            let report = check_file(&path);
            (path, report)
        })
        .collect();
    let invalid = results.iter().filter(|(_, r)| !r.is_valid()).count();

    if json {
        let all: Vec<serde_json::Value> = results.iter().map(|(p, r)| report_json(p, r)).collect();
        let text = serde_json::to_string_pretty(&all)
            .map_err(|e| format!("Could not build the JSON report: {e}"))?;
        println!("{text}");
    } else {
        for (path, report) in &results {
            print_report(path, report);
        }
        if results.len() > 1 {
            println!(
                "Checked {} files: {} valid, {invalid} invalid.",
                results.len(),
                results.len() - invalid
            );
        }
    }

    if invalid > 0 {
        Err(format!(
            "{invalid} of {} file(s) failed validation.",
            results.len()
        ))
    } else {
        Ok(())
    }
}
