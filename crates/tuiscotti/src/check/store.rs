use std::fs;
use std::path::{Path, PathBuf};

use crate::sample::{Gate, Sample, SampleError, read_bounded, read_bounded_optional};

use super::diff::{StateDiff, diff_snapshots};
use super::paths::{ensure_no_symlink, resolve_root, sha256_hex};
use super::pixels::{PixelVerdict, decide_pixels};
use super::{
    CheckError, CheckOutcome, CheckSuccess, FrozenStore, MISSING_BASELINE, MismatchReason,
    PresentComparison, validate_test_id,
};

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
        let actual = Sample::from_bytes(sample_bytes)?;
        self.check_decoded(sample_bytes, &actual)
    }

    #[doc = "Compare an owned sample buffer, accounting for its full allocation while retaining its exact bytes for evidence.\n\n# Errors\n\nReturns [`CheckError`] for invalid samples, test IDs, store paths, corrupt saved evidence, or I/O failures."]
    pub fn check_owned(&self, sample_bytes: Vec<u8>) -> Result<CheckOutcome, CheckError> {
        let outcome = (|| {
            let actual = Sample::from_owned_buffer(&sample_bytes)?;
            self.check_decoded(&sample_bytes, &actual)
        })();
        drop(sample_bytes);
        outcome
    }

    fn check_decoded(
        &self,
        sample_bytes: &[u8],
        actual: &Sample,
    ) -> Result<CheckOutcome, CheckError> {
        let candidate_sha256 = sha256_hex(sample_bytes);
        validate_test_id(actual.test_id())?;
        let baseline_path = self.baseline_path(actual.test_id())?;
        let baseline_bytes =
            read_bounded_optional(&baseline_path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
        match baseline_bytes {
            None => self.fail_without_baseline(actual, &candidate_sha256, sample_bytes),
            Some(bytes) => {
                self.compare_with_baseline(actual, &candidate_sha256, sample_bytes, &bytes)
            }
        }
    }

    #[doc = "Read a sample file with the shared bounded read, then [`FrozenStore::check`] it.\n\n# Errors\n\nReturns [`CheckError`] when the file is unreadable or the check fails."]
    pub fn check_file(&self, path: &Path) -> Result<CheckOutcome, CheckError> {
        let bytes = read_bounded(path).map_err(|error| match error {
            SampleError::Io { source } => CheckError::Io { source },
            other => CheckError::from(other),
        })?;
        self.check_owned(bytes)
    }

    fn compare_with_baseline(
        &self,
        actual: &Sample,
        candidate_digest: &str,
        candidate_bytes: &[u8],
        baseline_bytes: &Vec<u8>,
    ) -> Result<CheckOutcome, CheckError> {
        let baseline_digest = sha256_hex(baseline_bytes);
        let baseline = Sample::from_owned_buffer(baseline_bytes)?;
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
        relative.set_extension("tui.pb");
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
