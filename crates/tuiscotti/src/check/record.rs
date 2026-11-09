use std::path::Path;

use crate::report::{PixelSummary, ReportInput, atomic_write};
use crate::sample::{Gate, SampleError, read_bounded};

use super::diff::StateDiff;
use super::evidence::SavedImages;
use super::{CHECK_SCHEMA, CheckError, MISSING_BASELINE, MismatchReason};

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
pub(super) struct ResultRecord {
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
    pub(super) fn for_failure(
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

    pub(super) fn save(&self, check_dir: &Path) -> Result<(), CheckError> {
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

    pub(super) fn report_input<'a>(
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
