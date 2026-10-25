//! Frozen comparison and candidate evidence (D03, GOAL 14).
//!
//! [`FrozenStore`] compares a candidate [`Sample`] against the approved
//! baseline for its test ID without changing the baseline root. On any
//! mismatch it saves the candidate bytes and the derived evidence under the
//! artifact root, keyed by the compared baseline digest, and returns a
//! [`CheckFailure`] with the candidate path and both digests.
//!
//! Comparison order (GOAL 14.2): read and validate the actual sample, read
//! and validate the baseline when present (`Missing` without making the
//! baseline directory otherwise), require equal test IDs and gates, compare
//! canonical snapshot state first, then the render contract, then upstream
//! `compare_rgba` over DECODED RGBA pixels for `state-and-pixels`. Equal
//! pixels never hide a state difference: state is compared first, and the
//! `state` gate ignores image and render metadata entirely.
//!
//! Store note: these stores hold local project data. They are not a hostile
//! multi-user filesystem service. Store-relative write paths are still
//! checked for symlinks so a link can never redirect evidence outside the
//! store roots, and equal or contained baseline and candidate roots are
//! rejected before any comparison.

use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use image::ImageDecoder as _;
use sha2::{Digest, Sha256};
use tui_test::pixel::{PixelError, compare_rgba};
use tui_test::terminal::cell::Attrs;
use tui_test::{SnapshotCell, TerminalSnapshot};

use crate::report::{PixelSummary, ReportInput, ReportedCell, atomic_write, write_report};
use crate::sample::{
    Gate, MAX_IMAGE_DIM, MAX_RGBA_BYTES, MAX_TEST_ID_BYTES, MAX_TEST_ID_SEGMENT_BYTES, Sample,
    SampleError,
};

/// Schema identifier for `result.json` files written beside the evidence.
pub const CHECK_SCHEMA: &str = "tuiscotti.check.v1";

/// Literal baseline digest recorded when a check finds no baseline file.
pub const MISSING_BASELINE: &str = "missing";

/// Cap on changed cells kept in a failure: the total is exact, the kept
/// detail is row-major deterministic and bounded for report size.
const MAX_REPORTED_CELLS: usize = 64;

/// Every failure a frozen check or evidence write can report.
#[derive(Debug)]
#[non_exhaustive]
pub enum CheckError {
    /// A test ID breaks the Section 14.1 grammar.
    InvalidTestId {
        /// What the test ID broke.
        detail: String,
    },
    /// Baseline and artifact roots are equal, contained, or a
    /// store-relative path leaves its root through a symlink.
    InvalidStorePath {
        /// What the path broke.
        detail: String,
    },
    /// The candidate or baseline sample fails [`Sample`] validation.
    Sample(SampleError),
    /// Upstream pixel comparison rejects decoded image buffers.
    Pixel(PixelError),
    /// Saved candidate or evidence bytes fail re-validation.
    EvidenceCorrupt {
        /// Which saved file failed and how.
        detail: String,
    },
    /// A difference image fails PNG encoding.
    Image {
        /// The PNG encoding failure.
        source: image::ImageError,
    },
    /// A filesystem failure outside sample validation.
    Io {
        /// The filesystem failure.
        source: std::io::Error,
    },
    /// A static HTML report fails to render or save.
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

/// Structured mismatch reason with one value per failure class (GOAL 14.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MismatchReason {
    /// No baseline file exists for the test ID.
    Missing,
    /// Candidate and baseline carry different test IDs.
    TestId,
    /// Candidate and baseline carry different gates.
    Policy,
    /// Canonical snapshot state differs.
    State,
    /// Render contracts differ with equal state.
    Contract,
    /// Decoded image dimensions differ with equal state and contract.
    Dimensions,
    /// Decoded RGBA pixels differ with equal dimensions.
    Pixels,
}

