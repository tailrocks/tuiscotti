//! Baseline approval with digests and frozen checking (D04, GOAL Section 15).
//!
//! [`ApprovalStore::accept`] runs one explicit approval transaction for one
//! test ID: it refuses under `CI`, validates both digests, locks the stable
//! per-root file, reads the candidate once, arbitrates digests against the
//! baseline read under the same lock, re-renders image samples, then
//! atomically upserts the baseline and saves the approval evidence. The
//! baseline root is never trusted blindly: the expected previous digest (or
//! the literal `missing` for a first approval) must match the digest of the
//! bytes actually present, and the candidate digest must match the bytes
//! actually approved.
//!
//! What approval does and does not do:
//!
//! - It validates the candidate with [`Sample::from_bytes`], which checks
//!   the schema, test ID, gate pairing, snapshot, and, for a
//!   `state-and-pixels` sample, the recorded PNG and RGBA digests against the
//!   decoded image bytes. Frame and image agreement is structural.
//! - For an image sample it then re-renders the candidate snapshot through
//!   the upstream renderer under the candidate's SPECIFIED profile (steps
//!   10-12): an unresolvable profile rejects with
//!   [`ApproveError::UnknownProfile`], a renderer refusal rejects with
//!   [`ApproveError::RenderFailed`] (or [`ApproveError::MissingGlyph`] when
//!   no specified face holds mandatory text), and decoded pixels that differ
//!   from the candidate pixels reject with
//!   [`ApproveError::InconsistentSample`]. State-only samples skip steps
//!   10-12 ("if the sample has an image").
//! - It validates the candidate test-ID grammar only on first approval
//!   (absent baseline). When a baseline is present, no test-ID or gate
//!   cross-comparison runs at all: the present-baseline `FrozenStore`
//!   comparison classifies observed evidence shapes
//!   (snapshot state, render contracts, decoded pixels) without comparing
//!   ID or gate enums. The operation targets the candidate test ID; the
//!   baseline path derives from that ID.
//! - It records the change set for `report.json` from that same comparison
//!   (state first, then render contract, then decoded RGBA pixels through
//!   upstream `compare_rgba`), over the stored bytes only. The smallest
//!   change set wins; no caller-supplied flags steer it.
//! - It reads the baseline bytes exactly once per transaction, directly
//!   from the store under the held lock. The candidate bytes are read once
//!   and reused by digest, validation, comparison, and upsert, so the
//!   candidate reference stays consistent throughout.
//!
//! Transaction order (GOAL Section 15, numbered steps):
//!
//! 1. Refuse when the `CI` variable is set, before any write.
//! 2. Validate the digest arguments (missing, malformed, and zero digests
//!    are rejected as `INVALID_SAMPLE`) and store paths.
//! 3. Make the baseline root when missing.
//! 4. Acquire the operating-system lock on the stable approval file.
//! 5. Read the candidate bytes one time with the file-size limit.
//! 6. Compare their digest with the mandatory candidate digest.
//! 7. Validate the candidate schema and snapshot.
//! 8. Read the baseline from the store under the same lock.
//! 9. Compare its digest with the mandatory baseline digest.
//! 10. For an image sample, resolve the candidate's specified render profile.
//! 11. Re-render the candidate snapshot through the upstream renderer.
//! 12. Compare the new decoded pixels with the candidate pixels.
//! 13. Validate the store-relative paths for the baseline's parent directories.
//! 14. Make the baseline's parent directories under the locked baseline root.
//! 15. Write the approved sample bytes unchanged to a same-directory temporary file (exclusive, owner-only, no symlinks).
//! 16. Flush the temporary file.
//! 17. Close the temporary file.
//! 18. Atomically rename the temporary file over the baseline (same-directory renames cannot EXDEV, so no copy fallback).
//! 19. Release the lock (the stable lock file persists).
//! 20. Return the approved test ID and sample digest.
//!
//! Locking: one lock per baseline root, held from step 4 through step 19,
//! across the baseline read, digest comparison, re-render, and atomic
//! replacement. The lock file `<root>/.tuiscotti-approval.lock` is stable:
//! it is created once when missing, opened (never recreated) by every
//! holder, and kept after release. Mutual exclusion is the OS file lock on
//! the open handle, so holders serialize across processes as well as
//! threads, and a dead process leaves a stale file that never blocks. The
//! wait stops after 30 seconds with a typed timeout.
//!
//! Frozen checking: [`FrozenStore`] has no approval method and gains none
//! here; checking never writes the baseline root. Approval under a frozen
//! store is refused in two shapes: a read-only baseline tree fails the lock
//! or the write with a typed I/O error and writes nothing, and any set `CI`
//! variable (any value, including empty) refuses approval before any write
//! with [`ApproveError::CiApprovalForbidden`]. Each approval call is its own
//! explicit opt-in through its digest arguments; nothing unfreezes a root
//! persistently.
//!
//! Error codes (GOAL Section 16.3): there is no `INVALID_DIGEST` code.
//! Malformed digest arguments report `INVALID_SAMPLE`; every other
//! failure reports its Section 16.3 code, mapped per failure shape in
//! [`ApproveError::code`] (unsupported schemas, resource limits, image
//! errors, unknown profiles, render failures, missing glyphs, stale
//! digests, inconsistent samples, lock timeouts, store paths, and I/O
//! each keep their own code instead of flattening to `INVALID_SAMPLE` or
//! `INVALID_IMAGE`). Process exit codes are owned by the CLI boundary,
//! which maps the argument-error shape to 2 and every operational
//! failure to 3.
//!
//! Volatile-data exclusion: the records written during approval
//! (`report.json` and `candidate_bytes.evidence.json`) carry no absolute
//! paths, process IDs, timestamps, host names, or session names. They carry
//! only the test ID, gate, digests, and the compared change set, all of
//! which recompute deterministically from the approved bytes.
//!
//! CLI shape for D05 (recorded, not implemented here): the programmatic path
//! below is what the future offline command calls.
//!
//! ```text
//! tuiscotti accept --candidate PATH --candidate-sha256 SHA256 --expect-baseline SHA256_OR_MISSING --baselines DIR
//! ```
//!
//! No command equivalence is promised here: D05 owns argument parsing, JSON
//! output, and process exit codes. The stable machine codes from
//! [`ApproveError::code`] are the mapping inputs for that command.
//!
//! Design notes: approval reuses the frozen differ through `FrozenStore`
//! instead of duplicating it, so check and
//! accept cannot disagree on the change set. Small byte-level helpers
//! (digest shape, root resolution, symlink guard, SHA-256) mirror `check.rs`
//! without sharing code, because only lock and comparison helpers are
//! shared across the two files.

use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

