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

use std::fmt::{Display, Formatter, Write as _};
use std::fs;
use std::path::Path;

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

/// Render and save the static HTML report through one file write.
///
/// # Errors
///
/// Returns [`ReportError`] when the report file cannot be written.
pub fn write_report(input: &ReportInput<'_>, path: &Path) -> Result<(), ReportError> {
    fs::write(path, render_report(input)).map_err(ReportError::Io)?;
    Ok(())
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
    write!(
        html,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>tuiscotti check: {title}</title>\n</head>\n<body>\n<h1>tuiscotti check: {title}</h1>\n"
    )
    .unwrap_or_default();
}

/// Write the outcome summary: reason, gate, and both digests escaped.
fn push_summary(html: &mut String, input: &ReportInput<'_>) {
    write!(
        html,
        "<h2>Outcome</h2>\n<dl>\n<dt>test ID</dt><dd>{}</dd>\n<dt>gate</dt><dd>{}</dd>\n<dt>reason</dt><dd>{}</dd>\n<dt>candidate SHA-256</dt><dd>{}</dd>\n<dt>baseline SHA-256</dt><dd>{}</dd>\n</dl>\n",
        escape_html(input.test_id),
        escape_html(input.gate),
        escape_html(input.reason),
        escape_html(input.candidate_sha256),
        escape_html(input.baseline_sha256),
    )
    .unwrap_or_default();
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
            writeln!(html, "<li>{}</li>", escape_html(field)).unwrap_or_default();
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
        write!(
            html,
            "<h3>Changed values</h3>\n<table>\n<tr><th>field</th><th>candidate</th><th>baseline</th></tr>\n<tr><td>title</td><td>{}</td><td>{}</td></tr>\n</table>\n",
            escape_html(actual),
            escape_html(baseline),
        )
        .unwrap_or_default();
    }
}

/// Write the changed-cell table with coordinates and display text.
fn push_cells(html: &mut String, input: &ReportInput<'_>) {
    write!(
        html,
        "<p>Changed cells: {}</p>\n<table>\n<tr><th>column</th><th>row</th><th>fields</th><th>candidate text</th><th>baseline text</th></tr>\n",
        input.changed_cell_total
    )
    .unwrap_or_default();
    for cell in input.changed_cells {
        let fields: Vec<String> = cell
            .fields()
            .iter()
            .map(|field| escape_html(field))
            .collect();
        writeln!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            cell.col(),
            cell.row(),
            fields.join(", "),
            escape_html(cell.actual_text()),
            escape_html(cell.baseline_text()),
        )
        .unwrap_or_default();
    }
    html.push_str("</table>\n");
}

/// Write the compared-pixel summary when pixels were compared.
fn push_pixels(html: &mut String, input: &ReportInput<'_>) {
    if let Some(pixels) = input.pixels {
        let bounds = pixels.bounds;
        writeln!(
            html,
            "<p>Changed pixels: {} of {} compared; bounds ({}-{}, {}-{}).</p>",
            pixels.changed, pixels.compared, bounds[0], bounds[1], bounds[2], bounds[3],
        )
        .unwrap_or_default();
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
        write!(
            html,
            "<h3>{label}</h3>\n<img src=\"{file}\" alt=\"{label} image\">\n"
        )
        .unwrap_or_default();
    } else if label == "Expected" {
        html.push_str(
            "<h3>Expected</h3>\n<p>The expected image is missing: no baseline image was compared.</p>\n",
        );
    } else {
        write!(
            html,
            "<h3>{label}</h3>\n<p>The {label} image is missing for this comparison.</p>\n"
        )
        .unwrap_or_default();
    }
}

/// Close the document: no JavaScript application follows.
fn push_footer(html: &mut String) {
    html.push_str("</body>\n</html>\n");
}

#[cfg(test)]
mod tests {
    use super::{ReportInput, escape_html, render_report};

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
}
