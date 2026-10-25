//! D06 migration tests: live sessions, locators, input, and waits through
//! the replacement architecture.
//!
//! Covers the D06 subcase from `docs/migration/corpus.json`:
//!
//! - C02: a live session sample through the downstream path — a
//!   `tui-test::Session` runs a fixture program, an upstream locator waits
//!   for readiness, the owned snapshot converts into a downstream `Sample`,
//!   and `FrozenStore` checks it after explicit approval.
//!
//! Pass-through coverage for the upstream-owned corpus cases, proving they
//! hold through the downstream dependency (`tui-test` at the qual-stack
//! head): C13 checked mouse, paste, and focus; C14 child environment;
//! C15 native arguments and raw bytes; C16 quiet intervals and
//! cancellation. Every behavior goes through the accepted upstream method:
//! `Session::run` / `Engine::spawn_with` with `LaunchOptions`,
//! `mouse_checked` / `paste` / `focus`, `write_bytes`, and
//! `wait_quiet` / `WaitQuietOptions`.
//!
//! API split (recorded in the migration decision log): session lifecycle,
//! upstream locators, snapshots, and quiet waits go through the runtime
//! `tui_test::Session`; `spawn_with` with `LaunchOptions` and the checked
//! input methods live on `tui-test::Engine`, the only upstream surface
//! exposing them. No tuiscotti session, locator, input, or wait helper is
//! used anywhere here.
//!
//! Fixtures re-run this test binary as the PTY child (libtest-compatible
//! arguments only; child configuration travels in environment variables).
//! Filesystem state stays in tempdirs; the parallel runner environment is
//! never mutated. Bounded waits poll on a 10-millisecond interval against a
//! deadline — the upstream-established bounded-wait idiom, never a
//! race-hiding sleep.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use tui_test::engine::{CheckedInputError, CheckedMouseAction, Engine};
use tui_test::logger::Logger;
use tui_test::{
    AutomaticRecording, AutomaticRecordingMode, LaunchEnvMode, LaunchOptions, LocatorExpectOptions,
    MouseButton, MouseOptions, OpenOptions, Operation, OperationResult, RunOptions, Session,
    SnapshotScope, Timeouts, WaitQuietMode, WaitQuietOptions,
};
use tuiscotti::approve::{ApprovalStore, ApproveError};
use tuiscotti::check::{CheckOutcome, FrozenStore, MISSING_BASELINE};
use tuiscotti::render_profile::{PROFILE_NAME, resolve_profile};
use tuiscotti::sample::{Observation, Sample};

/// Child-mode selector for the re-executed fixture.
const FIXTURE_ENV: &str = "TUI_TEST_D06_FIXTURE";
/// Output-file path for the raw-byte fixture.
const FIXTURE_OUT: &str = "TUI_TEST_D06_OUT";
/// Exact byte count the raw-byte fixture reads before exiting.
const FIXTURE_LEN: &str = "TUI_TEST_D06_LEN";
/// Sentinel variable proving explicit environment overrides.
const SENTINEL: &str = "TUI_TEST_D06_SENTINEL";

/// Fixture entry point, run inside the test binary as the PTY child.
#[test]
fn d06_fixture() {
    let Ok(mode) = std::env::var(FIXTURE_ENV) else {
        return;
    };
    match mode.as_str() {
        "mouse" => run_echo_fixture(&["\x1b[?1000h", "\x1b[?1006h"]),
        "paste-focus" => run_echo_fixture(&["\x1b[?2004h", "\x1b[?1004h"]),
        "print-env" => run_print_env(),
        "byte-echo" => run_byte_echo(),
        other => panic!("unknown D06 fixture mode: {other}"),
    }
}

/// Enable terminal modes, announce readiness, then echo every stdin chunk as
/// a hex line. The `stty` call matches the upstream checked-input fixture:
/// raw delivery with local echo off, so observed bytes are exact.
fn run_echo_fixture(modes: &[&str]) {
    use std::io::Read as _;
    assert!(
        std::process::Command::new("stty")
            .args(["raw", "-echo"])
            .status()
            .is_ok_and(|status| status.success()),
        "fixture stty runs"
    );
    let mut stdout = std::io::stdout().lock();
    for mode in modes {
        emit(&mut stdout, mode.as_bytes());
    }
    emit(&mut stdout, b"READY\r\n");
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match std::io::stdin().lock().read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                buffer.extend_from_slice(&chunk[..count]);
                let taken = std::mem::take(&mut buffer);
                echo_hex(&mut stdout, &taken);
            }
        }
    }
}

