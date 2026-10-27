#[doc = include_str!("../docs/check.md")]
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use fs2::FileExt as _;
use image::ImageDecoder as _;
use sha2::{Digest, Sha256};
use tui_test::pixel::{PixelError, compare_rgba};
use tui_test::terminal::cell::Attrs;
use tui_test::{SnapshotCell, TerminalSnapshot};

use crate::report::{
    PixelSummary, ReportInput, ReportedCell, atomic_write, render_report, write_report,
};
use crate::sample::{
    Gate, MAX_IMAGE_DIM, MAX_RGBA_BYTES, Sample, SampleError, read_bounded, read_bounded_optional,
};

#[doc = "Schema identifier for `result.json` files written beside the evidence."]
pub const CHECK_SCHEMA: &str = "tuiscotti.check.v1";

#[doc = "Literal baseline digest recorded when a check finds no baseline file."]
pub const MISSING_BASELINE: &str = "missing";

const MAX_REPORTED_CELLS: usize = 64;

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
    #[doc = "The candidate or baseline sample fails [`Sample`] validation."]
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

impl PresentComparison {
    fn from_state(reason: MismatchReason, state: &StateDiff) -> Self {
        Self::Failure {
            reason,
            changed_fields: state.changed_fields.clone(),
        }
    }
}

impl FrozenStore {
    #[doc = "Open a frozen store over two roots.\n\n# Errors\n\nReturns [`CheckError`] for empty, equal, contained, or unresolvable roots, and for artifact-root creation failures."]
    pub fn new(baseline_root: &Path, artifact_root: &Path) -> Result<Self, CheckError> {
        let baselines = resolve_root(baseline_root, "baseline")?;
        let artifacts = resolve_root(artifact_root, "artifact")?;
        if baselines == artifacts {
            return Err(CheckError::InvalidStorePath {
                detail: "baseline and artifact roots are the same directory".to_string(),
            });
        }
        if baselines.starts_with(&artifacts) || artifacts.starts_with(&baselines) {
            return Err(CheckError::InvalidStorePath {
                detail: "baseline and artifact roots must not contain each other".to_string(),
            });
        }
        fs::create_dir_all(&artifacts).map_err(|source| CheckError::Io { source })?;
        Ok(Self {
            baselines,
            artifacts,
        })
    }

    #[must_use]
    #[doc = "The canonical baseline root."]
    pub fn baseline_root(&self) -> &Path {
        &self.baselines
    }

    #[must_use]
    #[doc = "The canonical artifact root."]
    pub fn artifact_root(&self) -> &Path {
        &self.artifacts
    }

