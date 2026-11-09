use std::path::Path;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use tuiscotti::view::{ViewError, ViewSettings, snapshot_from_buffer};

use super::helpers::{
    ChildEnv, accept_candidate, baseline_samples, case_dir, ci_lane, exit_code, log_sample,
    make_dir, open_lane, parse_json_line, pixel_test_id, run_cli, sha256_file, utf8, write_bytes,
};

#[path = "dep_rules.rs"]
mod dep_rules;

pub(super) fn invalid_input() {
    const OVERSIZED: usize = 64 * 1024 * 1024 + 1;
    let root = case_dir("invalid-input");
    let baselines = root.path().join("baselines");
    let artifacts = root.path().join("artifacts");
    let candidates = root.path().join("candidates");
    make_dir(&candidates);
    let lane = ci_lane();
    let sample = candidates.join("good.tui.pb");
    write_bytes(
        &sample,
        &log_sample(&pixel_test_id("invalid-input"), 30, 0, 8),
    );
    phase_legacy_json_baseline(&sample, &baselines, &artifacts, &lane);
    let missing = candidates.join("absent.tui.pb");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&missing),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    assert_eq!(parse_json_line(&out.stdout)["error_code"], "IO");
    let corrupt = candidates.join("corrupt.tui.json");
    write_bytes(&corrupt, b"{\"schema\": \"tuiscotti.sample.v9\"}");
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&corrupt),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    let rejected = parse_json_line(&out.stdout);
    assert!(rejected["error_code"].is_string());
    assert_eq!(rejected["candidate_sha256"], sha256_file(&corrupt));
    let huge = candidates.join("huge.tui.pb");
    write_bytes(&huge, &vec![b'x'; OVERSIZED]);
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(&huge),
            "--baselines",
            utf8(&baselines),
            "--artifacts",
            utf8(&artifacts),
            "--json",
        ],
        &lane,
    );
    assert_eq!(exit_code(&out), 3);
    assert_eq!(parse_json_line(&out.stdout)["error_code"], "RESOURCE_LIMIT");
    phase_usage_errors(&candidates, &sample, &baselines, &lane);
    dep_rules::phase_direct_dep_rules(&candidates);
    #[cfg(unix)]
    approval_refuses_symlink_lock_without_external_write();
    phase_view_error_coordinates();
    assert_eq!(baseline_samples(&baselines), [] as [std::path::PathBuf; 0]);
}

