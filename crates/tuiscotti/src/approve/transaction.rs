use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::check::{
    APPROVAL_LOCK_FILE, APPROVAL_LOCK_TIMEOUT_SECS, ApprovalFileLock, ApprovalLockError,
    FrozenStore, MISSING_BASELINE, validate_test_id,
};
use crate::sample::Sample;

use super::changes::ChangeSet;
use super::digest::{require_digest, require_no_ci, sha256_hex};
use super::errors::ApproveError;
use super::outcome::AcceptOutcome;
use super::paths::{
    baseline_relative, ensure_no_symlink, read_baseline_bytes, read_candidate_bytes,
    resolve_baseline_root, write_baseline,
};
use super::render::rerender_check;

#[derive(Debug, Clone)]
#[doc = "An explicit approval handle over one baseline root."]
pub struct ApprovalStore {
    #[doc = "The canonical baseline root this handle approves into."]
    baselines: PathBuf,
}

impl ApprovalStore {
    #[doc = "Open an approval handle over a baseline root.\n\n# Errors\n\nReturns [`ApproveError`] for an empty or unresolvable root."]
    pub fn new(baseline_root: &Path) -> Result<Self, ApproveError> {
        let baselines = resolve_baseline_root(baseline_root)?;
        Ok(Self { baselines })
    }

    #[must_use]
    #[doc = "The canonical baseline root this handle approves into."]
    pub fn baseline_root(&self) -> &Path {
        &self.baselines
    }

    #[doc = "Approve one candidate for one test ID.\n\n# Errors\n\nReturns [`ApproveError`] for `CI` refusal, digest rejections, stale candidates or baselines, invalid samples or paths, lock timeouts, and I/O failures."]
    pub fn accept(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        require_no_ci()?;
        self.accept_core(candidate_path, candidate_sha256, expected_baseline_sha256)
    }

    fn accept_core(
        &self,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline_sha256: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        require_digest("candidate", candidate_sha256, false)?;
        require_digest("expected baseline", expected_baseline_sha256, true)?;
        let baselines = resolve_baseline_root(&self.baselines)?;
        if !baselines.exists() {
            fs::create_dir_all(&baselines).map_err(|source| ApproveError::Io {
                source,
                observed_baseline: None,
            })?;
        }
        ensure_no_symlink(&baselines, Path::new(APPROVAL_LOCK_FILE))?;
        let lock =
            ApprovalFileLock::acquire(&baselines, Duration::from_secs(APPROVAL_LOCK_TIMEOUT_SECS))
                .map_err(|error| match error {
                    ApprovalLockError::Timeout => ApproveError::LockTimeout,
                    ApprovalLockError::Io(source) => ApproveError::Io {
                        source,
                        observed_baseline: None,
                    },
                })?;
        let transaction = Self::accept_locked(
            &baselines,
            candidate_path,
            candidate_sha256,
            expected_baseline_sha256,
        );
        drop(lock);
        transaction
    }

    fn accept_locked(
        baselines: &Path,
        candidate_path: &Path,
        candidate_sha256: &str,
        expected_baseline: &str,
    ) -> Result<AcceptOutcome, ApproveError> {
        let candidate_bytes = read_candidate_bytes(candidate_path)?;
        let actual_candidate = sha256_hex(&candidate_bytes);
        if actual_candidate != candidate_sha256 {
            return Err(ApproveError::StaleCandidate {
                expected: candidate_sha256.to_string(),
                actual: actual_candidate,
            });
        }
        let candidate = Sample::from_owned_buffer(&candidate_bytes)?;
        validate_test_id(candidate.test_id())?;
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
        rerender_check(&candidate)
            .map_err(|error| error.with_observed_baseline(&previous_digest))?;
        let changes = match baseline_bytes {
            None => ChangeSet::missing(),
            Some(bytes) => {
                let baseline = Sample::from_owned_buffer(&bytes)
                    .map_err(ApproveError::from)
                    .map_err(|error| error.with_observed_baseline(&previous_digest))?;
                let comparison = FrozenStore::compare_present(&candidate, &baseline)
                    .map_err(ApproveError::from)
                    .map_err(|error| error.with_observed_baseline(&previous_digest))?;
                ChangeSet::from_present(comparison)
            }
        };
        write_baseline(
            baselines,
            &relative,
            &candidate,
            &candidate_bytes,
            &actual_candidate,
            &previous_digest,
            changes,
        )
        .map_err(|error| error.with_observed_baseline(&previous_digest))
    }
}