    #[doc = "Compare candidate sample bytes against the stored baseline.\n\n# Errors\n\nReturns [`CheckError`] for invalid samples, test IDs, store paths, corrupt saved evidence, or I/O failures."]
    pub fn check(&self, sample_bytes: &[u8]) -> Result<CheckOutcome, CheckError> {
        let candidate_sha256 = sha256_hex(sample_bytes);
        let actual = Sample::from_bytes(sample_bytes)?;
        validate_test_id(actual.test_id())?;
        let baseline_path = self.baseline_path(actual.test_id())?;
        let baseline_bytes =
            read_bounded_optional(&baseline_path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
        match baseline_bytes {
            None => self.fail_without_baseline(&actual, &candidate_sha256, sample_bytes),
            Some(bytes) => {
                self.compare_with_baseline(&actual, &candidate_sha256, sample_bytes, &bytes)
            }
        }
    }

    #[doc = "Read a sample file with the shared bounded read, then [`FrozenStore::check`] it.\n\n# Errors\n\nReturns [`CheckError`] when the file is unreadable or the check fails."]
    pub fn check_file(&self, path: &Path) -> Result<CheckOutcome, CheckError> {
        let bytes = read_bounded(path).map_err(|error| match error {
            SampleError::Io { source } => CheckError::Io { source },
            other => CheckError::from(other),
        })?;
        self.check(&bytes)
    }

    fn compare_with_baseline(
        &self,
        actual: &Sample,
        candidate_digest: &str,
        candidate_bytes: &[u8],
        baseline_bytes: &[u8],
    ) -> Result<CheckOutcome, CheckError> {
        let baseline_digest = sha256_hex(baseline_bytes);
        let baseline = Sample::from_bytes(baseline_bytes)?;
        validate_test_id(baseline.test_id())?;
        if actual.test_id() != baseline.test_id() {
            let failure = self.write_failure(
                actual.test_id(),
                actual.gate(),
                MismatchReason::TestId,
                candidate_digest,
                candidate_bytes,
                &baseline_digest,
                Some(baseline_bytes),
                &StateDiff::named("test_id"),
                None,
            )?;
            return Ok(CheckOutcome::Failure(failure));
        }
        if actual.gate() != baseline.gate() {
            let failure = self.write_failure(
                actual.test_id(),
                actual.gate(),
                MismatchReason::Policy,
                candidate_digest,
                candidate_bytes,
                &baseline_digest,
                Some(baseline_bytes),
                &StateDiff::named("gate"),
                None,
            )?;
            return Ok(CheckOutcome::Failure(failure));
        }
        let state = diff_snapshots(actual.snapshot(), baseline.snapshot());
        if !state.is_equal() {
            let failure = self.write_failure(
                actual.test_id(),
                actual.gate(),
                MismatchReason::State,
                candidate_digest,
                candidate_bytes,
                &baseline_digest,
                Some(baseline_bytes),
                &state,
                None,
            )?;
            return Ok(CheckOutcome::Failure(failure));
        }
        if actual.gate() == Gate::State {
            return Ok(CheckOutcome::Pass(CheckSuccess {
                test_id: actual.test_id().to_string(),
                candidate_sha256: candidate_digest.to_string(),
                baseline_sha256: baseline_digest,
            }));
        }
        self.compare_pixels(
            actual,
            &baseline,
            candidate_digest,
            candidate_bytes,
            &baseline_digest,
            baseline_bytes,
        )
    }

    fn compare_pixels(
        &self,
        actual: &Sample,
        baseline: &Sample,
        candidate_digest: &str,
        candidate_bytes: &[u8],
        baseline_digest: &str,
        baseline_bytes: &[u8],
    ) -> Result<CheckOutcome, CheckError> {
        if actual.render_contract() != baseline.render_contract() {
            let failure = self.write_failure(
                actual.test_id(),
                actual.gate(),
                MismatchReason::Contract,
                candidate_digest,
                candidate_bytes,
                baseline_digest,
                Some(baseline_bytes),
                &StateDiff::named("render_contract"),
                None,
            )?;
            return Ok(CheckOutcome::Failure(failure));
        }
        match decide_pixels(actual, baseline)? {
            PixelVerdict::Equal => Ok(CheckOutcome::Pass(CheckSuccess {
                test_id: actual.test_id().to_string(),
                candidate_sha256: candidate_digest.to_string(),
                baseline_sha256: baseline_digest.to_string(),
            })),
            PixelVerdict::Dimensions => {
                let failure = self.write_failure(
                    actual.test_id(),
                    actual.gate(),
                    MismatchReason::Dimensions,
                    candidate_digest,
                    candidate_bytes,
                    baseline_digest,
                    Some(baseline_bytes),
                    &StateDiff::named("image_dimensions"),
                    None,
                )?;
                Ok(CheckOutcome::Failure(failure))
            }
            PixelVerdict::Different {
                changed,
                compared,
                bounds,
                diff,
            } => {
                let failure = self.write_failure(
                    actual.test_id(),
                    actual.gate(),
                    MismatchReason::Pixels,
                    candidate_digest,
                    candidate_bytes,
                    baseline_digest,
                    Some(baseline_bytes),
                    &StateDiff::pixels(changed, compared, bounds),
                    Some(&diff),
                )?;
                Ok(CheckOutcome::Failure(failure))
            }
        }
    }

    pub(crate) fn compare_present(
        candidate: &Sample,
        baseline: &Sample,
    ) -> Result<PresentComparison, CheckError> {
        let state = diff_snapshots(candidate.snapshot(), baseline.snapshot());
        if !state.is_equal() {
            return Ok(PresentComparison::from_state(MismatchReason::State, &state));
        }
        if candidate.gate() == Gate::State {
            return Ok(PresentComparison::Pass);
        }
        if baseline.image().is_none() {
            let gate = StateDiff::named("gate");
            return Ok(PresentComparison::from_state(MismatchReason::Policy, &gate));
        }
        if candidate.render_contract() != baseline.render_contract() {
            let contract = StateDiff::named("render_contract");
            return Ok(PresentComparison::from_state(
                MismatchReason::Contract,
                &contract,
            ));
        }
        match decide_pixels(candidate, baseline)? {
            PixelVerdict::Equal => Ok(PresentComparison::Pass),
            PixelVerdict::Dimensions => {
                let dimensions = StateDiff::named("image_dimensions");
                Ok(PresentComparison::from_state(
                    MismatchReason::Dimensions,
                    &dimensions,
                ))
            }
            PixelVerdict::Different {
                changed,
                compared,
                bounds,
                ..
            } => {
                let pixels = StateDiff::pixels(changed, compared, bounds);
                Ok(PresentComparison::from_state(
                    MismatchReason::Pixels,
                    &pixels,
                ))
            }
        }
    }

    fn fail_without_baseline(
        &self,
        actual: &Sample,
        candidate_digest: &str,
        candidate_bytes: &[u8],
    ) -> Result<CheckOutcome, CheckError> {
        let failure = self.write_failure(
            actual.test_id(),
            actual.gate(),
            MismatchReason::Missing,
            candidate_digest,
            candidate_bytes,
            MISSING_BASELINE,
            None,
            &StateDiff::named("baseline"),
            None,
        )?;
        Ok(CheckOutcome::Failure(failure))
    }

    fn baseline_path(&self, test_id: &str) -> Result<PathBuf, CheckError> {
        let mut relative = PathBuf::from(test_id);
        relative.set_extension("tui.json");
        ensure_no_symlink(&self.baselines, &relative)?;
        let joined = self.baselines.join(&relative);
        if joined.exists() {
            let canonical = joined
                .canonicalize()
                .map_err(|source| CheckError::Io { source })?;
            if !canonical.starts_with(&self.baselines) {
                return Err(CheckError::InvalidStorePath {
                    detail: format!(
                        "baseline path for {test_id:?} leaves the baseline root through a link"
                    ),
                });
            }
            return Ok(canonical);
        }
        self.missing_baseline_path(&joined)
    }

    fn missing_baseline_path(&self, joined: &Path) -> Result<PathBuf, CheckError> {
        let mut cursor = joined;
        loop {
            if cursor == self.baselines {
                return Ok(joined.to_path_buf());
            }
            let Some(parent) = cursor.parent() else {
                return Err(CheckError::InvalidStorePath {
                    detail: "baseline path has no resolvable ancestor".to_string(),
                });
            };
            if parent.exists() {
                let canonical = parent
                    .canonicalize()
                    .map_err(|source| CheckError::Io { source })?;
                if !canonical.starts_with(&self.baselines) {
                    return Err(CheckError::InvalidStorePath {
                        detail: "baseline path leaves the baseline root through a link".to_string(),
                    });
                }
                return Ok(joined.to_path_buf());
            }
            cursor = parent;
        }
    }
}

const MAX_DISPLAY_CHARS: usize = 200;

const CELL_FLAGS: [(&str, Attrs); 7] = [
    ("bold", Attrs::BOLD),
    ("dim", Attrs::DIM),
    ("italic", Attrs::ITALIC),
    ("inverse", Attrs::INVERSE),
    ("invisible", Attrs::INVISIBLE),
    ("strike", Attrs::STRIKE),
    ("blink", Attrs::BLINK),
];

struct StateDiff {
    changed_fields: Vec<String>,
    changed_cells: Vec<ReportedCell>,
    changed_cell_total: usize,
    title_values: Option<(String, String)>,
    pixel_changed: u64,
    pixel_compared: u64,
    pixel_bounds: Option<[u32; 4]>,
}

impl StateDiff {
    fn named(field: &str) -> Self {
        Self {
            changed_fields: vec![field.to_string()],
            changed_cells: Vec::new(),
            changed_cell_total: 0,
            title_values: None,
            pixel_changed: 0,
            pixel_compared: 0,
            pixel_bounds: None,
        }
    }

