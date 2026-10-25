//! 02: live Rust TUI test with direct upstream session control.
//!
//! Run: `cargo run --example 02-live-session`
//!
//! The live session is a `tui_test::Session` driven directly: no tuiscotti
//! session wrapper, no tuiscotti locator engine, no tuiscotti input helper.
//! Upstream locators find the readiness text, the owned snapshot converts
//! through the same downstream sample API the pure-view example uses
//! (`Sample`, `FrozenStore`).

use tui_test::{
    AutomaticRecording, AutomaticRecordingMode, LocatorExpectOptions, OpenOptions, RunOptions,
    Session, SnapshotScope, Timeouts,
};
use tuiscotti::approve::ApprovalStore;
use tuiscotti::check::{CheckOutcome, FrozenStore, MISSING_BASELINE};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};

fn run_options() -> RunOptions {
    let defaults = OpenOptions::default();
    RunOptions {
        backend: defaults.backend,
        program: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            "printf 'live-ready 0123456789'; sleep 30".to_string(),
        ],
        profile: defaults.profile,
        style: defaults.style,
        cols: 80,
        rows: 24,
        cwd: None,
        env: Vec::new(),
        wait_ready: Some(false),
        restart: false,
        timeouts: Timeouts::default(),
        recording: AutomaticRecording {
            mode: AutomaticRecordingMode::Disabled,
            directory: None,
        },
    }
}

#[cfg(not(unix))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("EXAMPLE-02-SKIP no Unix PTY on this platform");
    Ok(())
}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new(format!("example-live-{}", std::process::id()));
    session.run(run_options())?;
    // Bounded locator wait: the readiness text must appear, no sleeps.
    session
        .get_by_text("live-ready")
        .expect_with(LocatorExpectOptions {
            not: false,
            timeout_ms: Some(10_000),
        })?;

    let snapshot = session.capture_snapshot(SnapshotScope::ActiveScreen)?;
    session.close()?;

    let profile = resolve_profile(PROFILE_NAME)?;
    let test_id = format!("examples/live-session/pixels-{}", platform_key());
    let sample = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)?;
    let bytes = sample.to_bytes()?;

    let root = tempfile::tempdir()?;
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let store = FrozenStore::new(&baselines, &artifacts)?;
    let approvals = ApprovalStore::new(&baselines)?;
    match store.check(&bytes)? {
        CheckOutcome::Pass(_) => {}
        CheckOutcome::Failure(failure) => {
            approvals.accept(
                failure.candidate_path(),
                failure.candidate_sha256(),
                MISSING_BASELINE,
            )?;
            let rerun = store.check(&bytes)?;
            assert!(rerun.is_pass(), "approved live sample must check green");
        }
    }
    println!("EXAMPLE-02-OK id=examples/live-session");
    Ok(())
}
