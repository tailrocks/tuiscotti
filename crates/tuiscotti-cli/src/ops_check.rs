//! `check`: frozen comparison of a saved sample, fully offline.
//!
//! Reads the sample, compares it against the baseline for its test ID
//! through [`FrozenStore`](tuiscotti::check::FrozenStore), and reports
//! the outcome. Nothing here starts an application or writes a
//! baseline: a missing baseline is a `missing` failure at exit 1, and
//! failure evidence goes to the artifact root only.

use std::path::Path;

use tuiscotti::approve::sample_code;
use tuiscotti::check::{CheckError, FrozenStore, MISSING_BASELINE};

use crate::input::read_sample_bytes;
use crate::json_out::{CliResult, emit, encode_path};

/// The Section 16.3 code for a frozen-check failure, reusing the library
/// mapping: sample shapes keep their D04 codes, bad store paths are
/// `INVALID_STORE_PATH`, pixel-comparison refusals are `INVALID_IMAGE`,
/// and corrupt evidence, image-encoding, I/O, and report failures are
/// `IO`. Every shape exits 3.
fn check_error_code(error: &CheckError) -> &'static str {
    match error {
        CheckError::Sample(source) => sample_code(source),
        CheckError::InvalidTestId { .. } | CheckError::InvalidStorePath { .. } => {
            "INVALID_STORE_PATH"
        }
        CheckError::Pixel(_) => "INVALID_IMAGE",
        CheckError::EvidenceCorrupt { .. }
        | CheckError::Image { .. }
        | CheckError::Io { .. }
        | CheckError::Report(_)
        | _ => "IO",
    }
}

/// Emit a check error: the input path is known, the digest only when the
/// sample bytes were read.
fn emit_error(
    reason: String,
    code: &str,
    sample: &Path,
    candidate_sha256: Option<String>,
    as_json: bool,
) -> i32 {
    eprintln!("error: {reason}");
    let result = CliResult::new(
        "check",
        "error",
        None,
        Some(reason),
        Some(encode_path(sample)),
        candidate_sha256,
        None,
        None,
        Some(code.to_string()),
    );
    emit(&result, 3, as_json)
}

/// Emit a passing comparison.
fn emit_pass(
    test_id: &str,
    sample: &Path,
    candidate_sha256: &str,
    baseline_sha256: &str,
    as_json: bool,
) -> i32 {
    let result = CliResult::new(
        "check",
        "pass",
        Some(test_id.to_string()),
        None,
        Some(encode_path(sample)),
        Some(candidate_sha256.to_string()),
        Some(baseline_sha256.to_string()),
        None,
        None,
    );
    emit(&result, 0, as_json)
}

/// Emit a failing comparison: `missing` without a baseline, `different`
/// with the mismatch reason otherwise.
fn emit_failure(failure: &tuiscotti::check::CheckFailure, as_json: bool) -> i32 {
    let missing = failure.baseline_sha256() == MISSING_BASELINE;
    let result = CliResult::new(
        "check",
        if missing { "missing" } else { "different" },
        Some(failure.test_id().to_string()),
        Some(if missing {
            MISSING_BASELINE.to_string()
        } else {
            failure.reason().as_str().to_string()
        }),
        Some(encode_path(failure.candidate_path())),
        Some(failure.candidate_sha256().to_string()),
        Some(failure.baseline_sha256().to_string()),
        Some(encode_path(failure.report_path())),
        None,
    );
    emit(&result, 1, as_json)
}

/// Run `check` and return the process exit code.
pub(crate) fn cmd_check(sample: &Path, baselines: &Path, artifacts: &Path, as_json: bool) -> i32 {
    let bytes = match read_sample_bytes(sample) {
        Ok(bytes) => bytes,
        Err(failure) => {
            return emit_error(
                failure.message(sample),
                failure.code(),
                sample,
                None,
                as_json,
            );
        }
    };
    let candidate_sha256 = tuiscotti::sample::Sample::candidate_id(&bytes);
    let store = match FrozenStore::new(baselines, artifacts) {
        Ok(store) => store,
        Err(error) => {
            let reason = error.to_string();
            let code = check_error_code(&error);
            return emit_error(reason, code, sample, Some(candidate_sha256), as_json);
        }
    };
    match store.check(&bytes) {
        Ok(tuiscotti::check::CheckOutcome::Pass(success)) => emit_pass(
            success.test_id(),
            sample,
            success.candidate_sha256(),
            success.baseline_sha256(),
            as_json,
        ),
        Ok(tuiscotti::check::CheckOutcome::Failure(failure)) => emit_failure(&failure, as_json),
        Err(error) => {
            let reason = error.to_string();
            let code = check_error_code(&error);
            emit_error(reason, code, sample, Some(candidate_sha256), as_json)
        }
    }
}