    fn pixels(changed: u64, compared: u64, bounds: Option<[u32; 4]>) -> Self {
        Self {
            changed_fields: Vec::new(),
            changed_cells: Vec::new(),
            changed_cell_total: 0,
            title_values: None,
            pixel_changed: changed,
            pixel_compared: compared,
            pixel_bounds: bounds,
        }
    }

    fn is_equal(&self) -> bool {
        self.changed_fields.is_empty() && self.changed_cell_total == 0
    }
}

struct DiffPng {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

enum PixelVerdict {
    Equal,
    Dimensions,
    Different {
        changed: u64,
        compared: u64,
        bounds: Option<[u32; 4]>,
        diff: DiffPng,
    },
}

fn decide_pixels(actual: &Sample, baseline: &Sample) -> Result<PixelVerdict, CheckError> {
    let actual_image = actual.image().ok_or_else(|| SampleError::GateMismatch {
        detail: "state-and-pixels gate requires an image".to_string(),
    })?;
    let baseline_image = baseline.image().ok_or_else(|| SampleError::GateMismatch {
        detail: "state-and-pixels baseline requires an image".to_string(),
    })?;
    let (_, actual_width, actual_height, actual_pixels) = decode_sample_png(
        actual_image.png_base64(),
        actual_image.width(),
        actual_image.height(),
    )?;
    let (_, baseline_width, baseline_height, baseline_pixels) = decode_sample_png(
        baseline_image.png_base64(),
        baseline_image.width(),
        baseline_image.height(),
    )?;
    if (actual_width, actual_height) != (baseline_width, baseline_height) {
        return Ok(PixelVerdict::Dimensions);
    }
    let comparison = compare_rgba(
        actual_width,
        actual_height,
        &actual_pixels,
        baseline_width,
        baseline_height,
        &baseline_pixels,
    )?;
    if comparison.is_equal() {
        return Ok(PixelVerdict::Equal);
    }
    let bounds = comparison
        .bounds
        .map(|found| [found.min_x, found.min_y, found.max_x, found.max_y]);
    let Some(difference_image) = comparison.difference_image else {
        return Err(SampleError::MissingDifferenceImage {
            detail: "upstream pixel comparison reports changed pixels without a difference image"
                .to_string(),
        }
        .into());
    };
    Ok(PixelVerdict::Different {
        changed: comparison.changed_pixels,
        compared: comparison.compared_pixels,
        bounds,
        diff: DiffPng {
            width: actual_width,
            height: actual_height,
            pixels: difference_image,
        },
    })
}

#[doc = "Validate a test ID against the Section 14.1 grammar on every platform.\n\n# Errors\n\nReturns [`CheckError::InvalidTestId`] for every rejected shape."]
pub fn validate_test_id(test_id: &str) -> Result<(), CheckError> {
    crate::sample::validate_test_id_grammar(test_id).map_err(|error| match error {
        SampleError::InvalidTestId { detail } => CheckError::InvalidTestId { detail },
        other => CheckError::Sample(other),
    })
}

fn diff_snapshots(actual: &TerminalSnapshot, baseline: &TerminalSnapshot) -> StateDiff {
    let mut changed_fields = Vec::new();
    if actual.cols() != baseline.cols() {
        changed_fields.push("cols".to_string());
    }
    if actual.rows() != baseline.rows() {
        changed_fields.push("rows".to_string());
    }
    if actual.scope() != baseline.scope() {
        changed_fields.push("scope".to_string());
    }
    if actual.viewport_origin() != baseline.viewport_origin() {
        changed_fields.push("viewport_origin".to_string());
    }
    if actual.palette() != baseline.palette() {
        changed_fields.push("palette".to_string());
    }
    if actual.default_foreground() != baseline.default_foreground() {
        changed_fields.push("default_foreground".to_string());
    }
    if actual.default_background() != baseline.default_background() {
        changed_fields.push("default_background".to_string());
    }
    if actual.default_cursor() != baseline.default_cursor() {
        changed_fields.push("default_cursor".to_string());
    }
    changed_fields.extend(diff_cursors(actual, baseline));
    let title_values = if actual.title() == baseline.title() {
        None
    } else {
        changed_fields.push("title".to_string());
        Some((
            display_value(actual.title().unwrap_or("")),
            display_value(baseline.title().unwrap_or("")),
        ))
    };
    let (changed_cells, changed_cell_total) = diff_cells(actual, baseline);
    StateDiff {
        changed_fields,
        changed_cells,
        changed_cell_total,
        title_values,
        pixel_changed: 0,
        pixel_compared: 0,
        pixel_bounds: None,
    }
}

fn display_value(value: &str) -> String {
    if value.chars().count() <= MAX_DISPLAY_CHARS {
        return value.to_string();
    }
    let kept: String = value.chars().take(MAX_DISPLAY_CHARS).collect();
    format!("{kept}…")
}

fn diff_cursors(actual: &TerminalSnapshot, baseline: &TerminalSnapshot) -> Vec<String> {
    let mut fields = Vec::new();
    let (left, right) = (actual.cursor(), baseline.cursor());
    if left.col() != right.col() {
        fields.push("cursor.col".to_string());
    }
    if left.row() != right.row() {
        fields.push("cursor.row".to_string());
    }
    if left.in_capture() != right.in_capture() {
        fields.push("cursor.in_capture".to_string());
    }
    if left.visible() != right.visible() {
        fields.push("cursor.visible".to_string());
    }
    if left.shape() != right.shape() {
        fields.push("cursor.shape".to_string());
    }
    if left.color() != right.color() {
        fields.push("cursor.color".to_string());
    }
    if left.blink() != right.blink() {
        fields.push("cursor.blink".to_string());
    }
    fields
}

fn diff_cells(
    actual: &TerminalSnapshot,
    baseline: &TerminalSnapshot,
) -> (Vec<ReportedCell>, usize) {
    let rows = actual.rows().max(baseline.rows());
    let mut changed = Vec::new();
    let mut total = 0usize;
    for row in 0..rows {
        let row_cols = if row < actual.rows() && row < baseline.rows() {
            actual.cols().max(baseline.cols())
        } else if row < actual.rows() {
            actual.cols()
        } else {
            baseline.cols()
        };
        for col in 0..row_cols {
            let pair = (actual.cell_at(col, row), baseline.cell_at(col, row));
            let Some((fields, actual_text, baseline_text)) = changed_pair(pair) else {
                continue;
            };
            total += 1;
            if changed.len() < MAX_REPORTED_CELLS {
                changed.push(ReportedCell::new(
                    col,
                    row,
                    fields,
                    actual_text,
                    baseline_text,
                ));
            }
        }
    }
    (changed, total)
}

fn changed_pair(
    pair: (Option<&SnapshotCell>, Option<&SnapshotCell>),
) -> Option<(Vec<String>, String, String)> {
    let (fields, actual_text, baseline_text) = match pair {
        (Some(left), Some(right)) => {
            let fields = diff_cell(left, right);
            if fields.is_empty() {
                return None;
            }
            (
                fields,
                display_value(left.text()),
                display_value(right.text()),
            )
        }
        (Some(left), None) => (
            vec!["cell".to_string()],
            display_value(left.text()),
            String::new(),
        ),
        (None, Some(right)) => (
            vec!["cell".to_string()],
            String::new(),
            display_value(right.text()),
        ),
        (None, None) => return None,
    };
    Some((fields, actual_text, baseline_text))
}

fn diff_cell(actual: &SnapshotCell, baseline: &SnapshotCell) -> Vec<String> {
    let mut fields = Vec::new();
    if actual.text() != baseline.text() {
        fields.push("text".to_string());
    }
    if actual.width() != baseline.width() {
        fields.push("width".to_string());
    }
    if actual.fg() != baseline.fg() {
        fields.push("fg".to_string());
    }
    if actual.bg() != baseline.bg() {
        fields.push("bg".to_string());
    }
    for (name, flag) in CELL_FLAGS {
        if actual.has(flag) != baseline.has(flag) {
            fields.push(name.to_string());
        }
    }
    if actual.underline() != baseline.underline() {
        fields.push("underline".to_string());
    }
    if actual.underline_color() != baseline.underline_color() {
        fields.push("underline_color".to_string());
    }
    if actual.link_uri() != baseline.link_uri() {
        fields.push("link_uri".to_string());
    }
    if actual.link_id() != baseline.link_id() {
        fields.push("link_id".to_string());
    }
    fields
}

fn resolve_root(path: &Path, role: &str) -> Result<PathBuf, CheckError> {
    if path.as_os_str().is_empty() {
        return Err(CheckError::InvalidStorePath {
            detail: format!("{role} root path is empty"),
        });
    }
    let anchored: PathBuf;
    let path: &Path = if path.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => {
                anchored = cwd.join(path);
                &anchored
            }
            Err(_) => path,
        }
    } else {
        path
    };
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let mut remainder: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path;
    loop {
        let parent = cursor.parent();
        let name = cursor.file_name();
        let (Some(parent), Some(name)) = (parent, name) else {
            return Err(CheckError::InvalidStorePath {
                detail: format!("{role} root {} has no resolvable ancestor", path.display()),
            });
        };
        if name == ".." {
            return Err(CheckError::InvalidStorePath {
                detail: format!(
                    "{role} root {} carries an unresolvable `..`",
                    path.display()
                ),
            });
        }
        remainder.push(name.to_os_string());
        if let Ok(canonical) = parent.canonicalize() {
            let mut resolved = canonical;
            for component in remainder.iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        cursor = parent;
    }
}

fn ensure_no_symlink(root: &Path, relative: &Path) -> Result<(), CheckError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(CheckError::Io { source }),
        };
        if meta.file_type().is_symlink() {
            return Err(CheckError::InvalidStorePath {
                detail: format!(
                    "store-relative path {} crosses a symlink",
                    relative.display()
                ),
            });
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

fn decode_sample_png(
    base64_png: &str,
    claimed_width: u32,
    claimed_height: u32,
) -> Result<(Vec<u8>, u32, u32, Vec<u8>), CheckError> {
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(base64_png.as_bytes())
        .map_err(|source| SampleError::Base64Invalid { source })
        .map_err(CheckError::from)?;
    check_png_dims(claimed_width, claimed_height)?;
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(&raw))
        .map_err(|source| SampleError::PngDecode { source })
        .map_err(CheckError::from)?;
    if decoder.color_type() != image::ColorType::Rgba8 {
        return Err(SampleError::PngInvalid {
            detail: "check PNG is not 8-bit RGBA".to_string(),
        }
        .into());
    }
    let (width, height) = decoder.dimensions();
    if (width, height) != (claimed_width, claimed_height) {
        return Err(SampleError::DimensionMismatch {
            detail: format!(
                "check image claims {claimed_width}x{claimed_height} but decodes to {width}x{height}"
            ),
        }
        .into());
    }
    check_png_dims(width, height)?;
    let total = byte_total(width, height)?;
    let mut pixels = vec![0u8; total];
    decoder
        .read_image(&mut pixels)
        .map_err(|source| SampleError::PngDecode { source })
        .map_err(CheckError::from)?;
    Ok((raw, width, height, pixels))
}

