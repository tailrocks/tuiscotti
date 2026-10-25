//! Baseline approval with digests and frozen checking (D04, GOAL Section 15).
//!
//! [`ApprovalStore::accept`] runs one explicit approval transaction for one
//! test ID: it validates the candidate bytes and both digests first, checks
//! the candidate against the present baseline under a per-root lock, then
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
//! - It never calls the upstream renderer during approval: a historical or
//!   foreign render contract that passes structural validation is accepted
//!   when its image bytes validate, and approval never resolves font
//!   profiles. Unknown profiles therefore do not fail approval.
//! - It never compares the test ID inside the candidate with the test ID
//!   claimed inside the baseline. The operation targets the candidate test
//!   ID; the baseline path derives from that ID, and only the grammar of the
//!   candidate ID is validated.
//! - It computes the change set for `report.json` with the same comparison
//!   used by [`FrozenStore::check`] (state first, then render contract, then
//!   decoded RGBA pixels through upstream `compare_rgba`), over the stored
//!   bytes only. The smallest change set wins; no caller-supplied flags steer
//!   it.
//! - It reads the baseline bytes exactly once per transaction: the inner
//!   [`FrozenStore::check`] run is the single baseline read. The candidate
//!   bytes are read once and reused by digest, validation, comparison, and
//!   upsert, so the candidate reference stays consistent between check and
//!   accept.
//!
//! Transaction order:
//!
//! 1. Refuse when the `CI` variable is set, before any write.
//! 2. Validate the digest arguments (missing, malformed, and zero digests
//!    are rejected).
//! 3. Read the candidate bytes once with the file-size limit.
//! 4. Compare their digest with the mandatory candidate digest.
//! 5. Validate the candidate schema, test ID grammar, and snapshot.
//! 6. Resolve the baseline root, making it when missing.
//! 7. Acquire the per-root approval lock (fresh exclusive file, 30 seconds).
//! 8. Run [`FrozenStore::check`] on the candidate bytes with a single secure
//!    scratch directory as its artifact root: the one baseline read, and the
//!    change set for the evidence.
//! 9. Compare the observed previous digest with the expected digest.
//! 10. Atomically upsert `baselines/<test_id>.tui.json` with the candidate
//!     bytes unchanged (temporary file plus rename, with a copy fallback for
//!     cross-device mounts).
//! 11. Save `report.json` and `candidate_bytes.evidence.json` beside it.
//! 12. Release the lock and return the digests.
//!
//! Locking: one lock per baseline root, held from the comparison through the
//! replacement. The lock file `<root>/.tuiscotti-approval.lock` is created
//! fresh by each holder with exclusive creation and removed on release, so a
//! completed transaction leaves no lock file behind. Locks are not portable:
//! no cross-process before-or-after file semantics follow from the lock file,
//! and a process that dies mid-transaction can leave the empty file behind
//! for an operator to remove. The wait stops after 30 seconds with a typed
//! timeout.
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
//! [`ApproveError::code`] and the process codes from
//! [`ApproveError::exit_code`] are the mapping inputs for that command.
//!
//! Design notes: approval reuses the frozen comparison instead of
//! duplicating its differ, so check and accept cannot disagree on the change
//! set. Small byte-level helpers (digest shape, root resolution, symlink
//! guard, SHA-256) mirror `check.rs` without sharing code, because the task
//! packet restricts `check.rs` to lock additions only.