impl MismatchReason {
    /// The wire value recorded in `result.json` and the HTML report.
    #[must_use]
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

/// A passing comparison: the candidate digest and the compared baseline
/// digest (`missing` never appears here: a missing baseline fails).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckSuccess {
    test_id: String,
    candidate_sha256: String,
    baseline_sha256: String,
}

impl CheckSuccess {
    /// The compared test ID.
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// SHA-256 over the candidate sample file bytes.
    #[must_use]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    /// SHA-256 over the compared baseline file bytes.
    #[must_use]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }
}

/// A failing comparison with the saved candidate and evidence locations.
#[derive(Debug, Clone)]
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
    /// The compared test ID.
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// The gate both samples carry.
    #[must_use]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    /// The structured mismatch reason.
    #[must_use]
    pub const fn reason(&self) -> MismatchReason {
        self.reason
    }

    /// SHA-256 over the candidate sample file bytes.
    #[must_use]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    /// SHA-256 over the compared baseline bytes, or `missing`.
    #[must_use]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }

    /// Where the candidate bytes were saved (never a shared mutable file).
    #[must_use]
    pub fn candidate_path(&self) -> &Path {
        &self.candidate_path
    }

    /// The `checks/<baseline_sha256_or_missing>` directory for this run.
    #[must_use]
    pub fn evidence_dir(&self) -> &Path {
        &self.evidence_dir
    }

    /// The static HTML report for this comparison.
    #[must_use]
    pub fn report_path(&self) -> &Path {
        &self.report_path
    }

    /// Changed top-level state fields, in fixed order.
    #[must_use]
    pub fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }

    /// Changed cells with coordinates, row-major, bounded (see the total).
    #[must_use]
    pub fn changed_cells(&self) -> &[ReportedCell] {
        &self.changed_cells
    }

    /// The exact changed-cell count, without the report bound.
    #[must_use]
    pub const fn changed_cell_total(&self) -> usize {
        self.changed_cell_total
    }

    /// Changed pixel count from upstream `compare_rgba` (zero when pixels
    /// were not compared).
    #[must_use]
    pub const fn pixel_changed(&self) -> u64 {
        self.pixel_changed
    }

    /// Compared pixel count (zero after a dimension mismatch).
    #[must_use]
    pub const fn pixel_compared(&self) -> u64 {
        self.pixel_compared
    }

    /// Smallest changed rectangle as `(min_x, min_y, max_x, max_y)`.
    #[must_use]
    pub const fn pixel_bounds(&self) -> Option<[u32; 4]> {
        self.pixel_bounds
    }
}

/// The outcome of [`FrozenStore::check`]: a pass or a saved failure.
#[derive(Debug, Clone)]
pub enum CheckOutcome {
    /// The candidate equals the baseline under its gate.
    Pass(CheckSuccess),
    /// The candidate differs; evidence is saved under the artifact root.
    Failure(CheckFailure),
}

impl CheckOutcome {
    /// True only for an equal comparison.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass(_))
    }
}

/// A read-only comparison handle over a baseline root and an artifact root.
///
/// Checking never writes to the baseline root: no baseline file, no
/// directory, no lock file. Failure evidence goes to the artifact root.
/// There is no approval method here by design (D04 owns approval).
#[derive(Debug, Clone)]
pub struct FrozenStore {
    baselines: PathBuf,
    artifacts: PathBuf,
}

impl FrozenStore {
    /// Open a frozen store over two roots.
    ///
    /// Both roots resolve through their nearest existing ancestors, then
    /// equal roots and contained roots are rejected in either direction.
    /// The artifact root is created when missing; the baseline root is
    /// never created here (a missing baseline root makes every check
    /// `Missing`).
    ///
    /// # Errors
    ///
    /// Returns [`CheckError`] for empty, equal, contained, or
    /// unresolvable roots, and for artifact-root creation failures.
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

    /// The canonical baseline root.
    #[must_use]
    pub fn baseline_root(&self) -> &Path {
        &self.baselines
    }

