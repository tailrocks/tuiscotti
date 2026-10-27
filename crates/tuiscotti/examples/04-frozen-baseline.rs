//! 04: read-only baseline check with evidence outside the baseline root.
//!
//! Run: `cargo run --example 04-frozen-baseline`
//!
//! Frozen checking never writes to the baseline root: no baseline file, no
//! directory, no lock file. Failure evidence goes to a separate artifact
//! root that is not inside the baseline root. The baseline root below is
//! additionally made read-only (Unix), so any baseline write would fail at
//! the OS boundary instead of silently succeeding.
//!
//! Under `CI` approval is refused by design, so the setup plants the
//! baseline as a frozen fixture instead of approving; the marker and exit
//! code stay the same.

#[path = "common/ci_fixture.rs"]
mod ci_fixture;

use ratatui::Frame;
use ratatui::widgets::Paragraph;
use tuiscotti::approve::ApprovalStore;
use tuiscotti::check::{CheckOutcome, FrozenStore, MISSING_BASELINE};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view};

fn frozen_view(frame: &mut Frame<'_>) {
    frame.render_widget(Paragraph::new("frozen content"), frame.area());
}

fn changed_view(frame: &mut Frame<'_>) {
    frame.render_widget(Paragraph::new("changed content"), frame.area());
}

fn draw(draw_fn: impl FnOnce(&mut Frame<'_>)) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let snapshot = capture_view(80, 24, &ViewSettings::builtin(), draw_fn)?;
    let profile = resolve_profile(PROFILE_NAME)?;
    let test_id = format!("examples/frozen-baseline/pixels-{}", platform_key());
    Ok(Sample::with_pixels(&test_id, &snapshot, Observation::empty(), profile)?.to_bytes()?)
}

#[cfg(unix)]
fn make_read_only(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o555);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_read_only(_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");

    let approved = draw(frozen_view)?;
    let setup_store = FrozenStore::new(&baselines, &artifacts)?;
    let failure = match setup_store.check(&approved)? {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => return Err("setup check over empty baselines must fail".into()),
    };
    if ci_fixture::ci_is_set() {
        let test_id = format!("examples/frozen-baseline/pixels-{}", platform_key());
        ci_fixture::plant_frozen_baseline(&baselines, &test_id, &approved)?;
    } else {
        ApprovalStore::new(&baselines)?.accept(
            failure.candidate_path(),
            failure.candidate_sha256(),
            MISSING_BASELINE,
        )?;
    }
    make_read_only(&baselines)?;
    let listing_before = baseline_listing(&baselines)?;

    let store = FrozenStore::new(&baselines, &artifacts)?;
    let rerun = store.check(&approved)?;
    assert!(rerun.is_pass(), "equal sample must pass over a frozen root");

    let changed = draw(changed_view)?;
    let failure = match store.check(&changed)? {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => return Err("changed sample must fail".into()),
    };
    assert!(
        !failure.candidate_path().starts_with(&baselines),
        "evidence must stay outside the baseline root"
    );
    assert_eq!(
        baseline_listing(&baselines)?,
        listing_before,
        "frozen checking must not write to the baseline root"
    );

    println!("EXAMPLE-04-OK id=examples/frozen-baseline");
    Ok(())
}

fn baseline_listing(
    baselines: &std::path::Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names: Vec<String> = std::fs::read_dir(baselines)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    names.sort();
    Ok(names)
}