fn check_png_dims(width: u32, height: u32) -> Result<(), CheckError> {
    if width == 0 || height == 0 {
        return Err(SampleError::DimensionMismatch {
            detail: "check image dimensions are zero".to_string(),
        }
        .into());
    }
    if width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return Err(SampleError::DimensionTooLarge {
            width,
            height,
            limit: MAX_IMAGE_DIM,
        }
        .into());
    }
    let bytes = byte_total(width, height)?;
    let bytes_u64 = u64::try_from(bytes).map_err(|_| SampleError::RgbaTooLarge {
        bytes: u64::MAX,
        limit: MAX_RGBA_BYTES,
    })?;
    if bytes_u64 > MAX_RGBA_BYTES {
        return Err(SampleError::RgbaTooLarge {
            bytes: bytes_u64,
            limit: MAX_RGBA_BYTES,
        }
        .into());
    }
    Ok(())
}

fn byte_total(width: u32, height: u32) -> Result<usize, CheckError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(SampleError::RgbaTooLarge {
            bytes: u64::MAX,
            limit: MAX_RGBA_BYTES,
        })
        .map_err(CheckError::from)
}

fn encode_diff_png(diff: &DiffPng) -> Result<Vec<u8>, CheckError> {
    use image::ImageEncoder as _;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            &diff.pixels,
            diff.width,
            diff.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|source| CheckError::Image { source })?;
    Ok(out)
}

