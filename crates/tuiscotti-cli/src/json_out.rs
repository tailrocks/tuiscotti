#![doc = include_str!("../docs/json_out.md")]
use std::path::Path;

use base64::Engine as _;
use serde::Serialize;

pub(crate) const CLI_SCHEMA: &str = "tuiscotti.cli.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EncodedPath {
    encoding: String,
    value: String,
}

#[must_use]
pub(crate) fn encode_path(path: &Path) -> EncodedPath {
    if let Some(text) = path.as_os_str().to_str() {
        return EncodedPath {
            encoding: "utf8".to_string(),
            value: text.to_string(),
        };
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        EncodedPath {
            encoding: "unix-bytes-base64".to_string(),
            value: base64::engine::general_purpose::STANDARD.encode(path.as_os_str().as_bytes()),
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        let units: Vec<u16> = path.as_os_str().encode_wide().collect();
        let mut bytes = Vec::with_capacity(units.len() * 2);
        for unit in units {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        EncodedPath {
            encoding: "windows-utf16le-base64".to_string(),
            value: base64::engine::general_purpose::STANDARD.encode(&bytes),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        EncodedPath {
            encoding: "utf8".to_string(),
            value: path.as_os_str().to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CliResult {
    schema: String,
    command: String,
    status: String,
    test_id: Option<String>,
    reason: Option<String>,
    candidate_path: Option<EncodedPath>,
    candidate_sha256: Option<String>,
    baseline_sha256: Option<String>,
    output_path: Option<EncodedPath>,
    error_code: Option<String>,
}

impl CliResult {
    #[expect(clippy::too_many_arguments, reason = "one constructor per schema")]
    pub(crate) fn new(
        command: &str,
        status: &str,
        test_id: Option<String>,
        reason: Option<String>,
        candidate_path: Option<EncodedPath>,
        candidate_sha256: Option<String>,
        baseline_sha256: Option<String>,
        output_path: Option<EncodedPath>,
        error_code: Option<String>,
    ) -> Self {
        Self {
            schema: CLI_SCHEMA.to_string(),
            command: command.to_string(),
            status: status.to_string(),
            test_id,
            reason,
            candidate_path,
            candidate_sha256,
            baseline_sha256,
            output_path,
            error_code,
        }
    }

    #[must_use]
    pub(crate) fn human(&self) -> String {
        let mut line = format!("{}: {}", self.command, self.status);
        if let Some(test_id) = &self.test_id {
            line = format!("{line} test={test_id}");
        }
        if let Some(reason) = &self.reason {
            line = format!("{line} reason={reason}");
        }
        if let Some(code) = &self.error_code {
            line = format!("{line} error={code}");
        }
        line
    }
}

pub(crate) fn emit(result: &CliResult, exit_code: i32, as_json: bool) -> i32 {
    let mut line = if as_json {
        match serde_json::to_string(result) {
            Ok(line) => line,
            Err(error) => {
                eprintln!("error: result serialization failed: {error}");
                return 3;
            }
        }
    } else {
        result.human()
    };
    line.push('\n');
    match crate::write_stdout(&line) {
        crate::StdoutFlush::Written | crate::StdoutFlush::BrokenPipe => exit_code,
        crate::StdoutFlush::Failed => 3,
    }
}
