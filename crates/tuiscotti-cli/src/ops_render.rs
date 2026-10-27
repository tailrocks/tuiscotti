#[doc = include_str!("../docs/ops_render.md")]
use std::path::Path;

use tuiscotti::render_sample;
use tuiscotti::sample::Sample;

use crate::input::read_sample_bytes;
use crate::json_out::{CliResult, emit, encode_path};

pub(crate) fn cmd_render(sample_path: &Path, out: &Path, as_json: bool) -> i32 {
    let failed = |reason: String, code: &str, candidate_sha256: Option<String>| {
        eprintln!("error: {reason}");
        let result = CliResult::new(
            "render",
            "error",
            None,
            Some(reason),
            Some(encode_path(sample_path)),
            candidate_sha256,
            None,
            None,
            Some(code.to_string()),
        );
        emit(&result, 3, as_json)
    };
    let bytes = match read_sample_bytes(sample_path) {
        Ok(bytes) => bytes,
        Err(failure) => {
            return failed(failure.message(sample_path), failure.code(), None);
        }
    };
    match render_sample(&bytes, out) {
        Ok(outcome) => {
            let result = CliResult::new(
                "render",
                "pass",
                Some(outcome.test_id().to_string()),
                outcome.reason().map(str::to_string),
                Some(encode_path(sample_path)),
                Some(outcome.candidate_sha256().to_string()),
                None,
                Some(encode_path(outcome.output_path())),
                None,
            );
            emit(&result, 0, as_json)
        }
        Err(error) => {
            let reason = error.to_string();
            let code = error.code();
            let candidate_sha256 = Sample::candidate_id(&bytes);
            failed(reason, code, Some(candidate_sha256))
        }
    }
}