impl FrozenStore {
    #[expect(
        clippy::too_many_arguments,
        reason = "failure evidence needs every compared digest and byte string"
    )]
    fn write_failure(
        &self,
        test_id: &str,
        gate: Gate,
        reason: MismatchReason,
        candidate_digest: &str,
        candidate_bytes: &[u8],
        baseline_digest: &str,
        baseline_bytes: Option<&[u8]>,
        state: &StateDiff,
        diff: Option<&DiffPng>,
    ) -> Result<CheckFailure, CheckError> {
        let candidate_dir = self.candidate_dir(test_id, candidate_digest)?;
        let check_dir = candidate_dir.join("checks").join(baseline_digest);
        ensure_no_symlink(
            &self.artifacts,
            &Self::relative_dir(test_id, candidate_digest, baseline_digest),
        )?;
        fs::create_dir_all(&check_dir).map_err(|source| CheckError::Io { source })?;
        let candidate_path =
            Self::save_candidate(&candidate_dir, candidate_digest, candidate_bytes)?;
        Self::save_expected(&check_dir, baseline_digest, baseline_bytes)?;
        let images = Self::save_images(
            &check_dir,
            gate,
            baseline_digest,
            baseline_bytes,
            candidate_bytes,
            diff,
        )?;
        let record = ResultRecord::for_failure(
            test_id,
            gate,
            reason,
            candidate_digest,
            baseline_digest,
            state,
        );
        record.save(&check_dir)?;
        let report_path = check_dir.join("report.html");
        let input = record.report_input(state, &images);
        if report_path.exists() {
            validate_saved_report(&report_path, &render_report(&input))?;
        } else {
            write_report(&input, &report_path).map_err(CheckError::from)?;
        }
        Ok(CheckFailure {
            test_id: test_id.to_string(),
            gate,
            reason,
            candidate_sha256: candidate_digest.to_string(),
            baseline_sha256: baseline_digest.to_string(),
            candidate_path,
            evidence_dir: check_dir,
            report_path,
            changed_fields: state.changed_fields.clone(),
            changed_cells: state.changed_cells.clone(),
            changed_cell_total: state.changed_cell_total,
            pixel_changed: state.pixel_changed,
            pixel_compared: state.pixel_compared,
            pixel_bounds: state.pixel_bounds,
        })
    }

    fn candidate_dir(&self, test_id: &str, candidate_digest: &str) -> Result<PathBuf, CheckError> {
        let relative: PathBuf = [test_id, candidate_digest].iter().collect();
        ensure_no_symlink(&self.artifacts, &relative)?;
        Ok(self.artifacts.join(&relative))
    }

    fn relative_dir(test_id: &str, candidate_digest: &str, baseline_digest: &str) -> PathBuf {
        [test_id, candidate_digest, "checks", baseline_digest]
            .iter()
            .collect()
    }

    fn save_candidate(
        candidate_dir: &Path,
        candidate_digest: &str,
        candidate_bytes: &[u8],
    ) -> Result<PathBuf, CheckError> {
        fs::create_dir_all(candidate_dir).map_err(|source| CheckError::Io { source })?;
        let path = candidate_dir.join("sample.tui.json");
        if path.exists() {
            let saved = read_bounded(&path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
            validate_saved_sample(&path, &saved, candidate_digest)?;
            return Ok(path);
        }
        atomic_write(candidate_dir, "sample.tui.json", candidate_bytes)
            .map_err(|source| CheckError::Io { source })?;
        Ok(path)
    }

    fn save_expected(
        check_dir: &Path,
        baseline_digest: &str,
        baseline_bytes: Option<&[u8]>,
    ) -> Result<(), CheckError> {
        let Some(bytes) = baseline_bytes else {
            return Ok(());
        };
        let path = check_dir.join("expected.tui.json");
        if path.exists() {
            let saved = read_bounded(&path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
            validate_saved_sample(&path, &saved, baseline_digest)?;
            return Ok(());
        }
        atomic_write(check_dir, "expected.tui.json", bytes)
            .map_err(|source| CheckError::Io { source })?;
        Ok(())
    }

    fn save_images(
        check_dir: &Path,
        gate: Gate,
        baseline_digest: &str,
        baseline_bytes: Option<&[u8]>,
        candidate_bytes: &[u8],
        diff: Option<&DiffPng>,
    ) -> Result<SavedImages, CheckError> {
        if gate == Gate::State {
            return Ok(SavedImages::none());
        }
        let actual_png = sample_png_bytes(candidate_bytes)?;
        save_png(check_dir, "actual.png", &actual_png)?;
        let mut images = SavedImages {
            has_expected: false,
            has_actual: true,
            has_diff: false,
        };
        if let Some(bytes) = baseline_bytes {
            let baseline = Sample::from_bytes(bytes).map_err(CheckError::from)?;
            if baseline.image().is_some() {
                let expected_png = sample_png_bytes(bytes)?;
                save_png(check_dir, "expected.png", &expected_png)?;
                images.has_expected = baseline_digest != MISSING_BASELINE;
            }
        }
        if let Some(diff) = diff {
            let encoded = encode_diff_png(diff)?;
            save_png_bytes(check_dir, "diff.png", &encoded)?;
            images.has_diff = true;
        }
        Ok(images)
    }
}

struct SavedImages {
    has_expected: bool,
    has_actual: bool,
    has_diff: bool,
}

impl SavedImages {
    const fn none() -> Self {
        Self {
            has_expected: false,
            has_actual: false,
            has_diff: false,
        }
    }
}

fn validate_saved_report(path: &Path, expected: &str) -> Result<(), CheckError> {
    let file_type = fs::symlink_metadata(path)
        .map_err(|source| CheckError::Io { source })?
        .file_type();
    if !file_type.is_file() {
        return Err(CheckError::EvidenceCorrupt {
            detail: format!("{} is not a regular file", path.display()),
        });
    }
    let saved = read_bounded(path).map_err(|error| match error {
        SampleError::Io { source } => CheckError::Io { source },
        other => CheckError::from(other),
    })?;
    if saved != expected.as_bytes() {
        return Err(CheckError::EvidenceCorrupt {
            detail: format!(
                "{} disagrees with this comparison; conflicting evidence is never overwritten",
                path.display()
            ),
        });
    }
    Ok(())
}

fn validate_saved_sample(path: &Path, saved: &[u8], digest: &str) -> Result<(), CheckError> {
    Sample::from_bytes(saved).map_err(|source| CheckError::EvidenceCorrupt {
        detail: format!("{} fails validation: {source}", path.display()),
    })?;
    if sha256_hex(saved) != digest {
        return Err(CheckError::EvidenceCorrupt {
            detail: format!("{} digest does not match its evidence key", path.display()),
        });
    }
    Ok(())
}

fn sample_png_bytes(sample_bytes: &[u8]) -> Result<Vec<u8>, CheckError> {
    use base64::Engine as _;
    let sample = Sample::from_bytes(sample_bytes)?;
    let image = sample.image().ok_or_else(|| SampleError::GateMismatch {
        detail: "pixel evidence requires an image".to_string(),
    })?;
    base64::engine::general_purpose::STANDARD
        .decode(image.png_base64().as_bytes())
        .map_err(|source| SampleError::Base64Invalid { source })
        .map_err(CheckError::from)
}

fn save_png(check_dir: &Path, name: &str, png: &[u8]) -> Result<(), CheckError> {
    let path = check_dir.join(name);
    if path.exists() {
        let saved = read_bounded(&path).map_err(|error| match error {
            SampleError::Io { source } => CheckError::Io { source },
            other => CheckError::from(other),
        })?;
        if sha256_hex(&saved) != sha256_hex(png) {
            return Err(CheckError::EvidenceCorrupt {
                detail: format!("{} disagrees with the compared image", path.display()),
            });
        }
        return Ok(());
    }
    atomic_write(check_dir, name, png).map_err(|source| CheckError::Io { source })?;
    Ok(())
}

fn save_png_bytes(check_dir: &Path, name: &str, png: &[u8]) -> Result<(), CheckError> {
    let path = check_dir.join(name);
    if path.exists() {
        let saved = read_bounded(&path).map_err(|error| match error {
            SampleError::Io { source } => CheckError::Io { source },
            other => CheckError::from(other),
        })?;
        image::load_from_memory(&saved).map_err(|_| CheckError::EvidenceCorrupt {
            detail: format!("{} is not a decodable image", path.display()),
        })?;
        if sha256_hex(&saved) != sha256_hex(png) {
            return Err(CheckError::EvidenceCorrupt {
                detail: format!("{} disagrees with the compared difference", path.display()),
            });
        }
        return Ok(());
    }
    atomic_write(check_dir, name, png).map_err(|source| CheckError::Io { source })?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CheckSchema;

impl serde::Serialize for CheckSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(CHECK_SCHEMA)
    }
}