use tui_test::{PinnedFontError, RenderError, render_snapshot_png};

use crate::check::{
    APPROVAL_LOCK_TIMEOUT_SECS, ApprovalFileLock, ApprovalLockError, CheckError, FrozenStore,
    MISSING_BASELINE, MismatchReason, PresentComparison, validate_test_id,
};
use crate::render_profile::{ProfileError, resolve_profile};
use crate::sample::{Gate, Sample, SampleError, SampleImage, read_bounded, read_bounded_optional};

/// Schema identifier for the `report.json` files written by approval.
pub const ACCEPT_SCHEMA: &str = "tuiscotti.accept.v1";

/// Schema identifier for the `candidate_bytes.evidence.json` files.
pub const EVIDENCE_SCHEMA: &str = "tuiscotti.candidate-evidence.v1";

/// Reserved directory inside the baseline root holding approval evidence.
///
/// Test IDs can never collide with it: `.` starts no valid segment.
const ACCEPT_EVIDENCE_DIR: &str = ".tuiscotti-accept";

/// Every failure an approval transaction can report, with sources retained.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApproveError {
    /// The `CI` variable is set, so approval is refused before any write.
    CiApprovalForbidden,
    /// A digest argument is missing, malformed, or a zero placeholder.
    /// Digest arguments are sample-selection input, so these report
    /// `INVALID_SAMPLE`; the CLI boundary maps them to the argument-error
    /// exit code 2.
    InvalidArgument {
        /// Which argument failed and how.
        detail: String,
    },
    /// The candidate bytes disagree with the supplied candidate digest.
    StaleCandidate {
        /// The supplied candidate digest.
        expected: String,
        /// The digest of the bytes actually read.
        actual: String,
    },
    /// The present baseline disagrees with the expected previous digest.
    StaleBaseline {
        /// The expected previous digest, or `missing`.
        expected: String,
        /// The digest of the bytes actually present, or `missing`.
        actual: String,
    },
    /// The approval lock stayed held past the acquisition deadline.
    LockTimeout,
    /// The candidate names a render profile outside the D02 registry, so
    /// steps 10-12 cannot resolve it.
    UnknownProfile {
        /// The rejected profile name.
        requested: String,
    },
    /// The upstream renderer refused the candidate snapshot during the
    /// step-11 re-render.
    RenderFailed {
        /// The upstream rejection, with its source chain.
        source: RenderError,
    },
    /// No specified font can render mandatory text in the candidate
    /// snapshot during the step-11 re-render.
    MissingGlyph {
        /// The upstream missing-glyph rejection.
        source: PinnedFontError,
    },
    /// The step-12 comparison found re-rendered pixels that differ from
    /// the stored candidate pixels.
    InconsistentSample {
        /// What disagreed (dimensions and compared byte counts).
        detail: String,
    },
    /// A baseline or evidence path is empty, unresolvable, or escapes its
    /// root through a symlink.
    InvalidStorePath {
        /// What the path broke.
        detail: String,
    },
    /// The inner frozen comparison, candidate validation, or record
    /// serialization failed.
    Check {
        /// The frozen-store failure, with its source chain.
        source: CheckError,
    },
    /// A filesystem failure outside sample validation.
    Io {
        /// The filesystem failure.
        source: std::io::Error,
    },
}

impl Display for ApproveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CiApprovalForbidden => {
                write!(formatter, "approval is forbidden while CI is set")
            }
            Self::InvalidArgument { detail } => {
                write!(formatter, "invalid approval argument: {detail}")
            }
            Self::StaleCandidate { expected, actual } => write!(
                formatter,
                "candidate changed: expected digest {expected}, read bytes digest {actual}"
            ),
            Self::StaleBaseline { expected, actual } => write!(
                formatter,
                "baseline changed: expected previous digest {expected}, present digest {actual}"
            ),
            Self::LockTimeout => write!(formatter, "approval lock deadline passed"),
            Self::UnknownProfile { requested } => {
                write!(formatter, "unknown render profile {requested:?}")
            }
            Self::RenderFailed { source } => {
                write!(formatter, "candidate re-render failed: {source}")
            }
            Self::MissingGlyph { source } => {
                write!(
                    formatter,
                    "candidate re-render needs an unavailable glyph: {source}"
                )
            }
            Self::InconsistentSample { detail } => {
                write!(formatter, "candidate re-render is inconsistent: {detail}")
            }
            Self::InvalidStorePath { detail } => {
                write!(formatter, "invalid store path: {detail}")
            }
            Self::Check { source } => write!(formatter, "{source}"),
            Self::Io { source } => write!(formatter, "approval I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Check { source } => Some(source),
            Self::RenderFailed { source } => Some(source),
            Self::MissingGlyph { source } => Some(source),
            Self::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CheckError> for ApproveError {
    fn from(source: CheckError) -> Self {
        Self::Check { source }
    }
}

impl From<SampleError> for ApproveError {
    fn from(source: SampleError) -> Self {
        Self::Check {
            source: CheckError::from(source),
        }
    }
}

impl From<std::io::Error> for ApproveError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}

impl ApproveError {
    /// The stable machine code for this failure, shared with the CLI map.
    ///
    /// Every Section 16.3 code is producible here and maps 1:1 with its
    /// failure shape: malformed digest arguments are `INVALID_SAMPLE`
    /// (there is no `INVALID_DIGEST` code), and sample, image, profile,
    /// render, stale, lock, path, and filesystem failures keep their own
    /// codes instead of flattening to `INVALID_SAMPLE` or `INVALID_IMAGE`.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CiApprovalForbidden => "CI_APPROVAL_FORBIDDEN",
            Self::InvalidArgument { .. } => "INVALID_SAMPLE",
            Self::StaleCandidate { .. } => "STALE_CANDIDATE",
            Self::StaleBaseline { .. } => "STALE_BASELINE",
            Self::LockTimeout => "LOCK_TIMEOUT",
            Self::UnknownProfile { .. } => "UNKNOWN_PROFILE",
            Self::RenderFailed { .. } => "RENDER_FAILED",
            Self::MissingGlyph { .. } => "MISSING_GLYPH",
            Self::InconsistentSample { .. } => "INCONSISTENT_SAMPLE",
            Self::InvalidStorePath { .. } => "INVALID_STORE_PATH",
            Self::Check { source } => match source {
                CheckError::Sample(source) => sample_code(source),
                CheckError::InvalidTestId { .. } | CheckError::InvalidStorePath { .. } => {
                    "INVALID_STORE_PATH"
                }
                CheckError::Pixel(_) => "INVALID_IMAGE",
                CheckError::EvidenceCorrupt { .. }
                | CheckError::Image { .. }
                | CheckError::Io { .. }
                | CheckError::Report(_) => "IO",
            },
            Self::Io { .. } => "IO",
        }
    }

    /// The digest the baseline root actually held when this failure was
    /// reported: the present digest for a stale baseline (or `missing`
    /// when no baseline file was present), and `None` when the failure
    /// carries no observed baseline.
    #[must_use]
    pub fn observed_baseline(&self) -> Option<&str> {
        match self {
            Self::StaleBaseline { actual, .. } => Some(actual),
            _ => None,
        }
    }
}