#[expect(clippy::expect_used, reason = "test fixture setup")]
fn emit(stdout: &mut impl std::io::Write, bytes: &[u8]) {
    stdout.write_all(bytes).expect("fixture stdout write");
    stdout.flush().expect("fixture stdout flush");
}

#[expect(clippy::expect_used, reason = "test fixture setup")]
fn echo_hex(stdout: &mut impl std::io::Write, bytes: &[u8]) {
    use std::fmt::Write as _;
    if bytes.is_empty() {
        return;
    }
    let mut line = String::from("rx:");
    for byte in bytes {
        write!(line, "{byte:02x}").expect("hex formatting never fails");
    }
    line.push_str("\r\n");
    emit(stdout, line.as_bytes());
}

/// Print only the filtered environment keys, then exit. The parent waits
/// for the exit with a bounded wait and reads the final screen — no sleep
/// anywhere, and two lines never scroll.
fn run_print_env() {
    let wanted = std::env::var("TUI_TEST_D06_KEYS").unwrap_or_default();
    let wanted: Vec<&str> = wanted.split(',').collect();
    let mut stdout = std::io::stdout().lock();
    for key in wanted {
        match std::env::var(key) {
            Ok(value) => emit(&mut stdout, format!("ENV:{key}={value}\r\n").as_bytes()),
            Err(_) => emit(&mut stdout, format!("ENV:{key}=<absent>\r\n").as_bytes()),
        }
    }
}

/// Read an exact byte count from stdin (cooked mode delivers the line after
/// its newline; the payload carries no signal or flow-control bytes) and
/// write those bytes verbatim to the output file.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn run_byte_echo() {
    use std::io::Read as _;
    emit(&mut std::io::stdout().lock(), b"READY\r\n");
    let out = std::env::var(FIXTURE_OUT).expect("byte-echo fixture needs an output path");
    let len: usize = std::env::var(FIXTURE_LEN)
        .expect("byte-echo fixture needs a length")
        .parse()
        .expect("byte-echo length parses");
    let mut bytes = vec![0u8; len];
    std::io::stdin()
        .lock()
        .read_exact(&mut bytes)
        .expect("byte-echo fixture reads its payload");
    std::fs::write(&out, &bytes).expect("byte-echo fixture writes its output");
}

/// Unique tempdir per test: no two parallel tests share filesystem state.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn test_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().expect("D06 test directory")
}

/// A disabled-recording engine writing its cast into the test tempdir.
fn test_engine(dir: &Path, label: &str) -> Engine {
    Engine::new(
        format!("d06-{label}-{}", std::process::id()),
        Arc::new(Logger::disabled()),
        dir.join("session.cast"),
    )
}

/// `RunOptions` for a shell fixture program with explicit child env.
fn shell_options(program: &str, args: &[&str], env: Vec<(String, String)>) -> RunOptions {
    let defaults = OpenOptions::default();
    RunOptions {
        backend: defaults.backend,
        program: program.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        profile: defaults.profile,
        style: defaults.style,
        cols: 80,
        rows: 24,
        cwd: None,
        env,
        wait_ready: Some(false),
        restart: false,
        timeouts: Timeouts::default(),
        recording: AutomaticRecording {
            mode: AutomaticRecordingMode::Disabled,
            directory: None,
        },
    }
}

/// `RunOptions` shell for `Engine::spawn_with`: the launch supplies program,
/// arguments, and environment, so these fields stay empty and are ignored.
fn spawn_run_options() -> RunOptions {
    let defaults = OpenOptions::default();
    RunOptions {
        backend: defaults.backend,
        program: String::new(),
        args: Vec::new(),
        profile: defaults.profile,
        style: defaults.style,
        cols: 80,
        rows: 24,
        cwd: None,
        env: Vec::new(),
        wait_ready: Some(false),
        restart: false,
        timeouts: defaults.timeouts,
        recording: AutomaticRecording {
            mode: AutomaticRecordingMode::Disabled,
            directory: None,
        },
    }
}