    /// The canonical artifact root.
    #[must_use]
    pub fn artifact_root(&self) -> &Path {
        &self.artifacts
    }

    /// Compare candidate sample bytes against the stored baseline.
    ///
    /// On failure the candidate and derived evidence are saved and a
    /// [`CheckOutcome::Failure`] is returned. The baseline root is never
    /// written. A missing baseline file is a `Missing` failure without
    /// making the baseline directory.
    ///
    /// # Errors
    ///
    /// Returns [`CheckError`] for invalid samples, test IDs, store paths,
    /// corrupt saved evidence, or I/O failures.
    pub fn check(&self, sample_bytes: &[u8]) -> Result<CheckOutcome, CheckError> {
        let candidate_sha256 = sha256_hex(sample_bytes);
        let actual = Sample::from_bytes(sample_bytes)?;
        validate_test_id(actual.test_id())?;
        let baseline_path = self.baseline_path(actual.test_id())?;
        let baseline_bytes = match fs::read(&baseline_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => return Err(CheckError::Io { source }),
        };
        match baseline_bytes {
            None => self.fail_without_baseline(&actual, &candidate_sha256, sample_bytes),
            Some(bytes) => {
                self.compare_with_baseline(&actual, &candidate_sha256, sample_bytes, &bytes)
            }
        }
    }

    /// Read a sample file with a bounded read, then [`FrozenStore::check`] it.
    ///
    /// # Errors
    ///
    /// Returns [`CheckError`] when the file is unreadable or the check fails.
    pub fn check_file(&self, path: &Path) -> Result<CheckOutcome, CheckError> {
        use std::io::Read as _;
        let file = fs::File::open(path).map_err(|source| CheckError::Io { source })?;
        let mut bounded = file.take(crate::sample::MAX_SAMPLE_FILE_BYTES + 1);
        let mut bytes = Vec::new();
        bounded
            .read_to_end(&mut bytes)
            .map_err(|source| CheckError::Io { source })?;
        self.check(&bytes)
    }

    /// Compare against present baseline bytes after both sides validate.
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

    /// Compare render contracts, then decoded RGBA pixels. State is already
    /// equal here, so equal pixels can never hide a state difference.
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

    /// Record a `Missing` failure: candidate plus evidence, no baseline dir.
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

    /// Resolve the baseline file for a test ID inside the baseline root.
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