use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::check::{
    APPROVAL_LOCK_TIMEOUT_SECS, ApprovalFileLock, ApprovalLockError, CheckError, CheckOutcome,
    FrozenStore, MISSING_BASELINE, MismatchReason, validate_test_id,
};
use crate::sample::{Gate, MAX_SAMPLE_FILE_BYTES, Sample, SampleError, SampleImage};

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
    InvalidDigest {
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
            Self::InvalidDigest { detail } => write!(formatter, "invalid digest: {detail}"),
            Self::StaleCandidate { expected, actual } => write!(
                formatter,
                "candidate changed: expected digest {expected}, read bytes digest {actual}"
            ),
            Self::StaleBaseline { expected, actual } => write!(
                formatter,
                "baseline changed: expected previous digest {expected}, present digest {actual}"
            ),
            Self::LockTimeout => write!(formatter, "approval lock deadline passed"),
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
    /// `INVALID_DIGEST` covers malformed digest arguments; every stale,
    /// frozen, lock, sample, image, or filesystem failure maps to its
    /// Section 16.3 code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::CiApprovalForbidden => "CI_APPROVAL_FORBIDDEN",
            Self::InvalidDigest { .. } => "INVALID_DIGEST",
            Self::StaleCandidate { .. } => "STALE_CANDIDATE",
            Self::StaleBaseline { .. } => "STALE_BASELINE",
            Self::LockTimeout => "LOCK_TIMEOUT",
            Self::InvalidStorePath { .. } => "INVALID_STORE_PATH",
            Self::Check { source } => match source {
                CheckError::Sample(_) => "INVALID_SAMPLE",
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

    /// The process exit code for this failure: `2` for malformed digest
    /// arguments, `3` for stale, frozen, lock, sample, image, and I/O
    /// failures.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidDigest { .. } => 2,
            _ => 3,
        }
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
        require_no_ci()?;
        require_digest("candidate", candidate_sha256, false)?;
        require_digest("expected baseline", expected_baseline_sha256, true)?;
        let candidate_bytes = read_candidate_bytes(candidate_path)?;
        let actual_candidate = sha256_hex(&candidate_bytes);
        if actual_candidate != candidate_sha256 {
            return Err(ApproveError::StaleCandidate {
                expected: candidate_sha256.to_string(),
                actual: actual_candidate,
            });
        }
        let candidate = Sample::from_bytes(&candidate_bytes)?;
        validate_test_id(candidate.test_id())?;
        let baselines = resolve_baseline_root(&self.baselines)?;
        if !baselines.exists() {
            fs::create_dir_all(&baselines).map_err(|source| ApproveError::Io { source })?;
        }
        let scratch = AcceptTempDir::create().map_err(|source| ApproveError::Io { source })?;
        let frozen = FrozenStore::new(&baselines, scratch.path())?;
        let lock = ApprovalFileLock::acquire(
            frozen.baseline_root(),
            Duration::from_secs(APPROVAL_LOCK_TIMEOUT_SECS),
        )
        .map_err(|error| match error {
            ApprovalLockError::Timeout => ApproveError::LockTimeout,
            ApprovalLockError::Io(source) => ApproveError::Io { source },
        })?;
        let transaction = Self::accept_locked(
            frozen.baseline_root(),
            &candidate,
            &candidate_bytes,
            &actual_candidate,
            expected_baseline_sha256,
            &frozen,
        );
        drop(lock);
        transaction
    }

    /// Run the locked transaction: the single comparison, digest
    /// arbitration, atomic upsert, and evidence writes.
    fn accept_locked(
        baselines: &Path,
        candidate: &Sample,
        candidate_bytes: &[u8],
        candidate_digest: &str,
        expected_baseline: &str,
        frozen: &FrozenStore,
    ) -> Result<AcceptOutcome, ApproveError> {
        let outcome = frozen.check(candidate_bytes)?;
        let (previous_digest, changes) = match &outcome {
            CheckOutcome::Pass(success) => {
                (success.baseline_sha256().to_string(), ChangeSet::empty())
            }
            CheckOutcome::Failure(failure) => (
                failure.baseline_sha256().to_string(),
                ChangeSet::from_failure(failure),
            ),
        };
        if previous_digest != expected_baseline {
            return Err(ApproveError::StaleBaseline {
                expected: expected_baseline.to_string(),
                actual: previous_digest,
            });
        }
        let mut relative = PathBuf::from(candidate.test_id());
        relative.set_extension("tui.json");
        ensure_no_symlink(baselines, &relative)?;
        let parent = relative
            .parent()
            .ok_or_else(|| ApproveError::InvalidStorePath {
                detail: "baseline path has no parent directory".to_string(),
            })?;
        let parent_dir = baselines.join(parent);
        fs::create_dir_all(&parent_dir).map_err(|source| ApproveError::Io { source })?;
        let file_name = relative
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ApproveError::InvalidStorePath {
                detail: "baseline path has no file name".to_string(),
            })?;
        let baseline_path = atomic_replace(&parent_dir, file_name, candidate_bytes)
            .map_err(|source| ApproveError::Io { source })?;
        let evidence_dir = write_accept_evidence(
            baselines,
            candidate,
            candidate_bytes.len(),
            candidate_digest,
            &previous_digest,
            &changes,
        )?;
        Ok(AcceptOutcome {
            test_id: candidate.test_id().to_string(),
            gate: candidate.gate(),
            candidate_sha256: candidate_digest.to_string(),
            previous_baseline_sha256: previous_digest,
            baseline_sha256: candidate_digest.to_string(),
            baseline_path,
            evidence_dir,
        })
    }
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
/// placeholder digests are rejected. Only the expected baseline digest may
/// carry the literal `missing`.
fn require_digest(role: &str, value: &str, allow_missing: bool) -> Result<(), ApproveError> {
    let invalid = |detail: String| ApproveError::InvalidDigest { detail };
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

/// Read the candidate file once with a bounded read: the file is read-only
/// input, and bytes past the sample limit fail validation downstream. An
/// unreadable candidate names its path in the error.
fn read_candidate_bytes(path: &Path) -> Result<Vec<u8>, ApproveError> {
    let file = fs::File::open(path).map_err(|source| ApproveError::Io {
        source: std::io::Error::new(
            source.kind(),
            format!("cannot read candidate {}: {source}", path.display()),
        ),
    })?;
    let mut bounded = file.take(MAX_SAMPLE_FILE_BYTES + 1);
    let mut bytes = Vec::new();
    bounded
        .read_to_end(&mut bytes)
        .map_err(|source| ApproveError::Io { source })?;
    Ok(bytes)
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

/// One secure scratch directory for an approval transaction, removed on
/// drop. Creation never follows symlinks: an existing link at the fresh name
/// fails creation and the next name is tried.
struct AcceptTempDir {
    path: PathBuf,
}

impl AcceptTempDir {
    /// Create the single scratch directory for one transaction.
    fn create() -> Result<Self, std::io::Error> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        for _ in 0..64u32 {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = base.join(format!(".tmp-tuiscotti-accept-{}-{id}", std::process::id()));
            #[cfg(unix)]
            let created = {
                use std::os::unix::fs::DirBuilderExt as _;
                fs::DirBuilder::new().mode(0o700).create(&path)
            };
            #[cfg(not(unix))]
            let created = fs::create_dir(&path);
            match created {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::other(
            "too many approval scratch directory collisions",
        ))
    }

    /// The scratch directory holding the transaction's inner comparison.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for AcceptTempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.path));
    }
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
        let path = dir.join(format!(".tmp-accept-{}-{id}-{name}", std::process::id()));
        let file = fs::File::create(&path)?;
        Ok((file, Self { path: Some(path) }))
    }

    /// Flush, close, and atomically rename the temporary file.
    fn commit(mut self, target: &Path) -> Result<(), std::io::Error> {
        let Some(path) = self.path.take() else {
            return Err(std::io::Error::other(
                "temporary approval file is already committed",
            ));
        };
        match fs::rename(&path, target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {
                let _copied: u64 = fs::copy(&path, target)?;
                drop(fs::remove_file(&path));
                Ok(())
            }
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
/// rename. The rename targets the same directory, so it cannot cross
/// devices; a copy fallback still covers exotic mounts. Readers never see a
/// partial file.
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

    /// The change set from a failing frozen comparison, verbatim.
    fn from_failure(failure: &crate::check::CheckFailure) -> Self {
        Self {
            reason: Some(failure.reason()),
            fields: failure.changed_fields().to_vec(),
            cells: failure
                .changed_cells()
                .iter()
                .map(|cell| CellChange {
                    col: cell.col(),
                    row: cell.row(),
                    fields: cell.fields().to_vec(),
                })
                .collect(),
            total: failure.changed_cell_total(),
            pixel_changed: failure.pixel_changed(),
            pixel_compared: failure.pixel_compared(),
            bounds: failure.pixel_bounds(),
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
    use super::{atomic_replace, is_hex_digest, require_digest, resolve_baseline_root, sha256_hex};

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
            assert!(
                require_digest("candidate", bad, false).is_err(),
                "{bad:?} must fail"
            );
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
}