/// Launch descriptor for the re-executed fixture with one child mode.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn fixture_launch(mode: &str, extra_env: Vec<(String, String)>) -> LaunchOptions {
    let mut env = vec![(FIXTURE_ENV.to_string(), mode.to_string())];
    env.extend(extra_env);
    LaunchOptions::new(std::env::current_exe().expect("test binary path"))
        .with_args(["--exact", "d06_fixture", "--nocapture"])
        .with_envs(env)
}

/// Current screen text through the engine.
#[expect(clippy::expect_used, clippy::panic, reason = "test fixture setup")]
fn screen_text(engine: &Engine) -> String {
    match engine
        .execute(Operation::Text { full: false })
        .expect("screen text reads")
    {
        OperationResult::Text(text) => text,
        other => panic!("expected screen text, got {other:?}"),
    }
}

/// Bounded condition wait: polls screen text on a 10-millisecond interval
/// until the deadline. No wall-clock assumption beyond the timeout itself.
#[track_caller]
fn wait_for_text(engine: &Engine, needle: &str) {
    let started = Instant::now();
    while !screen_text(engine).contains(needle) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out waiting for {needle:?}",
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// C02: a live upstream session through the downstream sample path. The
/// runtime `Session` runs a fixture program, an upstream locator waits for
/// readiness, the owned snapshot becomes a downstream state-and-pixels
/// sample, and explicit approval turns the first missing-baseline failure
/// into a passing check.
#[test]
fn d06_c02_live_session_sample_through_downstream_path() {
    let dir = test_dir();
    let session = Session::new(format!("d06-c02-{}", std::process::id()));
    session
        .run(shell_options(
            "sh",
            &["-c", "printf 'live-ready 0123456789'; sleep 30"],
            Vec::new(),
        ))
        .expect("live session starts");
    session
        .get_by_text("live-ready")
        .expect_with(LocatorExpectOptions {
            not: false,
            timeout_ms: Some(10_000),
        })
        .expect("readiness text appears");
    let snapshot = session
        .capture_snapshot(SnapshotScope::ActiveScreen)
        .expect("live snapshot captures");
    session.close().expect("live session closes");
    assert_eq!(snapshot.cols(), 80);
    assert_eq!(snapshot.rows(), 24);

    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let test_id = format!(
        "views/d06-live/pixels-{}",
        tuiscotti::render_profile::platform_key()
    );
    let bytes = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)
        .expect("live sample builds")
        .to_bytes()
        .expect("live sample serializes");

    let baselines = dir.path().join("baselines");
    let artifacts = dir.path().join("artifacts");
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let failure = match store.check(&bytes).expect("first live check runs") {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => panic!("first check over empty baselines must fail"),
    };
    let approvals = ApprovalStore::new(&baselines).expect("approvals open");
    if std::env::var_os("CI").is_some() {
        // CI path: approval is refused by design, so the test probes that
        // refusal and finishes through a frozen fixture plus check-only
        // flow. The runner environment is never changed.
        match approvals.accept(
            failure.candidate_path(),
            failure.candidate_sha256(),
            MISSING_BASELINE,
        ) {
            Err(error @ ApproveError::CiApprovalForbidden) => {
                assert_eq!(error.code(), "CI_APPROVAL_FORBIDDEN");
                assert_eq!(error.exit_code(), 3);
            }
            other => panic!("live approval under CI must refuse, got {other:?}"),
        }
        let mut fixture = baselines.join(&test_id);
        fixture.set_extension("tui.json");
        if let Some(parent) = fixture.parent() {
            std::fs::create_dir_all(parent).expect("fixture parent creates");
        }
        std::fs::write(&fixture, &bytes).expect("frozen fixture writes");
        assert!(
            store
                .check(&bytes)
                .expect("second live check runs")
                .is_pass(),
            "frozen live sample must check green"
        );
        return;
    }
    approvals
        .accept(
            failure.candidate_path(),
            failure.candidate_sha256(),
            MISSING_BASELINE,
        )
        .expect("live candidate approves");
    assert!(
        store
            .check(&bytes)
            .expect("second live check runs")
            .is_pass(),
        "approved live sample must check green"
    );
}

