#[doc = include_str!("../docs/report.md")]
use std::fmt::{Display, Formatter, Write as _};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[doc = "Every failure a report render or save can report."]
#[derive(Debug)]
#[non_exhaustive]
pub enum ReportError {
    #[doc = "The report file cannot be written."]
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

#[doc = "One changed cell for the report: coordinates, changed field names, and the compared cell text."]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportedCell {
    #[doc = "The grid column of the changed cell."]
    col: u32,
    #[doc = "The grid row of the changed cell."]
    row: u32,
    #[doc = "Changed field names for this cell, in fixed order."]
    fields: Vec<String>,
    #[doc = "Display text of the candidate cell."]
    actual_text: String,
    #[doc = "Display text of the baseline cell."]
    baseline_text: String,
}

impl ReportedCell {
    #[doc = "Record a changed cell at grid coordinates with its changed fields and the compared display text."]
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

    #[doc = "The grid column of the changed cell."]
    #[must_use]
    pub const fn col(&self) -> u32 {
        self.col
    }

    #[doc = "The grid row of the changed cell."]
    #[must_use]
    pub const fn row(&self) -> u32 {
        self.row
    }

    #[doc = "Changed field names for this cell, in fixed order."]
    #[must_use]
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    #[doc = "Display text of the candidate cell."]
    #[must_use]
    pub fn actual_text(&self) -> &str {
        &self.actual_text
    }

    #[doc = "Display text of the baseline cell."]
    #[must_use]
    pub fn baseline_text(&self) -> &str {
        &self.baseline_text
    }
}

#[doc = "Compared-pixel summary for the report."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSummary {
    #[doc = "Changed pixel count."]
    pub changed: u64,
    #[doc = "Compared pixel count."]
    pub compared: u64,
    #[doc = "Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`."]
    pub bounds: [u32; 4],
}

#[doc = "Input for `render_report`: plain borrowed data, no filesystem reads."]
#[derive(Debug, Clone, Copy)]
pub struct ReportInput<'a> {
    #[doc = "The compared test ID."]
    pub test_id: &'a str,
    #[doc = "The gate wire value (`state` or `state-and-pixels`)."]
    pub gate: &'a str,
    #[doc = "The mismatch reason wire value."]
    pub reason: &'a str,
    #[doc = "SHA-256 over the candidate sample bytes."]
    pub candidate_sha256: &'a str,
    #[doc = "SHA-256 over the baseline bytes, or `missing`."]
    pub baseline_sha256: &'a str,
    #[doc = "Changed top-level state fields, in fixed order."]
    pub changed_fields: &'a [String],
    #[doc = "Changed cells with coordinates, row-major, bounded."]
    pub changed_cells: &'a [ReportedCell],
    #[doc = "The exact changed-cell count, without the report bound."]
    pub changed_cell_total: usize,
    #[doc = "Pixel summary when pixels were compared."]
    pub pixels: Option<PixelSummary>,
    #[doc = "Whether `expected.png` sits beside this report."]
    pub has_expected_image: bool,
    #[doc = "Whether `actual.png` sits beside this report."]
    pub has_actual_image: bool,
    #[doc = "Whether `diff.png` sits beside this report."]
    pub has_diff_image: bool,
    #[doc = "Compared window titles as `(actual, baseline)` when the title moved."]
    pub title_values: Option<(&'a str, &'a str)>,
}

#[doc = "One completed evidence index: the static page linking every saved check record."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexOutcome {
    #[doc = "The indexed check record count, in deterministic sort order."]
    records: usize,
}

impl IndexOutcome {
    #[doc = "The indexed check record count, in deterministic sort order."]
    #[must_use]
    pub const fn records(&self) -> usize {
        self.records
    }
}

#[doc = "Every failure an evidence index write can report."]
#[derive(Debug)]
#[non_exhaustive]
pub enum IndexError {
    #[doc = "The artifact root cannot be walked for check evidence."]
    Read {
        #[doc = "The artifact root that failed to list."]
        artifacts: PathBuf,
        #[doc = "The filesystem failure."]
        source: std::io::Error,
    },
    #[doc = "The static index page cannot be written."]
    Write {
        #[doc = "The index path that failed."]
        path: PathBuf,
        #[doc = "The filesystem failure."]
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
    #[doc = "The stable Section 16.3 machine code: every index failure is `IO`."]
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "IO"
    }
}

