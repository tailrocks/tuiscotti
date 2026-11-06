use std::fs;
use std::path::{Path, PathBuf};

use crate::report::{atomic_write, render_report};
use crate::sample::{Gate, Sample, SampleError, read_bounded};

use super::diff::StateDiff;
use super::paths::{ensure_no_symlink, sha256_hex};
use super::pixels::{DiffPng, encode_diff_png};
use super::record::ResultRecord;
use super::{CheckError, CheckFailure, FrozenStore, MISSING_BASELINE, MismatchReason};

impl FrozenStore {
    #[expect(
        clippy::too_many_arguments,
        reason = "failure evidence needs every compared digest and byte string"
    )]
    pub(super) fn write_failure(
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
        let mut html = render_report(&input);
        let mut links = String::from("<p><a href=\"../../sample.tui.pb\">candidate sample</a>");
        if baseline_bytes.is_some() {
            links.push_str(" · <a href=\"expected.tui.pb\">baseline sample</a>");
        }
        links.push_str("</p>\n");
        if let Some(position) = html.rfind("</body>") {
            html.insert_str(position, &links);
        }
        if report_path.exists() {
            validate_saved_report(&report_path, &html)?;
        } else {
            atomic_write(&check_dir, "report.html", html.as_bytes())
                .map_err(|source| CheckError::Io { source })?;
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
        let path = candidate_dir.join("sample.tui.pb");
        if path.exists() {
            let saved = read_bounded(&path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
            validate_saved_sample(&path, &saved, candidate_digest)?;
            return Ok(path);
        }
        atomic_write(candidate_dir, "sample.tui.pb", candidate_bytes)
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
        let path = check_dir.join("expected.tui.pb");
        if path.exists() {
            let saved = read_bounded(&path).map_err(|error| match error {
                SampleError::Io { source } => CheckError::Io { source },
                other => CheckError::from(other),
            })?;
            validate_saved_sample(&path, &saved, baseline_digest)?;
            return Ok(());
        }
        atomic_write(check_dir, "expected.tui.pb", bytes)
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

pub(super) struct SavedImages {
    pub(super) has_expected: bool,
    pub(super) has_actual: bool,
    pub(super) has_diff: bool,
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
    Sample::from_bytes(saved).map_err(|source| saved_sample_decode_error(path, source))?;
    if sha256_hex(saved) != digest {
        return Err(CheckError::EvidenceCorrupt {
            detail: format!("{} digest does not match its evidence key", path.display()),
        });
    }
    Ok(())
}

fn saved_sample_decode_error(path: &Path, source: SampleError) -> CheckError {
    match crate::approve::sample_code(&source) {
        "RESOURCE_LIMIT" | "UNSUPPORTED_SCHEMA" | "INVALID_IMAGE" => CheckError::from(source),
        _ => CheckError::EvidenceCorrupt {
            detail: format!("{} fails validation: {source}", path.display()),
        },
    }
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
        crate::sample::decode_png_bounded(&saved)
            .map_err(|error| saved_png_decode_error(&path, error))?;
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

fn saved_png_decode_error(path: &Path, error: SampleError) -> CheckError {
    if crate::approve::sample_code(&error) == "RESOURCE_LIMIT" {
        CheckError::from(error)
    } else {
        CheckError::EvidenceCorrupt {
            detail: format!("{} is not a decodable image: {error}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests;
