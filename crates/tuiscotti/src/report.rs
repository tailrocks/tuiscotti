//! Static HTML comparison reports (D03, GOAL 14.3).
//!
//! [`render_report`] builds a static HTML page from saved comparison data:
//! the test ID, mismatch reason, and digests; expected, actual, and
//! difference images when present (as relative sibling links); cell
//! coordinates and changed state fields for state differences; and an
//! explicit note when no expected image exists.
//!
//! The output carries no code from samples and no web server or script:
//! text and attribute values are escaped, links are relative file names or
//! embedded data, and no absolute developer path is ever written.
//!
//! [`write_report_index`] builds the static evidence index over an
//! artifact root; its links resolve from the index parent, so the index
//! may live anywhere.

use std::fmt::{Display, Formatter, Write as _};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Every failure a report render or save can report.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReportError {
    /// The report file cannot be written.
    Io(std::io::Error),
}

impl Display for ReportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(source) => write!(f, "report I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
        }
    }
}

impl From<std::io::Error> for ReportError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

/// One changed cell for the report: coordinates, changed field names, and
/// the compared cell text on both sides (truncated for display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportedCell {
    col: u32,
    row: u32,
    fields: Vec<String>,
    actual_text: String,
    baseline_text: String,
}

impl ReportedCell {
    /// Record a changed cell at grid coordinates with its changed fields
    /// and the compared display text.
    #[must_use]
    pub fn new(
        col: u32,
        row: u32,
        fields: Vec<String>,
        actual_text: String,
        baseline_text: String,
    ) -> Self {
        Self {
            col,
            row,
            fields,
            actual_text,
            baseline_text,
        }
    }

    /// The grid column of the changed cell.
    #[must_use]
    pub const fn col(&self) -> u32 {
        self.col
    }

    /// The grid row of the changed cell.
    #[must_use]
    pub const fn row(&self) -> u32 {
        self.row
    }

    /// Changed field names for this cell, in fixed order.
    #[must_use]
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// Display text of the candidate cell.
    #[must_use]
    pub fn actual_text(&self) -> &str {
        &self.actual_text
    }

    /// Display text of the baseline cell.
    #[must_use]
    pub fn baseline_text(&self) -> &str {
        &self.baseline_text
    }
}

/// Compared-pixel summary for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSummary {
    /// Changed pixel count.
    pub changed: u64,
    /// Compared pixel count.
    pub compared: u64,
    /// Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`.
    pub bounds: [u32; 4],
}

/// Input for [`render_report`]: plain borrowed data, no filesystem reads.
#[derive(Debug, Clone, Copy)]
pub struct ReportInput<'a> {
    /// The compared test ID.
    pub test_id: &'a str,
    /// The gate wire value (`state` or `state-and-pixels`).
    pub gate: &'a str,
    /// The mismatch reason wire value.
    pub reason: &'a str,
    /// SHA-256 over the candidate sample bytes.
    pub candidate_sha256: &'a str,
    /// SHA-256 over the baseline bytes, or `missing`.
    pub baseline_sha256: &'a str,
    /// Changed top-level state fields, in fixed order.
    pub changed_fields: &'a [String],
    /// Changed cells with coordinates, row-major, bounded.
    pub changed_cells: &'a [ReportedCell],
    /// The exact changed-cell count, without the report bound.
    pub changed_cell_total: usize,
    /// Pixel summary when pixels were compared.
    pub pixels: Option<PixelSummary>,
    /// Whether `expected.png` sits beside this report.
    pub has_expected_image: bool,
    /// Whether `actual.png` sits beside this report.
    pub has_actual_image: bool,
    /// Whether `diff.png` sits beside this report.
    pub has_diff_image: bool,
    /// Compared window titles as `(actual, baseline)` when the title moved.
    pub title_values: Option<(&'a str, &'a str)>,
}

/// One completed evidence index: the static page linking every saved
/// check record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexOutcome {
    records: usize,
}