fn left_button() -> MouseOptions {
    MouseOptions::new(MouseButton::Left)
}

/// C13: a checked mouse click reports exact SGR bytes, and an unpressed move
/// under click tracking fails before any byte write.
#[test]
fn d06_c13_checked_mouse_click_reports_exact_bytes() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "mouse");
    engine
        .spawn_with(fixture_launch("mouse", Vec::new()), spawn_run_options())
        .expect("mouse fixture spawns");
    wait_for_text(&engine, "READY");

    engine
        .mouse_checked(CheckedMouseAction::Press {
            x: 4,
            y: 2,
            options: left_button(),
        })
        .expect("press under click mode");
    // SGR-encoded press at 1-based (5, 3): ESC [ < 0 ; 5 ; 3 M.
    wait_for_text(&engine, "rx:1b5b3c303b353b334d");

    let error = engine
        .mouse_checked(CheckedMouseAction::Motion { x: 6, y: 3 })
        .expect_err("click mode rejects unpressed movement");
    assert!(
        matches!(error, CheckedInputError::Unsupported(_)),
        "rejection must be typed Unsupported, got {error:?}"
    );

    engine.execute(Operation::Close).expect("engine closes");
}

/// C13: paste wraps text in bracketed markers and focus reports reach the
/// child; focus without the mode fails before any byte write.
#[test]
fn d06_c13_paste_and_focus_report_exact_bytes() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "paste-focus");
    engine
        .spawn_with(
            fixture_launch("paste-focus", Vec::new()),
            spawn_run_options(),
        )
        .expect("paste fixture spawns");
    wait_for_text(&engine, "READY");

    engine.paste("hi").expect("paste with bracketed mode");
    // ESC [ 2 0 0 ~ h i ESC [ 2 0 1 ~, hex-encoded by the fixture.
    wait_for_text(&engine, "rx:1b5b3230307e68691b5b3230317e");

    engine.focus(true).expect("focus with focus mode");
    wait_for_text(&engine, "rx:1b5b49");

    engine.execute(Operation::Close).expect("engine closes");
}

/// C13: focus without the application enabling focus events is a typed
/// `Unsupported` failure with no bytes written.
#[test]
fn d06_c13_focus_without_mode_is_unsupported() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "focus-off");
    engine
        .spawn_with(fixture_launch("mouse", Vec::new()), spawn_run_options())
        .expect("no-focus fixture spawns");
    wait_for_text(&engine, "READY");

    let error = engine
        .focus(true)
        .expect_err("focus without the mode must fail");
    assert!(
        matches!(error, CheckedInputError::Unsupported(_)),
        "failure must be typed Unsupported, got {error:?}"
    );

    engine.execute(Operation::Close).expect("engine closes");
}

/// C14: a cleared launch carries only the explicit overrides plus transport
/// defaults; the inherited environment (PATH included) does not leak in.
#[test]
fn d06_c14_cleared_launch_keeps_only_overrides() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "cleared-env");
    let launch = fixture_launch(
        "print-env",
        vec![
            (SENTINEL.to_string(), "sentinel-value".to_string()),
            ("TUI_TEST_D06_KEYS".to_string(), format!("{SENTINEL},PATH")),
        ],
    )
    .with_env_mode(LaunchEnvMode::Cleared);
    engine
        .spawn_with(launch, spawn_run_options())
        .expect("cleared fixture spawns");
    engine
        .execute(Operation::WaitExit {
            timeout_ms: Some(10_000),
        })
        .expect("print-env fixture exits");
    let text = screen_text(&engine);
    assert!(
        text.contains("ENV:TUI_TEST_D06_SENTINEL=sentinel-value"),
        "explicit override must reach the child, screen was:\n{text}"
    );
    assert!(
        text.contains("ENV:PATH=<absent>"),
        "cleared launch must not leak the inherited PATH, screen was:\n{text}"
    );
}