/// The Section 16.3 code for one validated sample failure.
///
/// Resource exhaustion (`TooLarge`, `DimensionTooLarge`, `RgbaTooLarge`)
/// is `RESOURCE_LIMIT`; an unknown sample schema is `UNSUPPORTED_SCHEMA`;
/// PNG, digest, and dimension disagreements are `INVALID_IMAGE`; renderer
/// refusals are `RENDER_FAILED` except a missing glyph, which is
/// `MISSING_GLYPH`; filesystem failures are `IO`. Everything else —
/// test IDs, gates, formats, snapshots, contracts, and evidence shapes —
/// is `INVALID_SAMPLE`.
#[must_use]
pub fn sample_code(source: &SampleError) -> &'static str {
    match source {
        SampleError::TooLarge { .. }
        | SampleError::DimensionTooLarge { .. }
        | SampleError::RgbaTooLarge { .. } => "RESOURCE_LIMIT",
        SampleError::UnsupportedSchema { .. } => "UNSUPPORTED_SCHEMA",
        SampleError::PngInvalid { .. }
        | SampleError::PngDecode { .. }
        | SampleError::Base64Invalid { .. }
        | SampleError::DigestMismatch { .. }
        | SampleError::DimensionMismatch { .. } => "INVALID_IMAGE",
        SampleError::Render { source } => match source {
            RenderError::PinnedFont(PinnedFontError::MissingGlyph { .. }) => "MISSING_GLYPH",
            _ => "RENDER_FAILED",
        },
        SampleError::Io { .. } => "IO",
        _ => "INVALID_SAMPLE",
    }
}

/// One completed approval transaction: the reviewed candidate is now the
/// approved baseline for its test ID.
#[derive(Debug, Clone)]
pub struct AcceptOutcome {
    test_id: String,
    gate: Gate,
    candidate_sha256: String,
    previous_baseline_sha256: String,
    baseline_sha256: String,
    baseline_path: PathBuf,
    evidence_dir: PathBuf,
}

impl AcceptOutcome {
    /// The approved test ID.
    #[must_use]
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// The gate both the candidate and the new baseline carry.
    #[must_use]
    pub const fn gate(&self) -> Gate {
        self.gate
    }

    /// SHA-256 over the approved candidate bytes.
    #[must_use]
    pub fn candidate_sha256(&self) -> &str {
        &self.candidate_sha256
    }

    /// The previous baseline digest from the prepared transaction, or
    /// `missing` for a first approval.
    #[must_use]
    pub fn previous_baseline_sha256(&self) -> &str {
        &self.previous_baseline_sha256
    }

    /// SHA-256 over the written baseline bytes; always the candidate digest,
    /// because approval writes the reviewed bytes unchanged.
    #[must_use]
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }

    /// The upserted baseline file.
    #[must_use]
    pub fn baseline_path(&self) -> &Path {
        &self.baseline_path
    }

    /// The directory holding `report.json` and
    /// `candidate_bytes.evidence.json` for this transaction.
    #[must_use]
    pub fn evidence_dir(&self) -> &Path {
        &self.evidence_dir
    }
}

/// An explicit approval handle over one baseline root.
///
/// The handle never approves through frozen comparison: checking stays on
/// [`FrozenStore`], and only this type writes baselines. Roots that cannot
/// be written (read-only trees) fail at lock acquisition with a typed I/O
/// error; a set `CI` variable refuses every call before any write.
#[derive(Debug, Clone)]
pub struct ApprovalStore {
    baselines: PathBuf,
}

impl ApprovalStore {
    /// Open an approval handle over a baseline root.
    ///
    /// The root resolves through its nearest existing ancestor. It is not
    /// created here; a missing root is created during [`ApprovalStore::accept`].
    ///
    /// # Errors
    ///
    /// Returns [`ApproveError`] for an empty or unresolvable root.
    pub fn new(baseline_root: &Path) -> Result<Self, ApproveError> {
        let baselines = resolve_baseline_root(baseline_root)?;
        Ok(Self { baselines })
    }

    /// The canonical baseline root this handle approves into.
    #[must_use]
    pub fn baseline_root(&self) -> &Path {
        &self.baselines
    }

    /// Approve one candidate for one test ID.
    ///
    /// The candidate digest and the expected previous digest (a digest or
    /// the literal `missing`) are mandatory; missing, malformed, and zero
    /// digests are rejected. The candidate file is read-only input: it is
    /// read once and never written. The baseline root gains exactly the
    /// upserted baseline plus `report.json` and
    /// `candidate_bytes.evidence.json` under its reserved evidence directory;
    /// refusals (CI, stale digests, corrupt input, lock failures) write
    /// nothing anywhere.
    ///
    /// # Errors
    ///
    /// Returns [`ApproveError`] for CI refusal, digest rejections, stale
    /// candidates or baselines, invalid samples or paths, lock timeouts, and
    /// I/O failures.
    pub fn accept(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        // Step 1: refuse under CI before making the root or lock file.
        // This gate stays on the public path only: the transaction
        // mechanics below never read the environment, so unit tests
        // exercise them in both modes without a bypass.
        require_no_ci()?;
        self.accept_core(candidate_path, candidate_sha256, expected_baseline_sha256)
    }

    /// Run the approval transaction mechanics without the CI gate: steps
    /// 2-20 (digest validation, root creation, lock, read, arbitration,
    /// re-render, atomic upsert, evidence).
    ///
    /// Private: every production, integration-test, example, and CLI caller
    /// reaches approval through [`ApprovalStore::accept`], which refuses
    /// under `CI`. Only the unit tests below call this core directly, which
    /// keeps transaction coverage green under `CI` without unsetting it.
    ///
    /// # Errors
    ///
    /// Returns [`ApproveError`] for digest rejections, stale candidates or
    /// baselines, invalid samples or paths, lock timeouts, and I/O
    /// failures.
    fn accept_core(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        // Step 2: validate the digest arguments and store paths.
        require_digest("candidate", candidate_sha256, false)?;
        require_digest("expected baseline", expected_baseline_sha256, true)?;
        let baselines = resolve_baseline_root(&self.baselines)?;
        // Step 3: make the baseline root when missing.
        if !baselines.exists() {
            fs::create_dir_all(&baselines).map_err(|source| ApproveError::Io { source })?;
        }
        // Step 4: take the OS lock on the stable file, before any read.
        let lock =
            ApprovalFileLock::acquire(&baselines, Duration::from_secs(APPROVAL_LOCK_TIMEOUT_SECS))
                .map_err(|error| match error {
                    ApprovalLockError::Timeout => ApproveError::LockTimeout,
                    ApprovalLockError::Io(source) => ApproveError::Io { source },
                })?;
        // Steps 5-9 run under the lock; step 19 releases it on return.
        let transaction = Self::accept_locked(
            &baselines,
            candidate_path,
            candidate_sha256,
            expected_baseline_sha256,
        );
        drop(lock);
        transaction
    }