fn phase_legacy_json_baseline(
    sample: &Path,
    baselines: &Path,
    artifacts: &Path,
    lane: &ChildEnv<'_>,
) {
    let legacy_id = pixel_test_id("invalid-input");
    let legacy_baseline = baselines.join(format!("{legacy_id}.tui.json"));
    make_dir(&baselines.join("functional/invalid-input"));
    let legacy_bytes = b"{\"schema\":\"tuiscotti.sample.v1\"}";
    write_bytes(&legacy_baseline, legacy_bytes);
    let out = run_cli(
        &[
            "check",
            "--sample",
            utf8(sample),
            "--baselines",
            utf8(baselines),
            "--artifacts",
            utf8(artifacts),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 1);
    assert_eq!(parse_json_line(&out.stdout)["status"], "missing");
    assert!(std::fs::read(&legacy_baseline).is_ok_and(|bytes| bytes == legacy_bytes));
    assert_eq!(baseline_samples(baselines), [] as [std::path::PathBuf; 0]);
}

#[cfg(unix)]
fn approval_refuses_symlink_lock_without_external_write() {
    let lane = open_lane();
    let root = case_dir("approval-lock-regular");
    let baselines = root.path().join("baselines");
    let candidates = root.path().join("candidates");
    make_dir(&baselines);
    make_dir(&candidates);
    let lock = baselines.join(".tuiscotti-approval.lock");
    write_bytes(&lock, b"held");
    let candidate = candidates.join("regular.tui.pb");
    write_bytes(
        &candidate,
        &log_sample(&pixel_test_id("approval-lock-regular"), 30, 0, 8),
    );
    let digest = sha256_file(&candidate);
    let (code, _) = accept_candidate(&candidate, digest.as_str(), "missing", &baselines, &lane);
    assert_eq!(code, 0);
    assert_eq!(baseline_samples(&baselines).len(), 1);
    assert!(lock.is_file());
    assert_eq!(std::fs::read(&lock).unwrap_or_default(), b"held");
    let root = case_dir("approval-lock-symlink");
    let baselines = root.path().join("baselines");
    let candidates = root.path().join("candidates");
    let outside = root.path().join("outside");
    make_dir(&baselines);
    make_dir(&candidates);
    make_dir(&outside);
    let target = outside.join("external.lock");
    let link = baselines.join(".tuiscotti-approval.lock");
    assert!(std::os::unix::fs::symlink(&target, &link).is_ok());
    let candidate = candidates.join("candidate.tui.pb");
    write_bytes(
        &candidate,
        &log_sample(&pixel_test_id("approval-lock-symlink"), 30, 0, 8),
    );
    let digest = sha256_file(&candidate);
    let (code, stdout) =
        accept_candidate(&candidate, digest.as_str(), "missing", &baselines, &lane);
    assert_eq!(code, 3);
    assert_eq!(parse_json_line(&stdout)["error_code"], "INVALID_STORE_PATH");
    assert!(!target.exists());
    assert_eq!(baseline_samples(&baselines), [] as [std::path::PathBuf; 0]);
    assert!(std::fs::symlink_metadata(&link).is_ok_and(|meta| meta.file_type().is_symlink()));
}

fn phase_usage_errors(candidates: &Path, sample: &Path, baselines: &Path, lane: &ChildEnv<'_>) {
    let out = run_cli(
        &[
            "accept",
            "--candidate",
            utf8(sample),
            "--candidate-sha256",
            "xyz",
            "--expect-baseline",
            "missing",
            "--baselines",
            utf8(baselines),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 2);
    assert_eq!(out.stdout, [] as [u8; 0]);
    let bad_out = candidates.join("render.bad");
    let out = run_cli(
        &[
            "render",
            "--sample",
            utf8(sample),
            "--out",
            utf8(&bad_out),
            "--json",
        ],
        lane,
    );
    assert_eq!(exit_code(&out), 2);
    assert_eq!(out.stdout, [] as [u8; 0]);
}

#[expect(
    clippy::expect_used,
    reason = "constructed buffers contain their cells"
)]
fn phase_view_error_coordinates() {
    let settings = ViewSettings::builtin();
    let mut valid = Buffer::empty(Rect::new(10, 20, 2, 1));
    valid
        .cell_mut(Position::new(10, 20))
        .expect("valid buffer contains its leading cell")
        .set_symbol("あ");
    let snapshot = snapshot_from_buffer(&valid, &settings).expect("wide cell plus space converts");
    assert_eq!(snapshot.cols(), 2);
    assert_eq!(snapshot.rows(), 1);
    let mut occupied = Buffer::empty(Rect::new(10, 20, 2, 1));
    occupied
        .cell_mut(Position::new(10, 20))
        .expect("occupied buffer contains its leading cell")
        .set_symbol("あ");
    occupied
        .cell_mut(Position::new(11, 20))
        .expect("occupied buffer contains its covered cell")
        .set_symbol("x");
    assert!(matches!(
        snapshot_from_buffer(&occupied, &settings),
        Err(ViewError::CoveredCellOccupied { x: 1, y: 0 })
    ));
    let mut hollow = Buffer::empty(Rect::new(10, 20, 1, 1));
    hollow
        .cell_mut(Position::new(10, 20))
        .expect("hollow buffer contains its cell")
        .set_symbol("");
    assert!(matches!(
        snapshot_from_buffer(&hollow, &settings),
        Err(ViewError::UncoveredZeroWidthCell { x: 0, y: 0 })
    ));
    let mut overhang = Buffer::empty(Rect::new(10, 20, 1, 1));
    overhang
        .cell_mut(Position::new(10, 20))
        .expect("overhang buffer contains its leading cell")
        .set_symbol("あ");
    assert!(matches!(
        snapshot_from_buffer(&overhang, &settings),
        Err(ViewError::LeadingCellOverhang {
            x: 0,
            y: 0,
            width: 2
        })
    ));
    let mut leading = Cell::default();
    leading.set_symbol("a");
    let truncated = Buffer {
        area: Rect::new(10, 20, 2, 1),
        content: vec![leading],
    };
    assert!(matches!(
        snapshot_from_buffer(&truncated, &settings),
        Err(ViewError::CellOutOfBounds { x: 1, y: 0 })
    ));
}
