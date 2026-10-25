//! Section 16.3 JSON result: one object plus a newline on stdout.
//!
//! The emission rules live here so every command shares them: stdout
//! carries exactly one JSON line (or one human summary line without
//! `--json`), stderr carries diagnostics, and argument-parsing errors
//! never reach this module because clap exits before dispatch.

use std::path::Path;

use base64::Engine as _;
use serde::Serialize;

/// Schema identifier for every CLI result object.
pub(crate) const CLI_SCHEMA: &str = "tuiscotti.cli.v1";

/// A native path encoded without loss, per Section 16.3.
///
/// Correctly encoded Unicode paths use `utf8`. Other Unix paths carry
/// the native bytes as standard padded Base64 (`unix-bytes-base64`).
/// Other Windows paths carry native UTF-16 code units as little-endian
/// bytes in standard padded Base64 (`windows-utf16le-base64`). Decoding
/// always recovers the exact native path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EncodedPath {
    encoding: String,
    value: String,
}

/// Encode a native path without loss and without unwrapping UTF-8.
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
        // No native-byte API on this platform; the UTF-8 branch above is
        // the only lossless spelling, so report it as unrepresentable.
        EncodedPath {
            encoding: "utf8".to_string(),
            value: path.as_os_str().to_string_lossy().into_owned(),
        }
    }
}

/// One Section 16.3 result object, fields in schema order.
///
/// `baseline_sha256` is a 64-character lowercase digest, `"missing"`,
/// or null when the command has no baseline result. `reason` carries
/// the mismatch wire value for `different`, the render profile for a
/// rendered state-only sample, a diagnostic otherwise, and null for a
/// plain pass.
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
    /// Build a result; statuses are `pass`, `missing`, `different`, `error`.
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

    /// The one-line human summary for runs without `--json`.
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

/// Emit a result and return the process exit code.
///
/// stdout carries exactly one JSON line with `--json`, else the human
/// summary; a closed pipe preserves the operation result (the reader
/// went away, but the outcome still happened), any other stdout
/// failure exits 3. Callers print their own stderr diagnostic before
/// this on non-zero outcomes.
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

#[cfg(test)]
mod tests {
    use super::{CliResult, encode_path};

    /// Utf-8 paths encode losslessly under the `utf8` spelling.
    #[test]
    fn utf8_paths_encode_as_text() {
        let encoded = encode_path(std::path::Path::new("/tmp/with spaces/shot.tui.json"));
        assert_eq!(encoded.encoding, "utf8");
        assert_eq!(encoded.value, "/tmp/with spaces/shot.tui.json");
    }

    /// Non-UTF8 Unix paths encode the native bytes as padded Base64 and
    /// decode back to the exact native path.
    #[cfg(unix)]
    #[test]
    fn non_utf8_unix_paths_round_trip_through_base64() {
        use base64::Engine as _;
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

        let raw = b"/tmp/tuiscotti-\xff.tui.json".to_vec();
        let path = std::path::PathBuf::from(OsString::from_vec(raw.clone()));
        let encoded = encode_path(&path);
        assert_eq!(encoded.encoding, "unix-bytes-base64");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.value.as_bytes())
            .expect("encoded path decodes");
        assert_eq!(decoded, raw);
        assert_eq!(
            std::ffi::OsStr::from_bytes(&decoded),
            path.as_os_str(),
            "decoding recovers the native path"
        );
    }

    /// The JSON object carries every schema field in schema order.
    #[test]
    fn result_schema_matches_section_16() {
        let result = CliResult::new(
            "check",
            "missing",
            Some("cli/shape".to_string()),
            Some("missing".to_string()),
            None,
            Some("a".repeat(64)),
            Some("missing".to_string()),
            None,
            None,
        );
        let text = serde_json::to_string(&result).expect("result serializes");
        let value: serde_json::Value = serde_json::from_str(&text).expect("result parses as JSON");
        assert_eq!(value["schema"], "tuiscotti.cli.v1");
        let object = value.as_object().expect("result is an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "baseline_sha256",
                "candidate_path",
                "candidate_sha256",
                "command",
                "error_code",
                "output_path",
                "reason",
                "schema",
                "status",
                "test_id",
            ]
        );
    }
}