    /// Resolve a not-yet-existing baseline path: the nearest existing
    /// ancestor must stay inside the baseline root.
    fn missing_baseline_path(&self, joined: &Path) -> Result<PathBuf, CheckError> {
        let mut cursor = joined;
        loop {
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

/// Maximum display characters kept per compared value in reports.
const MAX_DISPLAY_CHARS: usize = 200;

/// Cell attribute flags compared field by field, in fixed report order.
const CELL_FLAGS: [(&str, Attrs); 7] = [
    ("bold", Attrs::BOLD),
    ("dim", Attrs::DIM),
    ("italic", Attrs::ITALIC),
    ("inverse", Attrs::INVERSE),
    ("invisible", Attrs::INVISIBLE),
    ("strike", Attrs::STRIKE),
    ("blink", Attrs::BLINK),
];

/// Pure state-comparison detail shared by every failure record and report.
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
    /// A non-state reason with one named field and no cell detail.
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

    /// A pixel reason: no state fields changed, only pixel counts.
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

    /// True when neither a field nor a cell changed.
    fn is_equal(&self) -> bool {
        self.changed_fields.is_empty() && self.changed_cell_total == 0
    }
}

/// Decoded difference image in row-major RGBA order with its dimensions.
struct DiffPng {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// The pixel comparison verdict: equal, dimension-mismatched, or different.
enum PixelVerdict {
    /// Decoded RGBA pixels are equal.
    Equal,
    /// Decoded image dimensions differ.
    Dimensions,
    /// Pixels differ with counts, bounds, and a difference image.
    Different {
        changed: u64,
        compared: u64,
        bounds: Option<[u32; 4]>,
        diff: DiffPng,
    },
}

/// Decode both images and run upstream `compare_rgba` over DECODED RGBA,
/// never PNG bytes. State and contract are already equal at this point.
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

/// Validate a test ID against the Section 14.1 grammar on every platform:
///
/// ```text
/// segment = [a-z0-9_-]{1,64}
/// test_id = segment ("/" segment)*
/// ```
///
/// Total length follows Section 8.4 (240 ASCII bytes). Absolute paths,
/// empty segments, backslashes, and `.`/`..` segments are rejected without
/// normalization. Windows device names (`con`, `prn`, `aux`, `nul`,
/// `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without an
/// extension) are rejected as segments on all platforms.
///
/// # Errors
///
/// Returns [`CheckError::InvalidTestId`] for every rejected shape.
pub fn validate_test_id(test_id: &str) -> Result<(), CheckError> {
    let invalid = |detail: &str| CheckError::InvalidTestId {
        detail: detail.to_string(),
    };
    if test_id.is_empty() {
        return Err(invalid("test ID is empty"));
    }
    if test_id.len() > MAX_TEST_ID_BYTES {
        return Err(CheckError::InvalidTestId {
            detail: format!(
                "test ID is {} bytes, above the {MAX_TEST_ID_BYTES} byte limit",
                test_id.len()
            ),
        });
    }
    if !test_id.is_ascii() {
        return Err(invalid("test ID is not ASCII"));
    }
    if test_id.contains('\\') {
        return Err(invalid("test ID contains a backslash"));
    }
    if test_id.starts_with('/') {
        return Err(invalid("test ID is an absolute path"));
    }
    for segment in test_id.split('/') {
        if segment.is_empty() {
            return Err(invalid("test ID has an empty segment"));
        }
        if segment.len() > MAX_TEST_ID_SEGMENT_BYTES {
            return Err(CheckError::InvalidTestId {
                detail: format!(
                    "test ID segment {segment:?} passes the {MAX_TEST_ID_SEGMENT_BYTES} byte limit"
                ),
            });
        }
        if segment == "." || segment == ".." {
            return Err(CheckError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a directory reference"),
            });
        }
        let shaped = segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        });
        if !shaped {
            return Err(CheckError::InvalidTestId {
                detail: format!("test ID segment {segment:?} breaks the [a-z0-9_-] shape"),
            });
        }
        if is_device_name(segment) {
            return Err(CheckError::InvalidTestId {
                detail: format!("test ID segment {segment:?} is a reserved device name"),
            });
        }
    }
    Ok(())
}