/// C14: removal drops an inherited variable while later overrides apply,
/// and a restart keeps the resolved values.
#[test]
fn d06_c14_remove_override_restart_sequences() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "restart-env");
    let launch = fixture_launch(
        "print-env",
        vec![
            (SENTINEL.to_string(), "restart-value".to_string()),
            ("TUI_TEST_D06_KEYS".to_string(), format!("{SENTINEL},PATH")),
        ],
    )
    .with_remove_env("PATH");
    engine
        .spawn_with(launch, spawn_run_options())
        .expect("restart fixture spawns");
    engine
        .execute(Operation::WaitExit {
            timeout_ms: Some(10_000),
        })
        .expect("print-env fixture exits");
    let first = screen_text(&engine);
    assert!(
        first.contains("ENV:TUI_TEST_D06_SENTINEL=restart-value"),
        "override must reach the child, screen was:\n{first}"
    );
    assert!(
        first.contains("ENV:PATH=<absent>"),
        "removed variable must stay out, screen was:\n{first}"
    );

    engine
        .execute(Operation::Restart {
            graceful_timeout_ms: 5_000,
        })
        .expect("restart runs");
    engine
        .execute(Operation::WaitExit {
            timeout_ms: Some(10_000),
        })
        .expect("restarted fixture exits");
    let second = screen_text(&engine);
    assert!(
        second.contains("ENV:TUI_TEST_D06_SENTINEL=restart-value"),
        "restart must keep resolved values, screen was:\n{second}"
    );
}

/// Existing printf for native-argument echo, without shell parsing.
#[expect(clippy::panic, reason = "test fixture setup")]
fn printf_program() -> PathBuf {
    for candidate in ["/usr/bin/printf", "/bin/printf"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return path;
        }
    }
    panic!("no printf program for the native-arguments fixture");
}

/// C15: native arguments keep spaces, quotes, and shell metacharacters
/// exactly — no shell parsing and no Unicode replacement.
#[test]
fn d06_c15_native_arguments_keep_metacharacters() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "native-args");
    let launch = LaunchOptions::new(printf_program()).with_args([
        "%s\n",
        "sp ace",
        "qu'ote",
        "$HOME;$(x)",
        "--flag=value",
    ]);
    engine
        .spawn_with(launch, spawn_run_options())
        .expect("printf fixture spawns");
    engine
        .execute(Operation::WaitExit {
            timeout_ms: Some(10_000),
        })
        .expect("printf fixture exits");
    let text = screen_text(&engine);
    for expected in ["sp ace", "qu'ote", "$HOME;$(x)", "--flag=value"] {
        assert!(
            text.contains(expected),
            "native argument {expected:?} must survive verbatim, screen was:\n{text}"
        );
    }
}