    /// Run the locked transaction: the single candidate read, digest
    /// arbitration, re-render, atomic upsert, and evidence writes.
    fn accept_locked(
        baselines: &Path,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        // Step 5: read the candidate bytes one time with the size limit.
        let candidate_bytes = read_candidate_bytes(candidate_path)?;
        // Step 6: compare their digest with the mandatory candidate digest.
        let actual_candidate = sha256_hex(&candidate_bytes);
        if actual_candidate != candidate_sha256 {
            return Err(ApproveError::StaleCandidate {
                expected: candidate_sha256.to_string(),
                actual: actual_candidate,
            });
        }
        // Step 7: validate the candidate schema and snapshot.
        let candidate = Sample::from_bytes(&candidate_bytes)?;
        // Steps 8-9: read the baseline under the same lock and arbitrate.
        let relative = baseline_relative(&candidate)?;
        ensure_no_symlink(baselines, &relative)?;
        let baseline_bytes = read_baseline_bytes(baselines, &relative)?;
        let previous_digest = baseline_bytes
            .as_deref()
            .map_or_else(|| MISSING_BASELINE.to_string(), sha256_hex);
        if previous_digest != expected_baseline {
            return Err(ApproveError::StaleBaseline {
                expected: expected_baseline.to_string(),
                actual: previous_digest,
            });
        }
        // First approval only: validate the candidate test ID. A present
        // baseline skips ID and gate cross-comparison entirely.
        if baseline_bytes.is_none() {
            validate_test_id(candidate.test_id())?;
        }
        // Steps 10-12: re-render image samples through the specified
        // profile and compare decoded pixels; state-only samples skip.
        rerender_check(&candidate)?;
        let changes = match baseline_bytes {
            None => ChangeSet::missing(),
            Some(bytes) => {
                let baseline = Sample::from_bytes(&bytes)?;
                let comparison = FrozenStore::compare_present(&candidate, &baseline)?;
                ChangeSet::from_present(comparison)
            }
        };
        // Steps 13-18: validate paths, make parents, atomically replace.
        let (baseline_path, evidence_dir) = write_baseline_and_evidence(
            baselines,
            &relative,
            &candidate,
            &candidate_bytes,
            &actual_candidate,
            &previous_digest,
            &changes,
        )?;
        // Step 20: return the approved test ID and sample digest.
        Ok(AcceptOutcome {
            test_id: candidate.test_id().to_string(),
            gate: candidate.gate(),
            candidate_sha256: actual_candidate,
            previous_baseline_sha256: previous_digest,
            baseline_sha256: candidate_sha256.to_string(),
            baseline_path,
            evidence_dir,
        })
    }
}

/// Derive the store-relative baseline path for a candidate test ID.
fn baseline_relative(candidate: &Sample) -> Result<PathBuf, ApproveError> {
    let mut relative = PathBuf::from(candidate.test_id());
    relative.set_extension("tui.json");
    if relative.parent().is_none() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
        });
    }
    if relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_none()
    {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
        });
    }
    Ok(relative)
}

/// Read the baseline file under the held lock through the shared bounded
/// read: `None` when absent, a resource limit past the size bound, an I/O
/// error for any other read failure. Bytes past the limit never load, so a
/// hostile baseline cannot exhaust memory before validation runs.
fn read_baseline_bytes(baselines: &Path, relative: &Path) -> Result<Option<Vec<u8>>, ApproveError> {
    read_bounded_optional(&baselines.join(relative)).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io { source },
        other => ApproveError::from(other),
    })
}

/// Steps 13-18 plus evidence: validate paths, make parent directories,
/// atomically replace the baseline, and save the approval records.
fn write_baseline_and_evidence(
    baselines: &Path,
    relative: &Path,
    candidate: &Sample,
    candidate_bytes: &[u8],
    candidate_digest: &str,
    previous_digest: &str,
    changes: &ChangeSet,
) -> Result<(PathBuf, PathBuf), ApproveError> {
    let parent = relative
        .parent()
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no parent directory".to_string(),
        })?;
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ApproveError::InvalidStorePath {
            detail: "baseline path has no file name".to_string(),
        })?;
    // Step 14: make the baseline's parent directories under the lock.
    let parent_dir = baselines.join(parent);
    fs::create_dir_all(&parent_dir).map_err(|source| ApproveError::Io { source })?;
    // Steps 15-18: write, flush, close, and atomically rename.
    let baseline_path = atomic_replace(&parent_dir, file_name, candidate_bytes)
        .map_err(|source| ApproveError::Io { source })?;
    let evidence_dir = write_accept_evidence(
        baselines,
        candidate,
        candidate_bytes.len(),
        candidate_digest,
        previous_digest,
        changes,
    )?;
    Ok((baseline_path, evidence_dir))
}

/// Steps 10-12: for an image sample, resolve the candidate's specified
/// render profile, re-render the snapshot through the upstream renderer,
/// and compare the new decoded pixels with the candidate pixels.
///
/// State-only samples carry no image and skip all three steps. An unknown
/// profile rejects with [`ApproveError::UnknownProfile`]; a renderer
/// refusal rejects with [`ApproveError::RenderFailed`] (or
/// [`ApproveError::MissingGlyph`]); differing pixels reject with
/// [`ApproveError::InconsistentSample`].
fn rerender_check(candidate: &Sample) -> Result<(), ApproveError> {
    let Some(image) = candidate.image() else {
        return Ok(());
    };
    let contract = candidate
        .render_contract()
        .ok_or_else(|| ApproveError::Check {
            source: CheckError::Sample(SampleError::GateMismatch {
                detail: "state-and-pixels gate requires a render contract".to_string(),
            }),
        })?;
    let profile = resolve_profile(contract.profile()).map_err(|error| match error {
        ProfileError::UnknownProfile { requested } => ApproveError::UnknownProfile { requested },
        other => ApproveError::RenderFailed {
            source: RenderError::RasterFailed {
                message: format!("render profile does not build: {other}"),
            },
        },
    })?;
    let rendered = match render_snapshot_png(candidate.snapshot(), &profile.render_options()) {
        Ok(rendered) => rendered,
        Err(RenderError::PinnedFont(source @ PinnedFontError::MissingGlyph { .. })) => {
            return Err(ApproveError::MissingGlyph { source });
        }
        Err(other) => return Err(ApproveError::RenderFailed { source: other }),
    };
    let fresh = decode_rendered_png(rendered.png())?;
    let stored = decode_stored_png(image)?;
    if fresh != stored {
        return Err(ApproveError::InconsistentSample {
            detail: format!(
                "re-rendered {}x{} pixels differ from the candidate {}x{} pixels",
                fresh.width, fresh.height, stored.width, stored.height,
            ),
        });
    }
    Ok(())
}