/// True for Windows device names on any platform: `con`, `prn`, `aux`,
/// `nul`, `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive, with or without a
/// dotted extension. The caller guarantees ASCII input.
fn is_device_name(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let stem = match lower.split('.').next() {
        Some(stem) => stem,
        None => lower.as_str(),
    };
    if matches!(stem, "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let bytes = stem.as_bytes();
    if bytes.len() == 4
        && (stem.starts_with("com") || stem.starts_with("lpt"))
        && bytes[3].is_ascii_digit()
        && bytes[3] != b'0'
    {
        return true;
    }
    false
}

/// Compare two snapshots through canonical equality, collecting changed
/// top-level fields (fixed order) and changed cells (row-major).
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
    let full = diff_cells(actual, baseline);
    let changed_cell_total = full.len();
    let changed_cells = full.into_iter().take(MAX_REPORTED_CELLS).collect();
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

/// Truncate a compared value to a fixed display length on a char boundary.
fn display_value(value: &str) -> String {
    if value.chars().count() <= MAX_DISPLAY_CHARS {
        return value.to_string();
    }
    let kept: String = value.chars().take(MAX_DISPLAY_CHARS).collect();
    format!("{kept}…")
}

/// Compare cursor state field by field; an absent cursor difference keeps
/// the field list stable.
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

/// Compare the cell grids; cells outside the shared area count as changed.
fn diff_cells(actual: &TerminalSnapshot, baseline: &TerminalSnapshot) -> Vec<ReportedCell> {
    let cols = actual.cols().max(baseline.cols());
    let rows = actual.rows().max(baseline.rows());
    let mut changed = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let pair = (actual.cell_at(col, row), baseline.cell_at(col, row));
            let (fields, actual_text, baseline_text) = match pair {
                (Some(left), Some(right)) => (
                    diff_cell(left, right),
                    display_value(left.text()),
                    display_value(right.text()),
                ),
                (Some(left), None) => (
                    vec!["cell".to_string()],
                    display_value(left.text()),
                    String::new(),
                ),
                _ => (
                    vec!["cell".to_string()],
                    String::new(),
                    display_value(baseline.cell_at(col, row).map_or("", SnapshotCell::text)),
                ),
            };
            if !fields.is_empty() {
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
    changed
}

/// Compare two cells through read-only accessors, in fixed field order.
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

/// Resolve a store root: canonicalize it, or its nearest existing ancestor
/// with the missing remainder appended lexically. A `..` remainder is
/// rejected rather than resolved.
fn resolve_root(path: &Path, role: &str) -> Result<PathBuf, CheckError> {
    if path.as_os_str().is_empty() {
        return Err(CheckError::InvalidStorePath {
            detail: format!("{role} root path is empty"),
        });
    }
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

/// Reject symlinks in every existing prefix of a store-relative path, so a
/// link can never redirect a write outside its store root.
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

/// SHA-256 hex over raw bytes.
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

/// Decode an embedded sample PNG with dimension validation before the pixel
/// buffer is allocated. Returns raw bytes, width, height, and RGBA pixels.
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

/// Enforce image dimension limits on numbers, before any allocation.
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

/// Checked `width * height * 4` for an RGBA8 buffer length.
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

/// Encode a difference image as PNG bytes for evidence.
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
    /// Save the candidate and derived evidence, then describe the failure.
    ///
    /// Evidence is keyed by the compared baseline digest (`missing` when
    /// none): `artifacts/<id>/<candidate>/checks/<baseline>/`. Saved
    /// candidate and evidence bytes are re-validated before reuse; corrupt
    /// or digest-mismatched files are a typed error, never a silent
    /// overwrite. The baseline is never written here.
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
        if !report_path.exists() {
            let input = record.report_input(state, &images);
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

    /// Resolve the candidate directory, rejecting symlink redirection.
    fn candidate_dir(&self, test_id: &str, candidate_digest: &str) -> Result<PathBuf, CheckError> {
        let relative: PathBuf = [test_id, candidate_digest].iter().collect();
        ensure_no_symlink(&self.artifacts, &relative)?;
        Ok(self.artifacts.join(&relative))
    }

    /// The store-relative check directory for one baseline comparison.
    fn relative_dir(test_id: &str, candidate_digest: &str, baseline_digest: &str) -> PathBuf {
        [test_id, candidate_digest, "checks", baseline_digest]
            .iter()
            .collect()
    }

    /// Save the candidate bytes once; re-validate them on later checks.
    fn save_candidate(
        candidate_dir: &Path,
        candidate_digest: &str,
        candidate_bytes: &[u8],
    ) -> Result<PathBuf, CheckError> {
        fs::create_dir_all(candidate_dir).map_err(|source| CheckError::Io { source })?;
        let path = candidate_dir.join("sample.tui.json");
        if path.exists() {
            let saved = fs::read(&path).map_err(|source| CheckError::Io { source })?;
            validate_saved_sample(&path, &saved, candidate_digest)?;
            return Ok(path);
        }
        atomic_write(candidate_dir, "sample.tui.json", candidate_bytes)
            .map_err(|source| CheckError::Io { source })?;
        Ok(path)
    }

    /// Save the compared baseline bytes (`expected.tui.json`); never read a
    /// newer baseline during later reporting, never overwrite another
    /// digest's evidence.
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
            let saved = fs::read(&path).map_err(|source| CheckError::Io { source })?;
            validate_saved_sample(&path, &saved, baseline_digest)?;
            return Ok(());
        }
        atomic_write(check_dir, "expected.tui.json", bytes)
            .map_err(|source| CheckError::Io { source })?;
        Ok(())
    }

    /// Save `expected.png`, `actual.png`, and `diff.png` for pixel gates.
    /// State gates carry no images; a missing baseline has no expected one.
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
            let expected_png = sample_png_bytes(bytes)?;
            save_png(check_dir, "expected.png", &expected_png)?;
            images.has_expected = baseline_digest != MISSING_BASELINE;
        }
        if let Some(diff) = diff {
            let encoded = encode_diff_png(diff)?;
            save_png_bytes(check_dir, "diff.png", &encoded)?;
            images.has_diff = true;
        }
        Ok(images)
    }
}

