// What a run says about itself afterwards: one record per file, the totals
// over them, and the --report file (CSV or JSON) they can be written to.
// Kept apart from the conversion so the formats can be tested without
// converting anything.

use serde::Serialize;
use std::path::Path;
use std::time::Duration;

/// What became of one input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Converted,
    Skipped,
    Failed,
    /// A --find preview: the report was printed, nothing was written.
    Previewed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Converted => "converted",
            Status::Skipped => "skipped",
            Status::Failed => "failed",
            Status::Previewed => "previewed",
        }
    }
}

/// The report line for one input.
#[derive(Debug, Clone, Serialize)]
pub struct FileRecord {
    pub input: String,
    /// Where the icon goes - or would have gone, for a skipped or failed file.
    pub output: String,
    pub status: Status,
    /// Size of the written icon file (converted files only).
    pub size_bytes: Option<u64>,
    /// The icon sizes in the file (converted files only).
    pub sizes: Vec<u32>,
    /// How many warnings this file raised.
    pub warnings: usize,
    pub duration_ms: u64,
    /// The error, or the reason a file was skipped.
    pub message: Option<String>,
}

/// Totals over all records of a run.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Summary {
    pub total: usize,
    pub converted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub warnings: usize,
    /// Combined size of the icon files written.
    pub output_bytes: u64,
    pub duration_ms: u64,
}

impl Summary {
    pub fn of(records: &[FileRecord], elapsed: Duration) -> Self {
        let count = |status| records.iter().filter(|r| r.status == status).count();
        Self {
            total: records.len(),
            converted: count(Status::Converted),
            // A preview wrote nothing either; it is closest to "skipped".
            skipped: count(Status::Skipped) + count(Status::Previewed),
            failed: count(Status::Failed),
            warnings: records.iter().map(|r| r.warnings).sum(),
            output_bytes: records.iter().filter_map(|r| r.size_bytes).sum(),
            duration_ms: elapsed.as_millis() as u64,
        }
    }
}

/// A file size for people: "812 B", "42 KB", "1.3 MB" (1 KB = 1024 bytes).
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if value < KB * KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{:.1} MB", value / (KB * KB))
    }
}

/// A duration for people: "850 ms" below a second, "3.4 s" above.
pub fn human_duration(millis: u64) -> String {
    if millis < 1000 {
        format!("{millis} ms")
    } else {
        format!("{:.1} s", millis as f64 / 1000.0)
    }
}

/// The format of a --report file, taken from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Csv,
    Json,
}

impl ReportFormat {
    /// Only `.csv` and `.json` are accepted. That also means a typo such as
    /// `--report logo.png` can never overwrite an image.
    pub fn of(path: &Path) -> Result<Self, String> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);
        match extension.as_deref() {
            Some("csv") => Ok(Self::Csv),
            Some("json") => Ok(Self::Json),
            _ => Err(format!(
                "--report '{}': the file name must end in .csv or .json - that chooses the format.",
                path.display()
            )),
        }
    }
}

