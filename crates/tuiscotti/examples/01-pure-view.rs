//! 01: pure Ratatui view test with the production draw function.
//!
//! Run: `cargo run --example 01-pure-view`
//!
//! A pure-view test renders an application view in the test process. It
//! starts no shell, child process, PTY, daemon, or terminal session: the
//! only inputs are the production draw closure below and the caller-supplied
//! view settings. The converted snapshot travels the same downstream sample
//! API the live-session example uses (`Sample`, `FrozenStore`).
//!
//! Under `CI` approval is refused by design, so the example plants the
//! candidate as a frozen baseline fixture and checks it green instead of
//! approving; the marker and exit code stay the same.

#[path = "common/ci_fixture.rs"]
mod ci_fixture;

use ratatui::Frame;
use ratatui::layout::Position;
use ratatui::widgets::Paragraph;
use tuiscotti::check::FrozenStore;
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

/// Production draw function: the real render path, never a test-only grid.
fn login_view(frame: &mut Frame<'_>) {
    frame.render_widget(Paragraph::new("hello tuiscotti"), frame.area());
    frame.set_cursor_position(Position::new(16, 2));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), login_view)?;
    let profile = resolve_profile(PROFILE_NAME)?;
    let test_id = format!("examples/pure-view/pixels-{}", platform_key());
    let sample = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)?;
    let bytes = sample.to_bytes()?;

    let root = tempfile::tempdir()?;
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let store = FrozenStore::new(&baselines, &artifacts)?;
    let outcome = store.check(&bytes)?;
    assert!(
        !outcome.is_pass(),
        "first check over an empty baseline root must fail"
    );

    let failure = match outcome {
        tuiscotti::check::CheckOutcome::Failure(failure) => failure,
        tuiscotti::check::CheckOutcome::Pass(_) => {
            return Err("first check unexpectedly passed".into());
        }
    };
    if ci_fixture::ci_is_set() {
        ci_fixture::plant_frozen_baseline(&baselines, &test_id, &bytes)?;
    } else {
        let approvals = tuiscotti::approve::ApprovalStore::new(&baselines)?;
        approvals.accept(
            failure.candidate_path(),
            failure.candidate_sha256(),
            tuiscotti::check::MISSING_BASELINE,
        )?;
    }
    let rerun = store.check(&bytes)?;
    assert!(
        rerun.is_pass(),
        "approved pure-view sample must check green"
    );

    println!("EXAMPLE-01-OK id=examples/pure-view");
    Ok(())
}