impl<'de> serde::Deserialize<'de> for CheckSchema {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == CHECK_SCHEMA {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported check schema {value:?}; expected {CHECK_SCHEMA:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CellRecord {
    col: u32,
    row: u32,
    fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsRecord {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitleValues {
    actual: String,
    baseline: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRecord {
    schema: CheckSchema,
    test_id: String,
    gate: Gate,
    reason: MismatchReason,
    candidate_sha256: String,
    baseline_sha256: String,
    changed_fields: Vec<String>,
    changed_cells: Vec<CellRecord>,
    changed_cell_total: usize,
    title_values: Option<TitleValues>,
    pixel_changed_pixels: u64,
    pixel_compared_pixels: u64,
    pixel_bounds: Option<BoundsRecord>,
}

impl ResultRecord {
    fn for_failure(
        test_id: &str,
        gate: Gate,
        reason: MismatchReason,
        candidate_digest: &str,
        baseline_digest: &str,
        state: &StateDiff,
    ) -> Self {
        Self {
            schema: CheckSchema,
            test_id: test_id.to_string(),
            gate,
            reason,
            candidate_sha256: candidate_digest.to_string(),
            baseline_sha256: baseline_digest.to_string(),
            changed_fields: state.changed_fields.clone(),
            changed_cells: state
                .changed_cells
                .iter()
                .map(|cell| CellRecord {
                    col: cell.col(),
                    row: cell.row(),
                    fields: cell.fields().to_vec(),
                })
                .collect(),
            changed_cell_total: state.changed_cell_total,
            title_values: state
                .title_values
                .clone()
                .map(|(actual, baseline)| TitleValues { actual, baseline }),
            pixel_changed_pixels: state.pixel_changed,
            pixel_compared_pixels: state.pixel_compared,
            pixel_bounds: state.pixel_bounds.map(|bounds| BoundsRecord {
                min_x: bounds[0],
                min_y: bounds[1],
                max_x: bounds[2],
                max_y: bounds[3],
            }),
        }
    }

    fn save(&self, check_dir: &Path) -> Result<(), CheckError> {
        let path = check_dir.join("result.json");
        if path.exists() {
            let saved = read_bounded(&path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
            self.validate_saved(&path, &saved)?;
            return Ok(());
        }
        let bytes = serde_json::to_vec(self).map_err(|source| CheckError::EvidenceCorrupt {
            detail: format!("comparison record does not serialize: {source}"),
        })?;
        atomic_write(check_dir, "result.json", &bytes)
            .map_err(|source| CheckError::Io { source })?;
        Ok(())
    }

    fn validate_saved(&self, path: &Path, saved: &[u8]) -> Result<(), CheckError> {
        let corrupt = |detail: String| CheckError::EvidenceCorrupt {
            detail: format!("{}: {detail}", path.display()),
        };
        let parsed: Self = serde_json::from_slice(saved).map_err(|source| {
            corrupt(format!("saved comparison record does not parse: {source}"))
        })?;
        if parsed != *self {
            return Err(corrupt(
                "saved comparison record disagrees with this comparison".to_string(),
            ));
        }
        if !is_hex_digest(&parsed.candidate_sha256) {
            return Err(corrupt("saved candidate digest is not hex".to_string()));
        }
        if parsed.baseline_sha256 != MISSING_BASELINE && !is_hex_digest(&parsed.baseline_sha256) {
            return Err(corrupt("saved baseline digest is not hex".to_string()));
        }
        Ok(())
    }

    fn report_input<'a>(
        &'a self,
        state: &'a StateDiff,
        images: &'a SavedImages,
    ) -> ReportInput<'a> {
        ReportInput {
            test_id: &self.test_id,
            gate: self.gate.as_str(),
            reason: self.reason.as_str(),
            candidate_sha256: &self.candidate_sha256,
            baseline_sha256: &self.baseline_sha256,
            changed_fields: &state.changed_fields,
            changed_cells: &state.changed_cells,
            changed_cell_total: state.changed_cell_total,
            title_values: state
                .title_values
                .as_ref()
                .map(|(actual, baseline)| (actual.as_str(), baseline.as_str())),
            pixels: state.pixel_bounds.map(|bounds| PixelSummary {
                changed: state.pixel_changed,
                compared: state.pixel_compared,
                bounds,
            }),
            has_expected_image: images.has_expected,
            has_actual_image: images.has_actual,
            has_diff_image: images.has_diff,
        }
    }
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) const APPROVAL_LOCK_FILE: &str = ".tuiscotti-approval.lock";

pub(crate) const APPROVAL_LOCK_TIMEOUT_SECS: u64 = 30;

const APPROVAL_LOCK_POLL: std::time::Duration = std::time::Duration::from_millis(5);

pub(crate) struct ApprovalFileLock {
    file: std::fs::File,
}

impl std::fmt::Debug for ApprovalFileLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalFileLock")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum ApprovalLockError {
    Timeout,
    Io(std::io::Error),
}

impl std::fmt::Display for ApprovalLockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(formatter, "approval lock deadline passed"),
            Self::Io(source) => write!(formatter, "approval lock I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ApprovalLockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timeout => None,
            Self::Io(source) => Some(source),
        }
    }
}

impl From<std::io::Error> for ApprovalLockError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl ApprovalFileLock {
    pub(crate) fn acquire(
        baseline_root: &std::path::Path,
        timeout: std::time::Duration,
    ) -> Result<Self, ApprovalLockError> {
        let path = baseline_root.join(APPROVAL_LOCK_FILE);
        let file = open_lock_file(&path)?;
        let start = std::time::Instant::now();
        loop {
            if file.try_lock_exclusive().is_ok() {
                return Ok(Self { file });
            }
            if start.elapsed() >= timeout {
                return Err(ApprovalLockError::Timeout);
            }
            std::thread::sleep(APPROVAL_LOCK_POLL);
        }
    }
}

impl Drop for ApprovalFileLock {
    fn drop(&mut self) {
        drop(self.file.unlock());
    }
}

fn open_lock_file(path: &std::path::Path) -> Result<std::fs::File, ApprovalLockError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path).map_err(ApprovalLockError::Io)
}