/// One CSV field, quoted when it has to be (RFC 4180).
fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// The report as CSV text: a header line, then one line per file.
pub fn to_csv(records: &[FileRecord]) -> String {
    let mut out =
        String::from("input,output,status,size_bytes,sizes,warnings,duration_ms,message\n");
    for record in records {
        let sizes = record
            .sizes
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let fields = [
            csv_field(&record.input),
            csv_field(&record.output),
            record.status.as_str().to_string(),
            record.size_bytes.map(|b| b.to_string()).unwrap_or_default(),
            sizes,
            record.warnings.to_string(),
            record.duration_ms.to_string(),
            csv_field(record.message.as_deref().unwrap_or("")),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

#[derive(Serialize)]
struct JsonReport<'a> {
    summary: &'a Summary,
    files: &'a [FileRecord],
}

/// The report as pretty-printed JSON: `{"summary": {...}, "files": [...]}`.
pub fn to_json(records: &[FileRecord], summary: &Summary) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(&JsonReport {
        summary,
        files: records,
    })
    .map_err(|e| format!("Could not build the JSON report: {e}"))?;
    text.push('\n');
    Ok(text)
}

/// Writes the report to `path`, in the format its extension names. An
/// existing file is replaced: a report describes the latest run.
pub fn write_report(path: &Path, records: &[FileRecord], summary: &Summary) -> Result<(), String> {
    let text = match ReportFormat::of(path)? {
        ReportFormat::Csv => to_csv(records),
        ReportFormat::Json => to_json(records, summary)?,
    };
    crate::util::write_atomic(path, text.as_bytes())
        .map_err(|e| format!("Could not write the report '{}': {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(input: &str, status: Status) -> FileRecord {
        FileRecord {
            input: input.to_string(),
            output: format!("{input}.ico"),
            status,
            size_bytes: None,
            sizes: Vec::new(),
            warnings: 0,
            duration_ms: 5,
            message: None,
        }
    }

    fn converted(input: &str, bytes: u64, warnings: usize) -> FileRecord {
        FileRecord {
            size_bytes: Some(bytes),
            sizes: vec![16, 32],
            warnings,
            ..record(input, Status::Converted)
        }
    }

    #[test]
    fn human_sizes_pick_a_sensible_unit() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(812), "812 B");
        assert_eq!(human_size(1024), "1 KB");
        assert_eq!(human_size(43_008), "42 KB");
        assert_eq!(human_size(1_363_149), "1.3 MB");
    }

    #[test]
    fn human_durations_switch_at_a_second() {
        assert_eq!(human_duration(850), "850 ms");
        assert_eq!(human_duration(3_400), "3.4 s");
    }

    #[test]
    fn the_summary_counts_by_status_and_adds_up_the_rest() {
        let mut failed = record("c.png", Status::Failed);
        failed.warnings = 1;
        let records = vec![
            converted("a.png", 1000, 2),
            converted("b.png", 500, 0),
            failed,
            record("d.png", Status::Skipped),
        ];
        let summary = Summary::of(&records, Duration::from_millis(1234));
        assert_eq!(
            summary,
            Summary {
                total: 4,
                converted: 2,
                skipped: 1,
                failed: 1,
                warnings: 3,
                output_bytes: 1500,
                duration_ms: 1234,
            }
        );
    }

    #[test]
    fn a_preview_counts_as_skipped() {
        let summary = Summary::of(&[record("a.png", Status::Previewed)], Duration::ZERO);
        assert_eq!((summary.skipped, summary.converted), (1, 0));
    }

    #[test]
    fn the_report_format_comes_from_the_extension() {
        assert_eq!(ReportFormat::of(Path::new("r.csv")), Ok(ReportFormat::Csv));
        assert_eq!(
            ReportFormat::of(Path::new("r.JSON")),
            Ok(ReportFormat::Json)
        );
        for bad in ["r.png", "r.txt", "report"] {
            let err = ReportFormat::of(Path::new(bad)).unwrap_err();
            assert!(err.contains(".csv or .json"), "{err}");
        }
    }

    #[test]
    fn csv_has_a_header_and_one_line_per_file() {
        let csv = to_csv(&[
            converted("a.png", 1000, 1),
            record("b.png", Status::Skipped),
        ]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "input,output,status,size_bytes,sizes,warnings,duration_ms,message"
        );
        assert_eq!(lines[1], "a.png,a.png.ico,converted,1000,16 32,1,5,");
        assert_eq!(lines[2], "b.png,b.png.ico,skipped,,,0,5,");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn csv_quotes_fields_that_need_it() {
        let mut tricky = record("my, \"odd\" name.png", Status::Failed);
        tricky.message = Some("line one\nline two".to_string());
        let csv = to_csv(&[tricky]);
        assert!(csv.contains("\"my, \"\"odd\"\" name.png\""), "{csv}");
        assert!(csv.contains("\"line one\nline two\""), "{csv}");
    }

    #[test]
    fn json_holds_a_summary_and_the_files() {
        let records = vec![converted("a.png", 1000, 0), record("b.png", Status::Failed)];
        let summary = Summary::of(&records, Duration::from_millis(10));
        let value: serde_json::Value =
            serde_json::from_str(&to_json(&records, &summary).unwrap()).unwrap();
        assert_eq!(value["summary"]["total"], 2);
        assert_eq!(value["summary"]["converted"], 1);
        assert_eq!(value["summary"]["failed"], 1);
        assert_eq!(value["summary"]["output_bytes"], 1000);
        assert_eq!(value["files"][0]["input"], "a.png");
        assert_eq!(value["files"][0]["status"], "converted");
        assert_eq!(value["files"][0]["sizes"], serde_json::json!([16, 32]));
        assert_eq!(value["files"][1]["status"], "failed");
        assert!(value["files"][1]["size_bytes"].is_null());
    }

    #[test]
    fn write_report_replaces_an_existing_file_and_refuses_other_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.csv");
        std::fs::write(&path, "old").unwrap();
        let records = [converted("a.png", 1, 0)];
        write_report(&path, &records, &Summary::default()).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .starts_with("input,")
        );

        let other = dir.path().join("r.png");
        assert!(write_report(&other, &records, &Summary::default()).is_err());
        assert!(!other.exists());
    }
}
