//! `report`: a static HTML index from saved candidate evidence.
//!
//! Walks the artifact root for check `result.json` records (schema
//! `tuiscotti.check.v1`), sorts them deterministically, and writes one
//! self-contained HTML page linking to each check directory's own
//! `report.html`. Nothing here captures state or changes approvals: the
//! baseline root is never even opened.

use std::path::{Path, PathBuf};

use tuiscotti::check::CHECK_SCHEMA;
use tuiscotti::report::escape_html;

use crate::json_out::{CliResult, emit, encode_path};

/// One indexed check record: identity plus its relative check directory.
struct IndexedRecord {
    test_id: String,
    reason: String,
    candidate_sha256: String,
    baseline_sha256: String,
    check_dir: PathBuf,
}

/// Collect check records under `artifacts`, sorted by test ID, candidate
/// digest, and baseline digest for deterministic output.
fn collect_records(artifacts: &Path) -> Result<Vec<IndexedRecord>, std::io::Error> {
    let mut records = Vec::new();
    let mut stack = vec![artifacts.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            entries.push(entry?.path());
        }
        entries.sort();
        for path in entries {
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "result.json")
                && let Some(record) = read_record(&path, artifacts)
            {
                records.push(record);
            }
        }
    }
    records.sort_by(|left, right| {
        (&left.test_id, &left.candidate_sha256, &left.baseline_sha256).cmp(&(
            &right.test_id,
            &right.candidate_sha256,
            &right.baseline_sha256,
        ))
    });
    Ok(records)
}

/// Read one check record; returns `None` for files that are not check
/// evidence (foreign files never break the index).
fn read_record(path: &Path, artifacts: &Path) -> Option<IndexedRecord> {
    let bytes = std::fs::read(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if value.get("schema")?.as_str()? != CHECK_SCHEMA {
        return None;
    }
    let check_dir = path.parent()?.strip_prefix(artifacts).ok()?.to_path_buf();
    Some(IndexedRecord {
        test_id: value.get("test_id")?.as_str()?.to_string(),
        reason: value.get("reason")?.as_str()?.to_string(),
        candidate_sha256: value.get("candidate_sha256")?.as_str()?.to_string(),
        baseline_sha256: value.get("baseline_sha256")?.as_str()?.to_string(),
        check_dir,
    })
}

/// Render the static index page: escaped text, relative links only, no
/// absolute developer paths and no scripts.
fn render_index(records: &[IndexedRecord]) -> String {
    let mut page = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\n<title>tuiscotti report</title>\n</head>\n<body>\n<h1>tuiscotti report</h1>\n",
    );
    crate::push_line(&mut page, &format!("<p>records: {}</p>", records.len()));
    crate::push_line(&mut page, "<table>");
    crate::push_line(
        &mut page,
        "<tr><th>test</th><th>reason</th><th>candidate</th><th>baseline</th><th>evidence</th></tr>",
    );
    for record in records {
        let test_id = escape_html(&record.test_id);
        let reason = escape_html(&record.reason);
        let candidate = escape_html(&record.candidate_sha256);
        let baseline = escape_html(&record.baseline_sha256);
        let link = record.check_dir.join("report.html");
        let href = escape_html(&link.to_string_lossy());
        crate::push_line(
            &mut page,
            &format!(
                "<tr><td>{test_id}</td><td>{reason}</td><td>{candidate}</td><td>{baseline}</td><td><a href=\"{href}\">report</a></td></tr>"
            ),
        );
    }
    crate::push_line(&mut page, "</table>");
    crate::push_line(&mut page, "</body>");
    crate::push_line(&mut page, "</html>");
    page
}

/// Run `report` and return the process exit code.
pub(crate) fn cmd_report(artifacts: &Path, out: &Path, as_json: bool) -> i32 {
    let failed = |reason: String| {
        eprintln!("error: {reason}");
        let result = CliResult::new(
            "report",
            "error",
            None,
            Some(reason),
            None,
            None,
            None,
            Some(encode_path(out)),
            Some("IO".to_string()),
        );
        emit(&result, 3, as_json)
    };
    let records = match collect_records(artifacts) {
        Ok(records) => records,
        Err(error) => {
            return failed(format!(
                "cannot read artifacts {}: {error}",
                artifacts.display()
            ));
        }
    };
    let page = render_index(&records);
    if let Err(error) = std::fs::write(out, page.as_bytes()) {
        return failed(format!("cannot write {}: {error}", out.display()));
    }
    let result = CliResult::new(
        "report",
        "pass",
        None,
        None,
        None,
        None,
        None,
        Some(encode_path(out)),
        None,
    );
    emit(&result, 0, as_json)
}