impl IndexOutcome {
    /// The indexed check record count, in deterministic sort order.
    #[must_use]
    pub const fn records(&self) -> usize {
        self.records
    }
}

/// Every failure an evidence index write can report.
#[derive(Debug)]
#[non_exhaustive]
pub enum IndexError {
    /// The artifact root cannot be walked for check evidence.
    Read {
        /// The artifact root that failed to list.
        artifacts: PathBuf,
        /// The filesystem failure.
        source: std::io::Error,
    },
    /// The static index page cannot be written.
    Write {
        /// The index path that failed.
        path: PathBuf,
        /// The filesystem failure.
        source: std::io::Error,
    },
}

impl Display for IndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { artifacts, source } => {
                write!(f, "cannot read artifacts {}: {source}", artifacts.display())
            }
            Self::Write { path, source } => {
                write!(f, "cannot write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
        }
    }
}

impl IndexError {
    /// The stable Section 16.3 machine code: every index failure is `IO`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "IO"
    }
}

/// Walk the artifact root for check `result.json` records (schema
/// `tuiscotti.check.v1`), sort them deterministically, and write one
/// self-contained HTML page linking each check directory's own
/// `report.html`. Links resolve from the index parent, so the index may
/// live anywhere; only forward slashes are emitted. Nothing here
/// modifies approvals: the baseline root is never opened.
///
/// # Errors
///
/// Returns [`IndexError`] when the artifact root cannot be walked or
/// the index page cannot be written.
pub fn write_report_index(artifacts: &Path, out: &Path) -> Result<IndexOutcome, IndexError> {
    let records = collect_records(artifacts).map_err(|source| IndexError::Read {
        artifacts: artifacts.to_path_buf(),
        source,
    })?;
    let page = render_index(artifacts, out, &records);
    std::fs::write(out, page.as_bytes()).map_err(|source| IndexError::Write {
        path: out.to_path_buf(),
        source,
    })?;
    Ok(IndexOutcome {
        records: records.len(),
    })
}

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
    if value.get("schema")?.as_str()? != crate::check::CHECK_SCHEMA {
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

/// Render the static index page: escaped text, links relative to the
/// index parent, no absolute developer paths and no scripts.
fn render_index(artifacts: &Path, out: &Path, records: &[IndexedRecord]) -> String {
    let parent = out.parent().filter(|parent| !parent.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    let mut page = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\n<title>tuiscotti report</title>\n</head>\n<body>\n<h1>tuiscotti report</h1>\n",
    );
    push_index_line(&mut page, &format!("<p>records: {}</p>", records.len()));
    push_index_line(&mut page, "<table>");
    push_index_line(
        &mut page,
        "<tr><th>test</th><th>reason</th><th>candidate</th><th>baseline</th><th>evidence</th></tr>",
    );
    for record in records {
        let test_id = escape_html(&record.test_id);
        let reason = escape_html(&record.reason);
        let candidate = escape_html(&record.candidate_sha256);
        let baseline = escape_html(&record.baseline_sha256);
        let target = artifacts.join(&record.check_dir).join("report.html");
        let href = escape_html(&relative_href(parent, &target));
        push_index_line(
            &mut page,
            &format!(
                "<tr><td>{test_id}</td><td>{reason}</td><td>{candidate}</td><td>{baseline}</td><td><a href=\"{href}\">report</a></td></tr>"
            ),
        );
    }
    push_index_line(&mut page, "</table>");
    push_index_line(&mut page, "</body>");
    push_index_line(&mut page, "</html>");
    page
}

/// Append one `\n`-terminated line to an index buffer.
fn push_index_line(buffer: &mut String, line: &str) {
    buffer.push_str(line);
    buffer.push('\n');
}

/// Href from the index parent to an evidence file, with forward
/// slashes on every platform. Both sides resolve lexically against the
/// process directory, so no filesystem access decides the spelling.
fn relative_href(index_parent: &Path, target: &Path) -> String {
    let base = normalize_lexical(&absolutize(index_parent));
    let target = normalize_lexical(&absolutize(target));
    if let Some(relative) = relative_lexical(&base, &target) {
        return relative;
    }
    target.to_string_lossy().replace('\\', "/")
}

/// Join a relative path onto the process directory; absolute paths
/// pass through unchanged.
fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// Resolve `.` and `..` lexically, without touching the filesystem.
fn normalize_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut stack: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(stack.last(), Some(Component::Normal(_))) {
                    stack.pop();
                } else {
                    stack.push(component);
                }
            }
            _ => stack.push(component),
        }
    }
    let mut normalized = PathBuf::new();
    for component in stack {
        normalized.push(component);
    }
    normalized
}