/// Which sibling images a report may link to.
struct SavedImages {
    has_expected: bool,
    has_actual: bool,
    has_diff: bool,
}

impl SavedImages {
    /// No images: state gates and missing baselines.
    const fn none() -> Self {
        Self {
            has_expected: false,
            has_actual: false,
            has_diff: false,
        }
    }
}

/// Re-validate saved sample bytes before reuse: they must parse and their
/// digest must match the key that stored them.
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

/// Raw PNG bytes embedded in validated sample bytes.
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

/// Save PNG bytes, validating any present file by decoding it.
fn save_png(check_dir: &Path, name: &str, png: &[u8]) -> Result<(), CheckError> {
    let path = check_dir.join(name);
    if path.exists() {
        let saved = fs::read(&path).map_err(|source| CheckError::Io { source })?;
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

/// Save encoded difference bytes, validating any present file by decoding.
fn save_png_bytes(check_dir: &Path, name: &str, png: &[u8]) -> Result<(), CheckError> {
    let path = check_dir.join(name);
    if path.exists() {
        let saved = fs::read(&path).map_err(|source| CheckError::Io { source })?;
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

/// The schema identifier as an explicit type: only [`CHECK_SCHEMA`] parses.
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

/// One changed-cell record in `result.json`: coordinates plus field names.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CellRecord {
    col: u32,
    row: u32,
    fields: Vec<String>,
}

/// One pixel-bounds record in `result.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsRecord {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

/// Compared window titles in `result.json`, display-truncated.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitleValues {
    actual: String,
    baseline: String,
}

/// The versioned comparison record saved as `result.json` beside the
/// evidence. Evidence keying, report rendering, and re-validation all read
/// this record back; unknown fields and unsupported schemas are rejected.
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
    /// Build the record for one failure.
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

    /// Save the record, or re-validate a present one against this failure.
    fn save(&self, check_dir: &Path) -> Result<(), CheckError> {
        let path = check_dir.join("result.json");
        if path.exists() {
            let saved = fs::read(&path).map_err(|source| CheckError::Io { source })?;
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

    /// Re-validate a saved record: schema, identity, reason, and digests.
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

    /// The report input for this failure.
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

/// True for 64 lowercase hex characters: a SHA-256 digest shape.
fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

// ---------------------------------------------------------------------------
// D04 approval-lock region (GOAL Section 15). Only approval-lock items live
// in this region: the lock file name, the acquisition deadline, the
// fresh-file lock guard, and its unit tests. No comparison, evidence, or
// store-path logic changes here.
// ---------------------------------------------------------------------------

/// File name of the per-root approval lock inside the baseline root.
///
/// The lock file is created fresh by its holder with exclusive creation, so
/// a holder never opens a pre-existing file as its lock. It exists only
/// while the approval transaction runs and is removed on release. Presence
/// of the file carries no cross-process meaning: locks are not portable, so
/// no cross-process before-or-after file semantics follow from it.
pub(crate) const APPROVAL_LOCK_FILE: &str = ".tuiscotti-approval.lock";

/// How long approval waits for a held lock before it reports a timeout.
pub(crate) const APPROVAL_LOCK_TIMEOUT_SECS: u64 = 30;

/// Poll interval while approval waits for a held lock to release.
const APPROVAL_LOCK_POLL: std::time::Duration = std::time::Duration::from_millis(5);

/// A held per-root approval lock.
///
/// The guard owns the exclusively created lock file: creation and removal
/// bracket the transaction, so no stale lock file survives a completed or
/// failed transaction. A process that dies mid-transaction can leave the
/// empty file behind; its presence alone never grants the lock, and an
/// operator removes it.
pub(crate) struct ApprovalFileLock {
    path: std::path::PathBuf,
}

/// Why approval lock acquisition failed.
#[derive(Debug)]
pub(crate) enum ApprovalLockError {
    /// Another holder kept the lock past the acquisition deadline.
    Timeout,
    /// The lock file could not be created, read, or removed.
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
    /// Acquire the approval lock for a canonical baseline root.
    ///
    /// The lock file is created exclusively: when a regular file already
    /// exists, this call waits for the deadline instead of opening the held
    /// file, then reports [`ApprovalLockError::Timeout`]. A directory or
    /// symlink at the lock path fails immediately. This call never creates
    /// directories; the root must already exist.
    pub(crate) fn acquire(
        baseline_root: &std::path::Path,
        timeout: std::time::Duration,
    ) -> Result<Self, ApprovalLockError> {
        let path = baseline_root.join(APPROVAL_LOCK_FILE);
        let start = std::time::Instant::now();
        loop {
            match create_lock_file(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !held_lock_is_a_file(&path)? {
                        return Err(ApprovalLockError::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "approval lock path {} is not a regular file",
                                path.display()
                            ),
                        )));
                    }
                    if start.elapsed() >= timeout {
                        return Err(ApprovalLockError::Timeout);
                    }
                    std::thread::sleep(APPROVAL_LOCK_POLL);
                }
                Err(error) => return Err(ApprovalLockError::Io(error)),
            }
        }
    }
}