#[doc = "Walk the artifact root for check records, sort them, and write one index page linking each report.\n\n# Errors\n\nReturns [`IndexError`] when the artifact root cannot be walked or the index page cannot be written."]
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

struct IndexedRecord {
    test_id: String,
    reason: String,
    candidate_sha256: String,
    baseline_sha256: String,
    check_dir: PathBuf,
}

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

fn read_record(path: &Path, artifacts: &Path) -> Option<IndexedRecord> {
    let bytes = crate::sample::read_bounded(path).ok()?;
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

fn push_index_line(buffer: &mut String, line: &str) {
    buffer.push_str(line);
    buffer.push('\n');
}

fn relative_href(index_parent: &Path, target: &Path) -> String {
    let base = normalize_lexical(&absolutize(index_parent));
    let target = normalize_lexical(&absolutize(target));
    if let Some(relative) = relative_lexical(&base, &target) {
        return relative;
    }
    file_url(&target)
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

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
            Component::Normal(name) => parts.push(encode_href_segment(name.as_encoded_bytes())),
            Component::ParentDir => parts.push("..".to_string()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    if parts.is_empty() {
        return Some("report.html".to_string());
    }
    Some(parts.join("/"))
}

fn encode_href_segment(raw: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(raw.len());
    for &byte in raw {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@' => out.push(byte as char),
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 15) as usize] as char);
            }
        }
    }
    out
}

fn file_url(target: &Path) -> String {
    use std::path::{Component, Prefix};
    let mut authority = String::new();
    let mut segments: Vec<String> = Vec::new();
    for component in target.components() {
        match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) => segments.push(format!("{}:", drive as char)),
                Prefix::UNC(server, share) => {
                    authority.push_str(&server.to_string_lossy());
                    segments.push(encode_href_segment(share.as_encoded_bytes()));
                }
                _ => segments.push(encode_href_segment(prefix.as_os_str().as_encoded_bytes())),
            },
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => segments.push("..".to_string()),
            Component::Normal(name) => segments.push(encode_href_segment(name.as_encoded_bytes())),
        }
    }
    format!("file://{authority}/{}", segments.join("/"))
}

#[doc = "Render the static HTML report for one comparison as a string."]
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

#[doc = "Render and save the static HTML report through one atomic file write.\n\n# Errors\n\nReturns [`ReportError`] when the report path has no file name or the report file cannot be written."]
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

struct TempFile {
    path: Option<PathBuf>,
}

const MAX_TEMP_CREATE_ATTEMPTS: u32 = 100;

impl TempFile {
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        Self::create_retry(dir, name)
    }

    fn create_retry(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        for _ in 0..MAX_TEMP_CREATE_ATTEMPTS {
            match Self::create_once(dir, name, COUNTER.fetch_add(1, Ordering::SeqCst)) {
                Ok(staged) => return Ok(staged),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("cannot stage a fresh temporary file for {name}"),
        ))
    }

    fn create_once(dir: &Path, name: &str, id: u64) -> Result<(fs::File, Self), std::io::Error> {
        let path = dir.join(format!(".tmp-{}-{id}-{name}", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok((file, Self { path: Some(path) }))
    }

    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary evidence file is already committed",
            ));
        };
        match fs::rename(&path, target) {
            Ok(()) => Ok(()),
            Err(error) => {
                drop(fs::remove_file(&path));
                Err(error)
            }
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            drop(fs::remove_file(path));
        }
    }
}

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

#[doc = "Escape text and attribute values for HTML: no code from a sample runs."]
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

fn push_header(html: &mut String, input: &ReportInput<'_>) {
    let title = escape_html(input.test_id);
    let _result = write!(
        html,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>tuiscotti check: {title}</title>\n</head>\n<body>\n<h1>tuiscotti check: {title}</h1>\n"
    );
}

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

fn push_images(html: &mut String, input: &ReportInput<'_>) {
    html.push_str("<h2>Images</h2>\n");
    push_image(html, "Expected", "expected.png", input.has_expected_image);
    push_image(html, "Actual", "actual.png", input.has_actual_image);
    push_image(html, "Difference", "diff.png", input.has_diff_image);
}

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

fn push_footer(html: &mut String) {
    html.push_str("</body>\n</html>\n");
}