/// Decoded PNG bytes with dimensions for pixel comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedPng {
    /// Decoded image width in pixels.
    width: u32,
    /// Decoded image height in pixels.
    height: u32,
    /// Row-major 8-bit RGBA pixels.
    pixels: Vec<u8>,
}

/// Decode freshly rendered PNG bytes: failures are render failures, since
/// the upstream renderer just encoded these bytes.
fn decode_rendered_png(png: &[u8]) -> Result<DecodedPng, ApproveError> {
    decode_png(png).map_err(|detail| ApproveError::RenderFailed {
        source: RenderError::RasterFailed {
            message: format!("re-rendered PNG does not decode: {detail}"),
        },
    })
}

/// Decode the stored candidate PNG bytes: failures are image errors. These
/// bytes already passed [`Sample::from_bytes`] validation, so failure here
/// is unreachable outside memory corruption.
fn decode_stored_png(image: &SampleImage) -> Result<DecodedPng, ApproveError> {
    use base64::Engine as _;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(image.png_base64().as_bytes())
        .map_err(|source| ApproveError::Check {
            source: CheckError::Sample(SampleError::Base64Invalid { source }),
        })?;
    if sha256_hex(&raw) != image.png_sha256() {
        return Err(ApproveError::Check {
            source: CheckError::Sample(SampleError::DigestMismatch {
                field: "png_sha256",
                expected: image.png_sha256().to_string(),
                actual: sha256_hex(&raw),
            }),
        });
    }
    decode_png(&raw).map_err(|detail| ApproveError::Check {
        source: CheckError::Sample(SampleError::PngInvalid { detail }),
    })
}

/// Decode 8-bit RGBA PNG bytes with their dimensions.
fn decode_png(png: &[u8]) -> Result<DecodedPng, String> {
    use image::ImageDecoder as _;
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png))
        .map_err(|error| format!("PNG does not decode: {error}"))?;
    if decoder.color_type() != image::ColorType::Rgba8 {
        return Err(format!(
            "PNG color type is {:?}; only 8-bit RGBA is supported",
            decoder.color_type()
        ));
    }
    let (width, height) = decoder.dimensions();
    let total = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4));
    let Some(total) = total.and_then(|bytes| usize::try_from(bytes).ok()) else {
        return Err("PNG dimensions overflow the pixel buffer".to_string());
    };
    let mut pixels = vec![0u8; total];
    decoder
        .read_image(&mut pixels)
        .map_err(|error| format!("PNG pixels do not read: {error}"))?;
    Ok(DecodedPng {
        width,
        height,
        pixels,
    })
}

/// Save `report.json` and `candidate_bytes.evidence.json` for one
/// transaction under the reserved evidence directory.
fn write_accept_evidence(
    baselines: &Path,
    candidate: &Sample,
    candidate_len: usize,
    candidate_digest: &str,
    previous_digest: &str,
    changes: &ChangeSet,
) -> Result<PathBuf, ApproveError> {
    let evidence_relative: PathBuf = [ACCEPT_EVIDENCE_DIR, candidate.test_id(), candidate_digest]
        .iter()
        .collect();
    ensure_no_symlink(baselines, &evidence_relative)?;
    let evidence_dir = baselines.join(&evidence_relative);
    fs::create_dir_all(&evidence_dir).map_err(|source| ApproveError::Io { source })?;
    let record = AcceptRecord {
        schema: AcceptSchema,
        test_id: candidate.test_id(),
        gate: candidate.gate(),
        reason: changes.reason,
        candidate_sha256: candidate_digest,
        previous_baseline_sha256: previous_digest,
        baseline_sha256: candidate_digest,
        changed_fields: &changes.fields,
        changed_cells: &changes.cells,
        changed_cell_total: changes.total,
        pixel_changed_pixels: changes.pixel_changed,
        pixel_compared_pixels: changes.pixel_compared,
        pixel_bounds: changes.bounds.map(|bounds| BoundsRecord {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        }),
    };
    let record_bytes = serde_json::to_vec(&record).map_err(|error| ApproveError::Check {
        source: CheckError::EvidenceCorrupt {
            detail: format!("approval record does not serialize: {error}"),
        },
    })?;
    atomic_replace(&evidence_dir, "report.json", &record_bytes)
        .map_err(|source| ApproveError::Io { source })?;
    let image = candidate.image();
    let evidence = CandidateEvidence {
        schema: EvidenceSchema,
        test_id: candidate.test_id(),
        gate: candidate.gate(),
        candidate_sha256: candidate_digest,
        byte_length: candidate_len,
        png_sha256: image.map(SampleImage::png_sha256),
        rgba_sha256: image.map(SampleImage::rgba_sha256),
    };
    let evidence_bytes = serde_json::to_vec(&evidence).map_err(|error| ApproveError::Check {
        source: CheckError::EvidenceCorrupt {
            detail: format!("candidate evidence does not serialize: {error}"),
        },
    })?;
    atomic_replace(
        &evidence_dir,
        "candidate_bytes.evidence.json",
        &evidence_bytes,
    )
    .map_err(|source| ApproveError::Io { source })?;
    Ok(evidence_dir)
}

/// True when the `CI` variable is present with any value, including empty.
///
/// Any presence freezes approval: there is no override, and ambient
/// `BLESS`, `UPDATE`, and `INSTA_UPDATE` variables are ignored outright
/// (they are never read).
fn ci_is_set() -> bool {
    std::env::var_os("CI").is_some()
}

/// Refuse approval while the `CI` variable is set, before any write.
fn require_no_ci() -> Result<(), ApproveError> {
    if ci_is_set() {
        return Err(ApproveError::CiApprovalForbidden);
    }
    Ok(())
}