/// Lexical path from `base` to `target`, joined with forward slashes;
/// `None` when the two sides share no comparable root (drive-letter
/// splits on Windows).
fn relative_lexical(base: &Path, target: &Path) -> Option<String> {
    use std::path::Component;
    let base: Vec<Component<'_>> = base.components().collect();
    let target: Vec<Component<'_>> = target.components().collect();
    match (base.first(), target.first()) {
        (Some(Component::Prefix(_)), Some(Component::Prefix(_))) if base[0] != target[0] => {
            return None;
        }
        _ => {}
    }
    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts: Vec<String> = Vec::new();
    for component in &base[common..] {
        if matches!(component, Component::Normal(_)) {
            parts.push("..".to_string());
        }
    }
    for component in &target[common..] {
        match component {
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            Component::ParentDir => parts.push("..".to_string()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    if parts.is_empty() {
        return Some("report.html".to_string());
    }
    Some(parts.join("/"))
}

/// Render the static HTML report for one comparison as a string.
#[must_use]
pub fn render_report(input: &ReportInput<'_>) -> String {
    let mut html = String::new();
    push_header(&mut html, input);
    push_summary(&mut html, input);
    push_state(&mut html, input);
    push_images(&mut html, input);
    push_footer(&mut html);
    html
}

/// Render and save the static HTML report through one atomic file write: a
/// temporary file in the destination directory, flushed and closed before
/// one atomic rename, like all sibling evidence.
///
/// # Errors
///
/// Returns [`ReportError`] when the report path has no file name or the
/// report file cannot be written.
pub fn write_report(input: &ReportInput<'_>, path: &Path) -> Result<(), ReportError> {
    let html = render_report(input);
    let invalid = |what: &str| {
        ReportError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("report path {} has {what}", path.display()),
        ))
    };
    let dir = path
        .parent()
        .ok_or_else(|| invalid("no parent directory"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("no file name"))?;
    atomic_write(dir, name, html.as_bytes()).map_err(ReportError::Io)?;
    Ok(())
}

/// A temporary file that removes itself on drop unless committed. Writes go
/// to the destination directory, flush before the handle closes, and reach
/// their name through one atomic rename.
struct TempFile {
    path: Option<PathBuf>,
}

impl TempFile {
    /// Create a uniquely named temporary file in a directory.
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_name = format!(".tmp-{}-{id}-{name}", std::process::id());
        let path = dir.join(temp_name);
        let file = fs::File::create(&path)?;
        Ok((file, Self { path: Some(path) }))
    }

    /// Flush, close, and atomically rename the temporary file.
    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary evidence file is already committed",
            ));
        };
        fs::rename(&path, target)?;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            drop(fs::remove_file(path));
        }
    }
}

/// Write bytes through a temporary file with flush, close, and atomic
/// rename. Every evidence write, including the HTML report, goes through
/// this helper so readers never see a partial file.
pub(crate) fn atomic_write(
    dir: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<PathBuf, std::io::Error> {
    let target = dir.join(name);
    let (mut file, temp) = TempFile::create(dir, name)?;
    file.write_all(bytes)?;
    file.flush()?;
    drop(file);
    temp.commit(&target)?;
    Ok(target)
}

/// Escape text and attribute values for HTML: no code from a sample runs.
#[must_use]
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for glyph in text.chars() {
        match glyph {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(glyph),
        }
    }
    out
}

