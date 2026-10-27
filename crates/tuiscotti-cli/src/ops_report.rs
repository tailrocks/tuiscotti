#![doc = include_str!("../docs/ops_report.md")]
use std::path::Path;

use tuiscotti::write_report_index;

use crate::json_out::{CliResult, emit, encode_path};

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
