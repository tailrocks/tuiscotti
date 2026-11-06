#![doc = include_str!("../docs/report.md")]
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

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
    #[doc = "The stable CLI machine code: every index failure is `IO`."]
    #[must_use]
    pub const fn code(&self) -> &'static str {
        "IO"
    }
}

mod atomic;
mod index;
mod renderer;

pub(crate) use atomic::atomic_write;
pub use index::write_report_index;
pub use renderer::{escape_html, render_report, write_report};
