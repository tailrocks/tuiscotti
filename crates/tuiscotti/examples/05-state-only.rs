//! 05: state-only sample for a test with no pixel requirement.
//!
//! Run: `cargo run --example 05-state-only`
//!
//! A state sample carries the snapshot with no render contract and no image:
//! nothing renders, so layout-only assertions check without a pixel gate.
//! Observation metadata stays evidence-only: re-noting the same state
//! changes the candidate digest but still checks green.

use ratatui::Frame;
use ratatui::widgets::Paragraph;
use tuiscotti::approve::ApprovalStore;
use tuiscotti::check::{CheckOutcome, FrozenStore, MISSING_BASELINE};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

fn layout_view(frame: &mut Frame<'_>) {
    frame.render_widget(Paragraph::new("layout only"), frame.area());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), layout_view)?;
    let sample = Sample::state("examples/state-only", snapshot, Observation::empty())?;
    assert!(
        sample.render_contract().is_none(),
        "state sample carries no contract"
    );
    assert!(sample.image().is_none(), "state sample carries no image");
    let bytes = sample.to_bytes()?;

    let root = tempfile::tempdir()?;
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let store = FrozenStore::new(&baselines, &artifacts)?;
    let failure = match store.check(&bytes)? {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => return Err("first check over empty baselines must fail".into()),
    };
    ApprovalStore::new(&baselines)?.accept(
        failure.candidate_path(),
        failure.candidate_sha256(),
        MISSING_BASELINE,
    )?;
    assert!(
        store.check(&bytes)?.is_pass(),
        "approved state sample must pass"
    );

    // A re-noted observation is a different candidate with equal state.
    let renoted_snapshot = capture_view(80, 24, &ViewSettings::builtin(), layout_view)?;
    let renoted = Sample::state(
        "examples/state-only",
        renoted_snapshot,
        Observation::noted("reviewer note"),
    )?;
    let renoted_bytes = renoted.to_bytes()?;
    assert_ne!(
        renoted_bytes, bytes,
        "observation must change the candidate ID"
    );
    assert!(
        store.check(&renoted_bytes)?.is_pass(),
        "equal state with different observation must still pass"
    );

    println!("EXAMPLE-05-OK id=examples/state-only");
    Ok(())
}
