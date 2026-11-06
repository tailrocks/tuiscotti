#![doc = include_str!("../docs/check.md")]
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use tui_test::pixel::PixelError;

use crate::report::ReportedCell;
use crate::sample::{Gate, SampleError};

#[doc = "Schema identifier for `result.json` files written beside the evidence."]
pub const CHECK_SCHEMA: &str = "tuiscotti.check.v1";

#[doc = "Literal baseline digest recorded when a check finds no baseline file."]
pub const MISSING_BASELINE: &str = "missing";

#[derive(Debug)]
#[non_exhaustive]
#[doc = "Every failure a frozen check or evidence write can report."]
pub enum CheckError {
    #[doc = "A test ID breaks the Section 14.1 grammar."]
    InvalidTestId {
        #[doc = "What the test ID broke."]
        detail: String,
    },
    #[doc = "Baseline and artifact roots are equal, contained, or a store-relative path leaves its root through a symlink."]
    InvalidStorePath {
        #[doc = "What the path broke."]
        detail: String,
    },
    #[doc = "The candidate or baseline sample fails [`crate::sample::Sample`] validation."]
    Sample(SampleError),
    #[doc = "Upstream pixel comparison rejects decoded image buffers."]
    Pixel(PixelError),
    #[doc = "Saved candidate or evidence bytes fail re-validation."]
    EvidenceCorrupt {
        #[doc = "Which saved file failed and how."]
        detail: String,
    },
    #[doc = "A difference image fails PNG encoding."]
    Image {
        #[doc = "The PNG encoding failure."]
        source: image::ImageError,
    },
    #[doc = "A filesystem failure outside sample validation."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
    },
    #[doc = "A static HTML report fails to render or save."]
    Report(crate::report::ReportError),
}

impl Display for CheckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTestId { detail } => write!(f, "invalid test ID: {detail}"),
            Self::InvalidStorePath { detail } => write!(f, "invalid store path: {detail}"),
            Self::Sample(source) => write!(f, "{source}"),
            Self::Pixel(source) => write!(f, "pixel comparison failed: {source}"),
            Self::EvidenceCorrupt { detail } => write!(f, "saved evidence is corrupt: {detail}"),
            Self::Image { source } => write!(f, "difference image encoding failed: {source}"),
            Self::Io { source } => write!(f, "store I/O failed: {source}"),
            Self::Report(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sample(source) => Some(source),
            Self::Pixel(source) => Some(source),
            Self::Image { source } => Some(source),
            Self::Io { source } => Some(source),
            Self::Report(source) => Some(source),
            _ => None,
        }
    }
}

impl From<SampleError> for CheckError {
    fn from(source: SampleError) -> Self {
        Self::Sample(source)
    }
}

impl From<PixelError> for CheckError {
    fn from(source: PixelError) -> Self {
        Self::Pixel(source)
    }
}

impl From<std::io::Error> for CheckError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}

impl From<crate::report::ReportError> for CheckError {
    fn from(source: crate::report::ReportError) -> Self {
        Self::Report(source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[doc = "Structured mismatch reason with one value per failure class (GOAL 14.2)."]
pub enum MismatchReason {
    #[doc = "No baseline file exists for the test ID."]
    Missing,
    #[doc = "Candidate and baseline carry different test IDs."]
    TestId,
    #[doc = "Candidate and baseline carry different gates."]
    Policy,
    #[doc = "Canonical snapshot state differs."]
    State,
    #[doc = "Render contracts differ with equal state."]
    Contract,
    #[doc = "Decoded image dimensions differ with equal state and contract."]
    Dimensions,
    #[doc = "Decoded RGBA pixels differ with equal dimensions."]
    Pixels,
}

impl MismatchReason {
    #[must_use]
    #[doc = "The wire value recorded in `result.json` and the HTML report."]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::TestId => "test-id",
            Self::Policy => "policy",
            Self::State => "state",
            Self::Contract => "contract",
            Self::Dimensions => "dimensions",
            Self::Pixels => "pixels",
        }
    }
}

impl Display for MismatchReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "A passing comparison: the candidate digest and the compared baseline digest."]
pub struct CheckSuccess {
    test_id: String,
    candidate_sha256: String,
    baseline_sha256: String,
}

impl CheckSuccess {
    #[must_use]
    #[doc = "The compared test ID."]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[must_use]
    #[doc = "SHA-256 over the candidate sample file bytes."]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    #[must_use]
    #[doc = "SHA-256 over the compared baseline file bytes."]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }
}

