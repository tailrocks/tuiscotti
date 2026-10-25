//! `accept`: the explicit D04 approval transaction, fully offline.
//!
//! Approves one candidate for one test ID with the mandatory digests.
//! There is no `--all`, no glob, and no CI override: a set `CI` variable
//! refuses approval before any write. Exit and machine codes come
//! straight from the library mapping: malformed digest arguments report
//! `INVALID_SAMPLE` at exit 2, every other failure reports its Section
//! 16.3 code at exit 3.

use std::path::Path;

use tuiscotti::approve::{ApprovalStore, ApproveError};

use crate::json_out::{CliResult, emit, encode_path};

/// Run `accept` and return the process exit code.
pub(crate) fn cmd_accept(
    candidate: &Path,
    candidate_sha256: &str,
    expect_baseline: &str,
    baselines: &Path,
    as_json: bool,
) -> i32 {
    let failed = |error: &ApproveError| {
        let reason = error.to_string();
        eprintln!("error: {reason}");
        let result = CliResult::new(
            "accept",
            "error",
            None,
            Some(reason),
            Some(encode_path(candidate)),
            Some(candidate_sha256.to_string()),
            Some(expect_baseline.to_string()),
            None,
            Some(error.code().to_string()),
        );
        emit(&result, error.exit_code(), as_json)
    };
    let store = match ApprovalStore::new(baselines) {
        Ok(store) => store,
        Err(error) => return failed(&error),
    };
    match store.accept(candidate, candidate_sha256, expect_baseline) {
        Ok(outcome) => {
            let result = CliResult::new(
                "accept",
                "pass",
                Some(outcome.test_id().to_string()),
                None,
                Some(encode_path(candidate)),
                Some(outcome.candidate_sha256().to_string()),
                Some(outcome.baseline_sha256().to_string()),
                Some(encode_path(outcome.baseline_path())),
                None,
            );
            emit(&result, 0, as_json)
        }
        Err(error) => failed(&error),
    }
}