/// Write the document head: escaped title, no script, no absolute path.
fn push_header(html: &mut String, input: &ReportInput<'_>) {
    let title = escape_html(input.test_id);
    let _result = write!(
        html,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>tuiscotti check: {title}</title>\n</head>\n<body>\n<h1>tuiscotti check: {title}</h1>\n"
    );
}

/// Write the outcome summary: reason, gate, and both digests escaped.
fn push_summary(html: &mut String, input: &ReportInput<'_>) {
    let _result = write!(
        html,
        "<h2>Outcome</h2>\n<dl>\n<dt>test ID</dt><dd>{}</dd>\n<dt>gate</dt><dd>{}</dd>\n<dt>reason</dt><dd>{}</dd>\n<dt>candidate SHA-256</dt><dd>{}</dd>\n<dt>baseline SHA-256</dt><dd>{}</dd>\n</dl>\n",
        escape_html(input.test_id),
        escape_html(input.gate),
        escape_html(input.reason),
        escape_html(input.candidate_sha256),
        escape_html(input.baseline_sha256),
    );
}

/// Write changed fields, values, and cell coordinates with an exact total.
fn push_state(html: &mut String, input: &ReportInput<'_>) {
    html.push_str("<h2>State differences</h2>\n");
    if input.changed_fields.is_empty() && input.changed_cell_total == 0 {
        html.push_str("<p>No state field changed.</p>\n");
        push_pixels(html, input);
    } else {
        html.push_str("<h3>Changed fields</h3>\n<ul>\n");
        for field in input.changed_fields {
            let _result = writeln!(html, "<li>{}</li>", escape_html(field));
        }
        html.push_str("</ul>\n");
        push_title_values(html, input);
        push_cells(html, input);
        push_pixels(html, input);
    }
}

/// Write the compared window titles when the title moved.
fn push_title_values(html: &mut String, input: &ReportInput<'_>) {
    if let Some((actual, baseline)) = input.title_values {
        let _result = write!(
            html,
            "<h3>Changed values</h3>\n<table>\n<tr><th>field</th><th>candidate</th><th>baseline</th></tr>\n<tr><td>title</td><td>{}</td><td>{}</td></tr>\n</table>\n",
            escape_html(actual),
            escape_html(baseline),
        );
    }
}

/// Write the changed-cell table with coordinates and display text.
fn push_cells(html: &mut String, input: &ReportInput<'_>) {
    let _result = write!(
        html,
        "<p>Changed cells: {}</p>\n<table>\n<tr><th>column</th><th>row</th><th>fields</th><th>candidate text</th><th>baseline text</th></tr>\n",
        input.changed_cell_total
    );
    for cell in input.changed_cells {
        let fields: Vec<String> = cell
            .fields()
            .iter()
            .map(|field| escape_html(field))
            .collect();
        let _result = writeln!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            cell.col(),
            cell.row(),
            fields.join(", "),
            escape_html(cell.actual_text()),
            escape_html(cell.baseline_text()),
        );
    }
    html.push_str("</table>\n");
}

/// Write the compared-pixel summary when pixels were compared.
fn push_pixels(html: &mut String, input: &ReportInput<'_>) {
    if let Some(pixels) = input.pixels {
        let bounds = pixels.bounds;
        let _result = writeln!(
            html,
            "<p>Changed pixels: {} of {} compared; bounds ({}-{}, {}-{}).</p>",
            pixels.changed, pixels.compared, bounds[0], bounds[1], bounds[2], bounds[3],
        );
    }
}

/// Write image links with relative sibling names, or explicit missing notes.
fn push_images(html: &mut String, input: &ReportInput<'_>) {
    html.push_str("<h2>Images</h2>\n");
    push_image(html, "Expected", "expected.png", input.has_expected_image);
    push_image(html, "Actual", "actual.png", input.has_actual_image);
    push_image(html, "Difference", "diff.png", input.has_diff_image);
}

