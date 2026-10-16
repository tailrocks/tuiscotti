use serde::{Deserialize, Serialize};

/// Where a frame came from. Recorded so reports are auditable; `created_unix`
/// is informational only and excluded from equality comparisons that must be
/// deterministic (use [`Frame::digest`](crate::frame::Frame::digest) / cell comparison for gates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Capturing tool name.
    pub tool: String,
    /// Capturing tool version.
    pub tool_version: String,
    /// Active profile name.
    pub profile: String,
    /// Capture source (view path, session id, ...).
    pub source: String,
    /// Invoked command line.
    pub argv: Vec<String>,
    /// Creation time as Unix seconds (informational only).
    pub created_unix: u64,
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            tool: "tuiscotti".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            profile: "tuiscotti-default".to_string(),
            source: "ansi".to_string(),
            argv: Vec::new(),
            created_unix: 0,
        }
    }
}

impl Provenance {
    /// Provenance stamped with the current Unix time (0 when the clock fails).
    ///
    /// Normalizes `source` and `argv` via [`Provenance::sanitized`] so
    /// host-specific absolute paths never enter gated, sealed, or committed
    /// artifacts.
    #[must_use]
    pub fn now(profile: &str, source: &str, argv: Vec<String>) -> Self {
        Self {
            tool: "tuiscotti".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            profile: profile.to_string(),
            source: source.to_string(),
            argv,
            created_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        }
        .sanitized()
    }

    /// Return a copy with host-specific paths stripped from `source` and `argv`
    /// (`argv[0]` reduced to its file name, and any absolute path argument or
    /// `--flag=/abs/path` value reduced to its file name).
    #[must_use]
    pub fn sanitized(&self) -> Self {
        let mut argv = self.argv.clone();
        if let Some(first) = argv.first_mut() {
            *first = file_name_only(first);
        }
        for arg in argv.iter_mut().skip(1) {
            *arg = sanitize_token(arg);
        }
        Self {
            tool: self.tool.clone(),
            tool_version: self.tool_version.clone(),
            profile: self.profile.clone(),
            source: sanitize_token(&self.source),
            argv,
            created_unix: self.created_unix,
        }
    }
}

fn is_abs_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    s.starts_with('/')
        || s.starts_with('\\')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes[2] == b'/' || bytes[2] == b'\\'))
}

fn file_name_only(s: &str) -> String {
    if !s.contains(['/', '\\']) {
        return s.to_string();
    }
    s.rsplit(['/', '\\'])
        .find(|seg| !seg.is_empty())
        .unwrap_or("root")
        .to_string()
}

fn sanitize_token(s: &str) -> String {
    if is_abs_path(s) {
        return file_name_only(s);
    }
    if let Some((k, v)) = s.split_once('=')
        && is_abs_path(v)
    {
        return format!("{k}={}", file_name_only(v));
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::Provenance;
    use crate::frame::Frame;

    #[test]
    fn provenance_strips_host_paths_from_argv_and_source() {
        let p = Provenance::now(
            "ci-80x24",
            "/Users/alice/work/repo/snapshots/home.json",
            vec![
                "/Users/alice/work/repo/target/debug/my-bin".to_string(),
                "--config=/Users/alice/.config/app.toml".to_string(),
                "/var/folders/tmp/input.txt".to_string(),
                "--port".to_string(),
                "8080".to_string(),
            ],
        );
        assert_eq!(p.source, "home.json");
        assert_eq!(
            p.argv,
            vec!["my-bin", "--config=app.toml", "input.txt", "--port", "8080"]
        );

        let frame = Frame::blank(
            10,
            4,
            Provenance {
                tool: "tuiscotti".to_string(),
                tool_version: "0.1.0".to_string(),
                profile: "ci".to_string(),
                source: "/home/runner/work/app/view.rs".to_string(),
                argv: vec!["C:\\Users\\runner\\target\\debug\\app.exe".to_string()],
                created_unix: 42,
            },
        );
        let json = frame.to_json();
        assert!(!json.contains("/home/runner"), "leaked unix path: {json}");
        assert!(!json.contains("C:\\\\Users"), "leaked windows path: {json}");
        assert!(json.contains("\"app.exe\""), "kept binary name: {json}");
        assert!(json.contains("\"view.rs\""), "kept source file: {json}");
    }
}