/// Validate one mandatory digest argument: missing, malformed, and zero
/// placeholder digests are rejected as invalid sample-selection input.
/// Only the expected baseline digest may carry the literal `missing`.
fn require_digest(role: &str, value: &str, allow_missing: bool) -> Result<(), ApproveError> {
    let invalid = |detail: String| ApproveError::InvalidArgument { detail };
    if value.is_empty() {
        return Err(invalid(format!("{role} digest is missing")));
    }
    if allow_missing && value == MISSING_BASELINE {
        return Ok(());
    }
    if !is_hex_digest(value) {
        return Err(invalid(format!(
            "{role} digest {value:?} is not a 64-character lowercase hex digest"
        )));
    }
    if value.bytes().all(|byte| byte == b'0') {
        return Err(invalid(format!(
            "{role} digest must not be a zero placeholder"
        )));
    }
    Ok(())
}

/// True for 64 lowercase hex characters: a SHA-256 digest shape.
///
/// This mirrors the frozen comparison helper so approval validates digest
/// arguments without depending on comparison internals.
fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// SHA-256 hex over raw bytes: the candidate and baseline keying.
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

/// Read the candidate file once through the shared bounded read: the file
/// is read-only input, and bytes past the sample limit fail as a resource
/// limit before validation runs. An unreadable candidate names its path in
/// the error.
fn read_candidate_bytes(path: &Path) -> Result<Vec<u8>, ApproveError> {
    read_bounded(path).map_err(|error| match error {
        SampleError::Io { source } => ApproveError::Io {
            source: std::io::Error::new(
                source.kind(),
                format!("cannot read candidate {}: {source}", path.display()),
            ),
        },
        other => ApproveError::from(other),
    })
}

/// Resolve a baseline root: canonicalize it, or its nearest existing
/// ancestor with the missing remainder appended lexically. A `..` remainder
/// is rejected rather than resolved.
fn resolve_baseline_root(path: &Path) -> Result<PathBuf, ApproveError> {
    if path.as_os_str().is_empty() {
        return Err(ApproveError::InvalidStorePath {
            detail: "baseline root path is empty".to_string(),
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
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} has no resolvable ancestor",
                    path.display()
                ),
            });
        };
        if name == ".." {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "baseline root {} carries an unresolvable `..`",
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
/// link can never redirect a baseline or evidence write outside its root.
fn ensure_no_symlink(root: &Path, relative: &Path) -> Result<(), ApproveError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(ApproveError::Io { source }),
        };
        if meta.file_type().is_symlink() {
            return Err(ApproveError::InvalidStorePath {
                detail: format!(
                    "store-relative path {} crosses a symlink",
                    relative.display()
                ),
            });
        }
    }
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
    ///
    /// Exclusive creation with owner-only mode: the call never follows a
    /// symlink (a planted link at the fresh name fails creation), and the
    /// file is never truncated or reused.
    fn create(dir: &Path, name: &str) -> Result<(fs::File, Self), std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = dir.join(format!(".tmp-accept-{}-{id}-{name}", std::process::id()));
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

    /// Flush, close, and atomically rename the temporary file.
    ///
    /// Both paths share one directory, so the rename cannot cross devices:
    /// there is no copy fallback (a copy would not be atomic), and any
    /// rename failure removes the temporary file and reports I/O.
    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary approval file is already committed",
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

/// Write bytes through a temporary file with flush, close, and atomic
/// rename. The temporary file lives in the target directory, so the rename
/// is always same-directory and can never report `CrossesDevices`; no copy
/// fallback exists. Readers never see a partial file.
fn atomic_replace(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, std::io::Error> {
    if name.contains('/') || name.contains('\\') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("evidence file name {name:?} must not carry separators"),
        ));
    }
    let target = dir.join(name);
    let (mut file, temp) = TempFile::create(dir, name)?;
    file.write_all(bytes)?;
    file.flush()?;
    drop(file);
    temp.commit(&target)?;
    Ok(target)
}

/// The change set between the candidate and the previous baseline, in
/// comparison order: the smallest difference the frozen comparison reports.
struct ChangeSet {
    reason: Option<MismatchReason>,
    fields: Vec<String>,
    cells: Vec<CellChange>,
    total: usize,
    pixel_changed: u64,
    pixel_compared: u64,
    bounds: Option<[u32; 4]>,
}

/// One changed cell in an approval record: coordinates plus field names.
#[derive(serde::Serialize)]
struct CellChange {
    col: u32,
    row: u32,
    fields: Vec<String>,
}

/// One pixel-bounds record in an approval record.
#[derive(serde::Serialize)]
struct BoundsRecord {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl ChangeSet {
    /// An empty change set: the candidate already equals the baseline.
    fn empty() -> Self {
        Self {
            reason: None,
            fields: Vec::new(),
            cells: Vec::new(),
            total: 0,
            pixel_changed: 0,
            pixel_compared: 0,
            bounds: None,
        }
    }

    /// The change set for a first approval: no baseline to compare, so
    /// the single changed field is the missing baseline itself.
    fn missing() -> Self {
        Self {
            reason: Some(MismatchReason::Missing),
            fields: vec!["baseline".to_string()],
            cells: Vec::new(),
            total: 0,
            pixel_changed: 0,
            pixel_compared: 0,
            bounds: None,
        }
    }

    /// The change set from a present-baseline comparison, verbatim. The
    /// comparison never ran test-ID or gate cross-comparison; a pass is an
    /// empty change set.
    fn from_present(comparison: PresentComparison) -> Self {
        match comparison {
            PresentComparison::Pass => Self::empty(),
            PresentComparison::Failure {
                reason,
                changed_fields,
                changed_cells,
                changed_cell_total,
                pixel_changed,
                pixel_compared,
                pixel_bounds,
            } => Self {
                reason: Some(reason),
                fields: changed_fields,
                cells: changed_cells
                    .iter()
                    .map(|cell| CellChange {
                        col: cell.col(),
                        row: cell.row(),
                        fields: cell.fields().to_vec(),
                    })
                    .collect(),
                total: changed_cell_total,
                pixel_changed,
                pixel_compared,
                bounds: pixel_bounds,
            },
        }
    }
}

/// The schema identifier as an explicit type: only [`ACCEPT_SCHEMA`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AcceptSchema;

impl serde::Serialize for AcceptSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(ACCEPT_SCHEMA)
    }
}

/// The schema identifier as an explicit type: only [`EVIDENCE_SCHEMA`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EvidenceSchema;

impl serde::Serialize for EvidenceSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(EVIDENCE_SCHEMA)
    }
}

