//! `report`: a static HTML index from saved candidate evidence.
//!
//! The typed [`tuiscotti::write_report_index`] operation walks the
//! artifact root and writes the index; this handler only maps its
//! outcome to the Section 16.3 result. Nothing here captures state or
//! changes approvals: the baseline root is never even opened.

use std::path::Path;

use tuiscotti::write_report_index;

use crate::json_out::{CliResult, emit, encode_path};

/// Run `report` and return the process exit code.
pub(crate) fn cmd_report(artifacts: &Path, out: &Path, as_json: bool) -> i32 {
    let failed = |error: tuiscotti::IndexError| {
        let reason = error.to_string();
        eprintln!("error: {reason}");
        let result = CliResult::new(
            "report",
            "error",
            None,
            Some(reason),
            None,
            None,
            None,
            Some(encode_path(out)),
            Some(error.code().to_string()),
        );
        emit(&result, 3, as_json)
    };
    match write_report_index(artifacts, out) {
        Ok(_) => {
            let result = CliResult::new(
                "report",
                "pass",
                None,
                None,
                None,
                None,
                None,
                Some(encode_path(out)),
                None,
            );
            emit(&result, 0, as_json)
        }
        Err(error) => failed(error),
    }
}
