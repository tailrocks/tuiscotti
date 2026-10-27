#![doc = include_str!("../docs/ops_accept.md")]
use std::path::Path;

use tuiscotti::approve::{ApprovalStore, ApproveError};

use crate::json_out::{CliResult, emit, encode_path};

fn accept_exit_code(error: &ApproveError) -> i32 {
    match error {
        ApproveError::InvalidArgument { .. } => 2,
        _ => 3,
    }
}

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
        let baseline_sha256 = error.observed_baseline().map(str::to_string);
        let result = CliResult::new(
            "accept",
            "error",
            None,
            Some(reason),
            Some(encode_path(candidate)),
            Some(candidate_sha256.to_string()),
            baseline_sha256,
            None,
            Some(error.code().to_string()),
        );
        emit(&result, accept_exit_code(error), as_json)
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
