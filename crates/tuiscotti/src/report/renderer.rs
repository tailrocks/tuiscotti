use std::fmt::Write as _;
use std::path::Path;

use super::atomic::atomic_write;
use super::{ReportError, ReportInput};

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