impl Drop for ApprovalFileLock {
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.path));
    }
}

/// True when the path blocking lock creation is a regular file: a held
/// lock that may release. Anything else at the lock path (a directory, a
/// symlink, or a raced removal) is never waited on: directories and links
/// fail fast, and a disappearance retries creation.
fn held_lock_is_a_file(path: &std::path::Path) -> Result<bool, ApprovalLockError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => Ok(meta.file_type().is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(ApprovalLockError::Io(error)),
    }
}

/// Create the lock file exclusively with owner-only permissions on Unix.
///
/// `create_new` fails when the path already exists, so a second holder can
/// never reuse the first holder's file as its own lock. The handle closes
/// on return; the file's existence is the lock signal.
fn create_lock_file(path: &std::path::Path) -> Result<(), std::io::Error> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path).map(|_| ())
}

#[cfg(test)]
mod approval_lock_tests {
    use super::{APPROVAL_LOCK_FILE, ApprovalFileLock};
    use std::time::Duration;

    /// A fresh directory for lock tests, removed on drop.
    fn lock_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("lock test directory creates")
    }

    #[test]
    fn acquire_creates_a_fresh_lock_file() {
        let dir = lock_dir();
        let expected = dir.path().join(APPROVAL_LOCK_FILE);
        assert!(!expected.exists(), "no lock file before acquisition");
        let _lock = ApprovalFileLock::acquire(dir.path(), Duration::from_secs(5))
            .expect("first acquisition succeeds");
        assert!(expected.exists(), "lock file exists while held");
    }

    #[test]
    fn release_removes_the_lock_file() {
        let dir = lock_dir();
        let expected = dir.path().join(APPROVAL_LOCK_FILE);
        let lock = ApprovalFileLock::acquire(dir.path(), Duration::from_secs(5))
            .expect("acquisition succeeds");
        drop(lock);
        assert!(!expected.exists(), "lock file is gone after release");
    }

    #[test]
    fn held_lock_times_out_instead_of_opening_the_file() {
        let dir = lock_dir();
        let _held = ApprovalFileLock::acquire(dir.path(), Duration::from_secs(5))
            .expect("first acquisition succeeds");
        let started = std::time::Instant::now();
        let outcome = ApprovalFileLock::acquire(dir.path(), Duration::from_millis(120));
        assert!(
            matches!(outcome, Err(super::ApprovalLockError::Timeout)),
            "a held lock reports a timeout"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the wait stops at its deadline"
        );
    }

    #[test]
    fn release_frees_the_lock_for_the_next_holder() {
        let dir = lock_dir();
        let first = ApprovalFileLock::acquire(dir.path(), Duration::from_secs(5))
            .expect("first acquisition succeeds");
        drop(first);
        let second = ApprovalFileLock::acquire(dir.path(), Duration::from_secs(5));
        assert!(second.is_ok(), "the next holder acquires a fresh file");
    }
}

