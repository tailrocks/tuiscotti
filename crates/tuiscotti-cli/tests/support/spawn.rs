//! Shared spawn harness for the offline CLI tests: binary spawning
//! behind a serialized fork gate plus JSON and `NO_COLOR` assertions.
//!
//! Spawning serializes across threads of one test binary: platforms
//! without atomic-CLOEXEC pipes race a `fork()` between a sibling's
//! `pipe()` and its CLOEXEC setup, leaking pipe fds into the child.
//! The lock covers `spawn()` only, so waits still run in parallel.
//!
//! No test touches process-wide state: `CI` values live only on spawned
//! child environments, never in the runner process.

use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};

/// The compiled `tuiscotti` executable under test.
#[must_use]
pub(crate) fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tuiscotti"))
}

static SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Spawn a child behind the fork gate.
pub(crate) fn spawn_locked(cmd: &mut Command) -> std::io::Result<Child> {
    let _guard = SPAWN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cmd.spawn()
}

/// Run the CLI with `CI` explicitly set on the child, regardless of the
/// runner environment: the substantive non-approval flows must assert
/// their real outcomes (pass, missing, different, operational error)
/// under `CI`, not just the refusal. Approval itself stays refused here
/// by production design; only the dedicated CI-refusal test asserts
/// that, while frozen pre-written baselines plus `check` carry the
/// transaction outcomes.
pub(crate) fn run_cli_ci(args: &[&str]) -> Output {
    run_cli_env(args, &[("CI", "true")], false)
}

/// Run the CLI with explicit child environment control. Approval
/// (`accept`) paths pass `remove_ci` so the transaction runs on the
/// non-CI lane production requires; every other flow sets `CI`
/// explicitly (see [`run_cli_ci`]) so both lanes prove behavior under
/// `CI` instead of stripping it.
#[expect(clippy::expect_used, reason = "test children must spawn")]
pub(crate) fn run_cli_env(args: &[&str], vars: &[(&str, &str)], remove_ci: bool) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    if remove_ci {
        cmd.env_remove("CI");
    }
    for (key, value) in vars {
        cmd.env(key, value);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = spawn_locked(&mut cmd).expect("CLI child spawns");
    child.wait_with_output().expect("CLI child waits")
}

/// Exit code of a finished child.
#[must_use]
pub(crate) fn code(out: &Output) -> Option<i32> {
    out.status.code()
}

/// Parse `--json` stdout: exactly one newline-terminated object.
#[expect(clippy::expect_used, reason = "JSON shape is the assertion")]
pub(crate) fn parse_json(out: &Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "JSON stdout is one line, got {stdout:?}");
    serde_json::from_str(lines[0]).expect("stdout parses as JSON")
}

/// Assert a byte stream carries no terminal escapes (the recorded
/// `NO_COLOR` behavior: plain output, no color framework).
pub(crate) fn assert_no_ansi(bytes: &[u8], what: &str) {
    assert!(
        !bytes.contains(&0x1b),
        "{what} carries terminal escapes: {bytes:?}"
    );
}

/// Assert both streams carry no terminal escapes.
pub(crate) fn assert_no_color(out: &Output, what: &str) {
    assert_no_ansi(&out.stdout, &format!("{what} stdout"));
    assert_no_ansi(&out.stderr, &format!("{what} stderr"));
}
