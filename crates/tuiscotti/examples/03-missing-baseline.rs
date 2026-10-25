//! 03: missing-baseline failure and explicit candidate approval.
//!
//! Run: `cargo run --example 03-missing-baseline`
//!
//! A check against an empty baseline root fails without writing a baseline:
//! the candidate and its evidence land under the artifact root, never under
//! the baseline root. Approval then takes the saved candidate path with its
//! digest, and the same bytes check green afterwards.

use ratatui::Frame;
use ratatui::widgets::Paragraph;
use tuiscotti::approve::ApprovalStore;
use tuiscotti::check::{CheckOutcome, FrozenStore, MISSING_BASELINE, MismatchReason};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

fn styled_view(frame: &mut Frame<'_>) {
    use ratatui::style::{Color, Modifier, Style};
    let styled = Paragraph::new("approve me").style(
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(styled, frame.area());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), styled_view)?;
    let profile = resolve_profile(PROFILE_NAME)?;
    let test_id = format!("examples/missing-baseline/pixels-{}", platform_key());
    let sample = Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)?;
    let bytes = sample.to_bytes()?;

    let root = tempfile::tempdir()?;
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let store = FrozenStore::new(&baselines, &artifacts)?;

    // The missing-baseline failure: no baseline is written.
    let failure = match store.check(&bytes)? {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => return Err("check over empty baselines must fail".into()),
    };
    assert_eq!(failure.reason(), MismatchReason::Missing);
    assert_eq!(failure.baseline_sha256(), MISSING_BASELINE);
    assert!(
        failure.candidate_path().is_file(),
        "candidate evidence must be saved under the artifact root"
    );
    assert!(
        !failure.candidate_path().starts_with(&baselines),
        "candidate evidence must never land under the baseline root"
    );
    // A missing baseline root lists as empty: the missing failure must not
    // create the root, let alone a baseline file.
    let baselines_before: Vec<_> = match std::fs::read_dir(&baselines) {
        Ok(entries) => entries
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<_, _>>()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    assert!(
        baselines_before.is_empty(),
        "missing baseline must not create a baseline file"
    );

    // Explicit candidate approval, then the same bytes check green.
    let approvals = ApprovalStore::new(&baselines)?;
    let outcome = approvals.accept(
        failure.candidate_path(),
        failure.candidate_sha256(),
        MISSING_BASELINE,
    )?;
    assert_eq!(outcome.test_id(), test_id);
    let rerun = store.check(&bytes)?;
    assert!(rerun.is_pass(), "approved candidate must check green");

    println!("EXAMPLE-03-OK id=examples/missing-baseline");
    Ok(())
}