/// The versioned approval record saved as `report.json`.
///
/// Field order is fixed. The record carries no absolute paths, process IDs,
/// timestamps, host names, or session names: only the test ID, gate,
/// digests, and the compared change set.
#[derive(serde::Serialize)]
struct AcceptRecord<'a> {
    schema: AcceptSchema,
    test_id: &'a str,
    gate: Gate,
    reason: Option<MismatchReason>,
    candidate_sha256: &'a str,
    previous_baseline_sha256: &'a str,
    baseline_sha256: &'a str,
    changed_fields: &'a [String],
    changed_cells: &'a [CellChange],
    changed_cell_total: usize,
    pixel_changed_pixels: u64,
    pixel_compared_pixels: u64,
    pixel_bounds: Option<BoundsRecord>,
}

/// The versioned candidate evidence saved as `candidate_bytes.evidence.json`.
///
/// It binds the approved byte length and the recorded image digests to the
/// candidate digest, with no volatile data.
#[derive(serde::Serialize)]
struct CandidateEvidence<'a> {
    schema: EvidenceSchema,
    test_id: &'a str,
    gate: Gate,
    candidate_sha256: &'a str,
    byte_length: usize,
    png_sha256: Option<&'a str>,
    rgba_sha256: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tui_test::profile::Rgb;
    use tui_test::terminal::cell::Attrs;
    use tui_test::{
        SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
        SnapshotParts, SnapshotScope, TerminalSnapshot,
    };

    use super::{
        ApprovalStore, atomic_replace, is_hex_digest, require_digest, resolve_baseline_root,
        sample_code, sha256_hex,
    };
    use crate::sample::{Observation, Sample, SampleError};
    use crate::view::ViewSettings;

    #[test]
    fn sha256_matches_the_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn digest_shape_rejects_non_digests() {
        assert!(is_hex_digest(&"a".repeat(64)));
        assert!(is_hex_digest(&"0".repeat(64)));
        for bad in ["", "abc", &"a".repeat(63), &"a".repeat(65), &"A".repeat(64)] {
            assert!(!is_hex_digest(bad), "{bad:?} is not a digest shape");
        }
    }

    #[test]
    fn digest_arguments_reject_missing_malformed_and_zero() {
        assert!(require_digest("candidate", &"a".repeat(64), false).is_ok());
        assert!(require_digest("expected baseline", "missing", true).is_ok());
        for bad in ["", "abc", &"A".repeat(64), &"0".repeat(64)] {
            match require_digest("candidate", bad, false) {
                Err(error @ super::ApproveError::InvalidArgument { .. }) => {
                    assert_eq!(error.code(), "INVALID_SAMPLE");
                    assert!(error.observed_baseline().is_none());
                }
                other => panic!("{bad:?} must fail as an invalid argument, got {other:?}"),
            }
        }
        assert!(
            require_digest("expected baseline", "", true).is_err(),
            "an empty expected digest must fail"
        );
        assert!(
            require_digest("expected baseline", "missing", false).is_err(),
            "`missing` needs its own argument slot"
        );
    }

    #[test]
    fn sample_errors_keep_their_section_16_3_codes() {
        let invalid_sample = [
            SampleError::InvalidTestId {
                detail: "bad".to_string(),
            },
            SampleError::GateMismatch {
                detail: "bad".to_string(),
            },
            SampleError::UnknownField {
                detail: "bad".to_string(),
            },
        ];
        for error in &invalid_sample {
            assert_eq!(sample_code(error), "INVALID_SAMPLE", "{error:?}");
        }
        let resource_limit = [
            SampleError::TooLarge {
                what: "sample file",
                bytes: 1,
                limit: 0,
            },
            SampleError::DimensionTooLarge {
                width: 1,
                height: 1,
                limit: 0,
            },
            SampleError::RgbaTooLarge { bytes: 1, limit: 0 },
        ];
        for error in &resource_limit {
            assert_eq!(sample_code(error), "RESOURCE_LIMIT", "{error:?}");
        }
        assert_eq!(
            sample_code(&SampleError::UnsupportedSchema {
                version: "tuiscotti.sample.v2".to_string(),
            }),
            "UNSUPPORTED_SCHEMA"
        );
    }

    #[test]
    fn empty_roots_fail_fast() {
        let outcome = resolve_baseline_root(std::path::Path::new(""));
        assert!(
            matches!(outcome, Err(super::ApproveError::InvalidStorePath { .. })),
            "an empty root fails before any write"
        );
    }

    #[test]
    fn atomic_replace_writes_without_temp_residue() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let target = atomic_replace(dir.path(), "report.json", b"{\"ok\":true}")
            .expect("atomic write succeeds");
        assert_eq!(
            std::fs::read(&target).expect("written bytes read"),
            b"{\"ok\":true}"
        );
        for entry in std::fs::read_dir(dir.path()).expect("directory lists") {
            let name = entry
                .expect("entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned();
            assert!(
                !name.starts_with(".tmp-accept-"),
                "temporary file {name:?} left behind"
            );
        }
    }

    /// Fixed 4x2 snapshot fixture: one grapheme per cell, hidden cursor.
    fn unit_snapshot() -> Result<TerminalSnapshot, String> {
        let cells: Vec<SnapshotCell> = (0..8)
            .map(|_| {
                SnapshotCell::new(
                    "a".to_string(),
                    1,
                    SnapshotColor::Default,
                    SnapshotColor::Default,
                    Attrs::empty(),
                    tui_test::SnapshotUnderline::None,
                    SnapshotColor::Default,
                    None,
                    None,
                )
                .map_err(|error| format!("fixture cell: {error:?}"))
            })
            .collect::<Result<_, _>>()?;
        TerminalSnapshot::try_new(SnapshotParts {
            cols: 4,
            rows: 2,
            scope: SnapshotScope::Viewport,
            viewport_origin: 0,
            cells,
            palette: *ViewSettings::builtin().palette(),
            default_foreground: Rgb::new(192, 192, 192),
            default_background: Rgb::new(0, 0, 0),
            default_cursor: Rgb::new(192, 192, 192),
            cursor: SnapshotCursor::new(
                0,
                0,
                true,
                false,
                SnapshotCursorShape::Block,
                Rgb::new(192, 192, 192),
                None,
            ),
            title: None,
            metadata: SnapshotMetadata::default(),
        })
        .map_err(|error| format!("fixture snapshot: {error:?}"))
    }

    /// State-only sample bytes for a test ID: no render contract, no
    /// image, so the core covers steps 5-9 and 13-20 without fonts.
    fn unit_state_bytes(test_id: &str) -> Result<Vec<u8>, String> {
        let snapshot = unit_snapshot()?;
        Sample::state(test_id, snapshot, Observation::empty())
            .map_err(|error| format!("fixture sample: {error:?}"))?
            .to_bytes()
            .map_err(|error| format!("fixture bytes: {error:?}"))
    }

    /// Write candidate bytes to a file and return its path.
    fn unit_candidate_file(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
        let path = dir.join(name);
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    /// Store-relative baseline path for a test ID, without touching disk.
    fn unit_baseline_path(root: &Path, test_id: &str) -> PathBuf {
        let mut path = root.join(test_id);
        path.set_extension("tui.json");
        path
    }

    /// Write baseline bytes for a test ID, making parent directories.
    fn unit_write_baseline(root: &Path, test_id: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
        let path = unit_baseline_path(root, test_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    /// The core approves a first state-only sample: baseline, evidence,
    /// and outcome digests, with the lock file kept.
    #[test]
    fn core_first_approval_writes_baseline_and_evidence() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let baselines = dir.path().join("baselines");
        let bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        let digest = sha256_hex(&bytes);
        let candidate = unit_candidate_file(dir.path(), "candidate.tui.json", &bytes)
            .expect("candidate writes");
        let store = ApprovalStore::new(&baselines).expect("approval store opens");
        let accepted = store
            .accept_core(&candidate, &digest, "missing")
            .expect("core approves");
        assert_eq!(accepted.test_id(), "views/login");
        assert_eq!(accepted.candidate_sha256(), digest);
        assert_eq!(accepted.previous_baseline_sha256(), "missing");
        assert_eq!(accepted.baseline_sha256(), digest);
        assert_eq!(
            std::fs::read(unit_baseline_path(&baselines, "views/login")).expect("baseline reads"),
            bytes
        );
        assert!(accepted.baseline_path().is_file());
        assert!(accepted.evidence_dir().join("report.json").is_file());
        assert!(
            accepted
                .evidence_dir()
                .join("candidate_bytes.evidence.json")
                .is_file()
        );
    }

    /// The core records the previous digest on a second approval of the
    /// same bytes.
    #[test]
    fn core_second_approval_records_previous_digest() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let baselines = dir.path().join("baselines");
        let bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        let digest = sha256_hex(&bytes);
        let candidate = unit_candidate_file(dir.path(), "candidate.tui.json", &bytes)
            .expect("candidate writes");
        let store = ApprovalStore::new(&baselines).expect("approval store opens");
        let first = store
            .accept_core(&candidate, &digest, "missing")
            .expect("first approval runs");
        assert_eq!(first.previous_baseline_sha256(), "missing");
        let second = store
            .accept_core(&candidate, &digest, &digest)
            .expect("second approval runs");
        assert_eq!(second.test_id(), first.test_id());
        assert_eq!(second.baseline_sha256(), digest);
        assert_eq!(second.previous_baseline_sha256(), digest);
    }

    /// The core rejects a wrong candidate digest before any baseline
    /// write.
    #[test]
    fn core_stale_candidate_rejected_without_baseline() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let baselines = dir.path().join("baselines");
        let bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        let digest = sha256_hex(&bytes);
        let candidate = unit_candidate_file(dir.path(), "candidate.tui.json", &bytes)
            .expect("candidate writes");
        let store = ApprovalStore::new(&baselines).expect("approval store opens");
        let wrong = "d".repeat(64);
        match store.accept_core(&candidate, &wrong, "missing") {
            Err(super::ApproveError::StaleCandidate { expected, actual }) => {
                assert_eq!(expected, wrong);
                assert_eq!(actual, digest);
            }
            other => panic!("a wrong candidate digest must fail, got {other:?}"),
        }
        assert!(
            !unit_baseline_path(&baselines, "views/login").exists(),
            "stale candidates write no baseline"
        );
    }

    /// The core rejects a stale expected digest with the planted baseline
    /// left untouched.
    #[test]
    fn core_stale_baseline_leaves_planted_bytes() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let baselines = dir.path().join("baselines");
        std::fs::create_dir_all(&baselines).expect("baseline root creates");
        let first_bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        unit_write_baseline(&baselines, "views/login", &first_bytes).expect("baseline plants");
        let second_bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        let second_digest = sha256_hex(&second_bytes);
        let second = unit_candidate_file(dir.path(), "second.tui.json", &second_bytes)
            .expect("candidate writes");
        let store = ApprovalStore::new(&baselines).expect("approval store opens");
        match store.accept_core(&second, &second_digest, &"c".repeat(64)) {
            Err(super::ApproveError::StaleBaseline { .. }) => {}
            other => panic!("a stale baseline must fail, got {other:?}"),
        }
        assert_eq!(
            std::fs::read(unit_baseline_path(&baselines, "views/login")).expect("baseline reads"),
            first_bytes
        );
    }

    /// The core fails a lock path blocked by a directory with a typed I/O
    /// error and writes nothing.
    #[test]
    fn core_blocked_lock_path_fails_without_writes() {
        let dir = tempfile::tempdir().expect("scratch directory creates");
        let baselines = dir.path().join("baselines");
        std::fs::create_dir_all(&baselines).expect("baseline root creates");
        std::fs::create_dir(baselines.join(".tuiscotti-approval.lock"))
            .expect("lock blocker creates");
        let bytes = unit_state_bytes("views/login").expect("fixture sample builds");
        let digest = sha256_hex(&bytes);
        let candidate = unit_candidate_file(dir.path(), "candidate.tui.json", &bytes)
            .expect("candidate writes");
        let store = ApprovalStore::new(&baselines).expect("approval store opens");
        match store.accept_core(&candidate, &digest, "missing") {
            Err(error @ super::ApproveError::Io { .. }) => {
                assert_eq!(error.code(), "IO");
                assert_eq!(error.exit_code(), 3);
            }
            other => panic!("a blocked lock path must fail I/O, got {other:?}"),
        }
        assert!(
            !unit_baseline_path(&baselines, "views/login").exists(),
            "lock faults write no baseline"
        );
    }

    /// The CI gate coheres with the environment: refusal happens exactly
    /// when `CI` is present, with the code-3 error. Per-value refusal
    /// (including empty values) and the no-write guarantee run in
    /// dedicated child processes in `tests/approval.rs`, which set the
    /// child environment without touching this process.
    #[test]
    fn ci_gate_matches_environment_presence() {
        let present = std::env::var_os("CI").is_some();
        assert_eq!(super::ci_is_set(), present);
        match super::require_no_ci() {
            Ok(()) => assert!(!present, "an absent CI variable opens the gate"),
            Err(error @ super::ApproveError::CiApprovalForbidden) => {
                assert!(present, "a present CI variable refuses the gate");
                assert_eq!(error.code(), "CI_APPROVAL_FORBIDDEN");
                assert_eq!(error.exit_code(), 3);
            }
            Err(other) => panic!("the CI gate reports only refusal, got {other:?}"),
        }
    }
}