/// Write one image row: a relative link when present, an explicit note when
/// no expected image exists or an image does not apply to this gate.
fn push_image(html: &mut String, label: &str, file: &str, present: bool) {
    if present {
        let _result = write!(
            html,
            "<h3>{label}</h3>\n<img src=\"{file}\" alt=\"{label} image\">\n"
        );
    } else if label == "Expected" {
        html.push_str(
            "<h3>Expected</h3>\n<p>The expected image is missing: no baseline image was compared.</p>\n",
        );
    } else {
        let _result = write!(
            html,
            "<h3>{label}</h3>\n<p>The {label} image is missing for this comparison.</p>\n"
        );
    }
}

/// Close the document: no JavaScript application follows.
fn push_footer(html: &mut String) {
    html.push_str("</body>\n</html>\n");
}

#[cfg(test)]
mod tests {
    use super::{ReportInput, escape_html, relative_href, render_report, write_report};

    #[test]
    fn escaping_keeps_markup_inert() {
        let hostile = "<script>alert('x')</script>";
        assert_eq!(
            escape_html(hostile),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("a&b\"c"), "a&amp;b&quot;c");
    }

    #[test]
    fn report_states_a_missing_expected_image() {
        let fields: Vec<String> = Vec::new();
        let cells = Vec::new();
        let input = ReportInput {
            test_id: "views/login",
            gate: "state",
            reason: "missing",
            candidate_sha256: "aa",
            baseline_sha256: "missing",
            changed_fields: &fields,
            changed_cells: &cells,
            changed_cell_total: 0,
            pixels: None,
            has_expected_image: false,
            has_actual_image: false,
            has_diff_image: false,
            title_values: None,
        };
        let html = render_report(&input);
        assert!(html.contains("The expected image is missing"));
        assert!(!html.contains("<script"));
    }

    /// The report write is atomic: the saved bytes equal the full rendering
    /// and no temporary file is left behind in the destination directory.
    #[test]
    fn write_report_saves_complete_bytes_without_temp_residue() {
        let dir = tempfile::tempdir().expect("tempdir creates");
        let fields = vec!["title".to_string()];
        let cells = Vec::new();
        let input = ReportInput {
            test_id: "views/login",
            gate: "state",
            reason: "state",
            candidate_sha256: "cc",
            baseline_sha256: "bb",
            changed_fields: &fields,
            changed_cells: &cells,
            changed_cell_total: 0,
            pixels: None,
            has_expected_image: false,
            has_actual_image: false,
            has_diff_image: false,
            title_values: None,
        };
        let path = dir.path().join("report.html");
        write_report(&input, &path).expect("report writes");
        let saved = std::fs::read(&path).expect("report reads");
        assert_eq!(saved, render_report(&input).as_bytes());
        let text = String::from_utf8(saved).expect("report is UTF-8");
        assert!(text.starts_with("<!DOCTYPE html>"));
        assert!(text.ends_with("</html>\n"));
        for entry in std::fs::read_dir(dir.path()).expect("directory lists") {
            let name = entry
                .expect("entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned();
            assert!(
                !name.starts_with(".tmp-"),
                "temporary file {name:?} left behind"
            );
        }
    }

    /// Index hrefs resolve from the index parent with forward slashes.
    #[test]
    fn index_hrefs_resolve_from_the_index_parent() {
        use std::path::Path;
        let cases = [
            (
                "/repo/out",
                "/repo/artifacts/suite/digest/report.html",
                "../artifacts/suite/digest/report.html",
            ),
            (
                "/repo/artifacts",
                "/repo/artifacts/suite/digest/report.html",
                "suite/digest/report.html",
            ),
            (
                "/repo/artifacts/sub",
                "/repo/artifacts/suite/digest/report.html",
                "../suite/digest/report.html",
            ),
            (
                "/repo/artifacts/suite/digest",
                "/repo/artifacts/suite/digest/report.html",
                "report.html",
            ),
        ];
        for (parent, target, expected) in cases {
            assert_eq!(
                relative_href(Path::new(parent), Path::new(target)),
                expected,
                "href from {parent} to {target}"
            );
        }
    }
}