#[derive(Debug, Clone)]
#[doc = "A failing comparison with the saved candidate and evidence locations."]
pub struct CheckFailure {
    test_id: String,
    gate: Gate,
    reason: MismatchReason,
    candidate_sha256: String,
    baseline_sha256: String,
    candidate_path: PathBuf,
    evidence_dir: PathBuf,
    report_path: PathBuf,
    changed_fields: Vec<String>,
    changed_cells: Vec<ReportedCell>,
    changed_cell_total: usize,
    pixel_changed: u64,
    pixel_compared: u64,
    pixel_bounds: Option<[u32; 4]>,
}

impl CheckFailure {
    #[must_use]
    #[doc = "The compared test ID."]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    #[must_use]
    #[doc = "The gate both samples carry."]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    #[must_use]
    #[doc = "The structured mismatch reason."]
    pub const fn reason(&self) -> MismatchReason {
        self.reason
    }

    #[must_use]
    #[doc = "SHA-256 over the candidate sample file bytes."]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    #[must_use]
    #[doc = "SHA-256 over the compared baseline bytes, or `missing`."]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }

    #[must_use]
    #[doc = "Where the candidate bytes were saved (never a shared mutable file)."]
    pub fn candidate_path(&self) -> &Path {
        &self.candidate_path
    }

    #[must_use]
    #[doc = "The `checks/<baseline_sha256_or_missing>` directory for this run."]
    pub fn evidence_dir(&self) -> &Path {
        &self.evidence_dir
    }

    #[must_use]
    #[doc = "The static HTML report for this comparison."]
    pub fn report_path(&self) -> &Path {
        &self.report_path
    }

    #[must_use]
    #[doc = "Changed top-level state fields, in fixed order."]
    pub fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }

    #[must_use]
    #[doc = "Changed cells with coordinates, row-major, bounded (see the total)."]
    pub fn changed_cells(&self) -> &[ReportedCell] {
        &self.changed_cells
    }

    #[must_use]
    #[doc = "The exact changed-cell count, without the report bound."]
    pub const fn changed_cell_total(&self) -> usize {
        self.changed_cell_total
    }

    #[must_use]
    #[doc = "Changed pixel count from upstream `compare_rgba` (zero when pixels were not compared)."]
    pub const fn pixel_changed(&self) -> u64 {
        self.pixel_changed
    }

    #[must_use]
    #[doc = "Compared pixel count (zero after a dimension mismatch)."]
    pub const fn pixel_compared(&self) -> u64 {
        self.pixel_compared
    }

    #[must_use]
    #[doc = "Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`."]
    pub const fn pixel_bounds(&self) -> Option<[u32; 4]> {
        self.pixel_bounds
    }
}

#[derive(Debug, Clone)]
#[doc = "The outcome of [`FrozenStore::check`]: a pass or a saved failure."]
pub enum CheckOutcome {
    #[doc = "The candidate equals the baseline under its gate."]
    Pass(CheckSuccess),
    #[doc = "The candidate differs; evidence is saved under the artifact root."]
    Failure(CheckFailure),
}

impl CheckOutcome {
    #[must_use]
    #[doc = "True only for an equal comparison."]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass(_))
    }
}

#[derive(Debug, Clone)]
#[doc = "A read-only comparison handle over a baseline root and an artifact root."]
pub struct FrozenStore {
    baselines: PathBuf,
    artifacts: PathBuf,
}

pub(crate) enum PresentComparison {
    Pass,
    Failure {
        reason: MismatchReason,
        changed_fields: Vec<String>,
    },
}

mod approval_lock;
mod diff;
mod evidence;
mod paths;
mod pixels;
mod record;
mod store;

pub(crate) use approval_lock::{
    APPROVAL_LOCK_FILE, APPROVAL_LOCK_TIMEOUT_SECS, ApprovalFileLock, ApprovalLockError,
};

const MAX_REPORTED_CELLS: usize = 64;

#[doc = "Validate a test ID against the Section 14.1 grammar on every platform.\n\n# Errors\n\nReturns [`CheckError::InvalidTestId`] for every rejected shape."]
pub fn validate_test_id(test_id: &str) -> Result<(), CheckError> {
    crate::sample::validate_test_id_grammar(test_id).map_err(|error| match error {
        SampleError::InvalidTestId { detail } => CheckError::InvalidTestId { detail },
        other => CheckError::Sample(other),
    })
}