/// C15: raw bytes keep NUL, escapes, and invalid UTF-8 through `write_bytes`.
#[test]
fn d06_c15_raw_bytes_keep_nul_and_invalid_utf8() {
    let dir = test_dir();
    let engine = test_engine(dir.path(), "raw-bytes");
    let out = dir.path().join("received.bin");
    // No signal, flow-control, or line-discipline bytes: cooked-mode stdin
    // delivers the line after its newline, and every byte below survives.
    let payload: Vec<u8> = b"\x00\x1b[Z\xff\xfe\xc3(A\n".to_vec();
    let launch = fixture_launch(
        "byte-echo",
        vec![
            (FIXTURE_OUT.to_string(), out.to_string_lossy().into_owned()),
            (FIXTURE_LEN.to_string(), payload.len().to_string()),
        ],
    );
    engine
        .spawn_with(launch, spawn_run_options())
        .expect("byte-echo fixture spawns");
    wait_for_text(&engine, "READY");
    engine
        .write_bytes(&payload)
        .expect("raw bytes write without UTF-8 conversion");

    let started = Instant::now();
    loop {
        match std::fs::read(&out) {
            Ok(bytes) if bytes == payload => break,
            Ok(_) => panic!("byte-echo fixture wrote corrupted bytes"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("byte-echo output unreadable: {error:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out waiting for the raw-byte output file"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// C16: a visual-quiet wait succeeds once the screen stops changing.
#[test]
fn d06_c16_visual_quiet_succeeds_on_quiet_screen() {
    let session = Session::new(format!("d06-c16-visual-{}", std::process::id()));
    session
        .run(shell_options(
            "sh",
            &["-c", "printf quiet-ready; sleep 30"],
            Vec::new(),
        ))
        .expect("quiet session starts");
    session
        .get_by_text("quiet-ready")
        .expect_with(LocatorExpectOptions {
            not: false,
            timeout_ms: Some(10_000),
        })
        .expect("readiness text appears");
    let options = WaitQuietOptions::new(Duration::from_secs(10), Duration::from_millis(300))
        .expect("quiet options validate");
    assert_eq!(options.mode(), WaitQuietMode::Visual);
    session
        .wait_quiet(&options)
        .expect("quiet screen passes a visual-quiet wait");
    session.close().expect("quiet session closes");
}

/// C16: a byte-quiet wait rides out continuous output, then succeeds; a
/// pre-cancelled wait reports cancellation; a quiet interval equal to the
/// overall timeout always reports timeout.
#[test]
fn d06_c16_byte_quiet_timeout_and_cancellation_bounds() {
    let session = Session::new(format!("d06-c16-bytes-{}", std::process::id()));
    session
        .run(shell_options(
            "sh",
            &[
                "-c",
                "for i in 1 2 3; do echo line; sleep 1; done; sleep 30",
            ],
            Vec::new(),
        ))
        .expect("burst session starts");
    let bytes_options = WaitQuietOptions::new(Duration::from_secs(20), Duration::from_secs(2))
        .expect("byte-quiet options validate")
        .with_mode(WaitQuietMode::Bytes);
    session
        .wait_quiet(&bytes_options)
        .expect("byte-quiet wait survives the burst");

    let cancelled = session.wait_quiet_cancellable(&bytes_options, &|| true);
    assert!(
        cancelled.is_err(),
        "a pre-cancelled quiet wait must report cancellation"
    );

    let deadline = WaitQuietOptions::new(Duration::from_millis(200), Duration::from_millis(200))
        .expect("equal-bound options validate");
    let timed_out = session.wait_quiet(&deadline);
    assert!(
        timed_out.is_err(),
        "a quiet interval equal to the timeout always reports timeout"
    );
    session.close().expect("burst session closes");
}

/// GOAL 17: concurrent attempts for one test ID keep every candidate
/// available. Two threads check different samples for the same ID against
/// one store; both failures persist with distinct candidate digests.
#[test]
fn d06_concurrent_attempts_keep_all_candidates() {
    use ratatui::widgets::Paragraph;
    use tuiscotti::view::{ViewSettings, capture_view};

    let dir = test_dir();
    let baselines = dir.path().join("baselines");
    let artifacts = dir.path().join("artifacts");
    let store = Arc::new(FrozenStore::new(&baselines, &artifacts).expect("store opens"));
    let barrier = Arc::new(Barrier::new(2));

    let worker = |text: &'static str, store: Arc<FrozenStore>, barrier: Arc<Barrier>| {
        thread::spawn(move || {
            let snapshot = capture_view(80, 24, &ViewSettings::builtin(), |frame| {
                frame.render_widget(Paragraph::new(text), frame.area());
            })
            .expect("concurrent view captures");
            let bytes = Sample::state("views/d06-concurrent", snapshot, Observation::empty())
                .expect("concurrent sample builds")
                .to_bytes()
                .expect("concurrent sample serializes");
            barrier.wait();
            let failure = match store.check(&bytes).expect("concurrent check runs") {
                CheckOutcome::Failure(failure) => failure,
                CheckOutcome::Pass(_) => panic!("concurrent check over empty baselines must fail"),
            };
            (
                failure.candidate_sha256().to_string(),
                failure.candidate_path().to_path_buf(),
            )
        })
    };

    let first = worker("candidate one", Arc::clone(&store), Arc::clone(&barrier));
    let second = worker("candidate two", Arc::clone(&store), Arc::clone(&barrier));
    let (first_digest, first_path) = first.join().expect("first attempt finishes");
    let (second_digest, second_path) = second.join().expect("second attempt finishes");
    assert_ne!(
        first_digest, second_digest,
        "different states stay distinct"
    );
    assert!(
        first_path.is_file(),
        "first candidate evidence stays available"
    );
    assert!(
        second_path.is_file(),
        "second candidate evidence stays available"
    );
    assert_ne!(first_path, second_path, "candidates never share one path");
}