#[cfg(test)]
mod tests {
    use super::validate_test_id;
    use crate::sample::SampleError;

    /// The full Section 14.1 reserved device list, platform-independent.
    fn device_names() -> Vec<String> {
        let mut names = vec![
            "con".to_string(),
            "prn".to_string(),
            "aux".to_string(),
            "nul".to_string(),
        ];
        for port in 1..=9u8 {
            names.push(format!("com{port}"));
            names.push(format!("lpt{port}"));
        }
        names
    }

    #[test]
    fn valid_ids_pass_on_any_platform() {
        for valid in ["a", "views/login", "a-b_c/9x", "x/y/z", &"s".repeat(64)] {
            assert!(
                validate_test_id(valid).is_ok(),
                "valid test ID {valid:?} must pass"
            );
        }
    }

    #[test]
    fn device_names_fail_on_any_platform() {
        assert_eq!(device_names().len(), 22);
        for name in device_names() {
            assert!(
                validate_test_id(&name).is_err(),
                "device name {name:?} must fail"
            );
            let nested = format!("views/{name}/login");
            assert!(
                validate_test_id(&nested).is_err(),
                "nested device name {nested:?} must fail"
            );
            let upper = format!("views/{}", name.to_ascii_uppercase());
            assert!(
                validate_test_id(&upper).is_err(),
                "uppercase device name {upper:?} must fail"
            );
        }
    }

    #[test]
    fn malformed_ids_fail_on_any_platform() {
        for invalid in [
            "",
            "/views/login",
            "views/login/",
            "views//login",
            "views/./login",
            "views/../login",
            ".",
            "..",
            "views\\login",
            "views/Login",
            "views/log in",
            "views/lögn",
        ] {
            assert!(
                validate_test_id(invalid).is_err(),
                "malformed test ID {invalid:?} must fail"
            );
        }
        let long_segment = "s".repeat(65);
        assert!(validate_test_id(&long_segment).is_err());
        let long_id = [
            "s".repeat(60),
            "s".repeat(60),
            "s".repeat(60),
            "s".repeat(60),
        ]
        .join("/");
        assert!(long_id.len() > 240);
        assert!(validate_test_id(&long_id).is_err());
    }

    /// A missing upstream difference image is its own error variant, never a
    /// dimension mismatch.
    #[test]
    fn missing_difference_image_has_dedicated_variant() {
        let error = SampleError::MissingDifferenceImage {
            detail: "upstream pixel comparison reports changed pixels without a difference image"
                .to_string(),
        };
        assert!(matches!(error, SampleError::MissingDifferenceImage { .. }));
        assert!(!matches!(error, SampleError::DimensionMismatch { .. }));
        assert!(error.to_string().contains("difference image"));
    }
}
