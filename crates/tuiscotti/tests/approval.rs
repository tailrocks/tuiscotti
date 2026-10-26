//! D04 approval tests: prepared transactions, digest arbitration, and frozen
//! checking.
//!
//! Covers the D04 subcases:
//!
//! - C09: a missing baseline fails the frozen check without a baseline
//!   write; the prepared accept transaction then writes the baseline,
//!   including a first approval under an initially missing baseline root.
//! - C10: candidate and baseline digests agree with the supplied values, and
//!   frame and image data agree; image samples re-render through the
//!   specified profile (unknown profiles and pixel mismatches reject).
//! - C11: a stale expected digest is rejected with the baseline left
//!   untouched; concurrent accepts let at most one conflicting replacement
//!   succeed, in threads and across child processes; a fault before
//!   replacement keeps the previous baseline.
//! - C12: the frozen store never writes (zero new files, no lock file); CI
//!   approval is refused in a dedicated child process per value and writes
//!   nothing; frozen-tree and frozen-store acceptance are both refused.
//!
//! Also: digest-mixture rejections, accept-to-accept idempotence, corrupt
//! versus missing candidates, tamper-evident candidates, read-only roots,
//! the Section 16.3 code table, the stable-lock lifecycle (kept file, stale
//! residue, cross-process deadline), and volatile-free evidence records.
//! Barriers order thread concurrency; tempdirs isolate the filesystem; no
//! test changes process-wide state (CI values and contender roles live only
//! in spawned child processes).
//! The suite runs green with and without `CI` set: any set `CI` freezes
//! every public approval call by design, so each approval-writing test
//! probes that refusal through [`ci_refusal_probed`] and returns early,
//! while the transaction mechanics under `CI` stay covered by the private
//! unit-tested core in `src/approve.rs` (which never reads the gate).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use fs2::FileExt as _;
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
    SnapshotParts, SnapshotScope, TerminalSnapshot,
};
use tuiscotti::approve::{ApprovalStore, ApproveError};
use tuiscotti::check::{CheckOutcome, FrozenStore, MismatchReason};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::ViewSettings;

/// Built-in view palette: the fixed migration record, no child or session.
fn test_palette() -> [Rgb; 256] {
    *ViewSettings::builtin().palette()
}

/// Hidden cursor fixture: keeps reports focused on cell state.
fn hidden_cursor() -> SnapshotCursor {
    SnapshotCursor::new(
        0,
        0,
        true,
        false,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
        None,
    )
}

/// One plain cell fixture.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn plain_cell(text: &str, width: u16) -> SnapshotCell {
    SnapshotCell::new(
        text.to_string(),
        width,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        tui_test::SnapshotUnderline::None,
        SnapshotColor::Default,
        None,
        None,
    )
    .expect("fixture cell is valid")
}

/// A 4-column, 2-row grid filled with one grapheme.
fn filled_cells(fill: &str) -> Vec<SnapshotCell> {
    let mut cells = Vec::new();
    for _ in 0..8 {
        cells.push(plain_cell(fill, 1));
    }
    cells
}

/// Snapshot fixture with default observation metadata.
fn base_snapshot() -> TerminalSnapshot {
    snapshot_with_cells(filled_cells("a"), SnapshotMetadata::default())
}

/// Snapshot fixture with explicit cells and metadata.
fn snapshot_with_cells(cells: Vec<SnapshotCell>, metadata: SnapshotMetadata) -> TerminalSnapshot {
    snapshot_with_grid(4, 2, cells, metadata)
}

/// Snapshot fixture with an explicit grid size.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn snapshot_with_grid(
    cols: u32,
    rows: u32,
    cells: Vec<SnapshotCell>,
    metadata: SnapshotMetadata,
) -> TerminalSnapshot {
    TerminalSnapshot::try_new(SnapshotParts {
        cols,
        rows,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells,
        palette: test_palette(),
        default_foreground: Rgb::new(192, 192, 192),
        default_background: Rgb::new(0, 0, 0),
        default_cursor: Rgb::new(192, 192, 192),
        cursor: hidden_cursor(),
        title: None,
        metadata,
    })
    .expect("fixture snapshot is valid")
}

/// State sample bytes for a test ID.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn state_sample_bytes(test_id: &str, snapshot: &TerminalSnapshot) -> Vec<u8> {
    Sample::state(test_id, snapshot.clone(), Observation::empty())
        .expect("fixture state sample is valid")
        .to_bytes()
        .expect("fixture state sample serializes")
}

/// State-and-pixels sample bytes for a logical test ID on this platform.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn pixel_sample_bytes(logical_id: &str, snapshot: &TerminalSnapshot) -> Vec<u8> {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    Sample::with_pixels(
        &format!("{logical_id}/pixels-{}", platform_key()),
        snapshot,
        Observation::empty(),
        profile,
    )
    .expect("fixture pixel sample is valid")
    .to_bytes()
    .expect("fixture pixel sample serializes")
}

/// Rewrite one JSON sample's render-contract profile name.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_profile(bytes: &[u8], profile: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    value["render_contract"]["profile"] = serde_json::json!(profile);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// Rewrite one JSON sample's top-level schema identifier.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_schema(bytes: &[u8], schema: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    value["schema"] = serde_json::json!(schema);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// Set the first snapshot cell's text in a JSON sample, leaving the image
/// evidence untouched so structural validation still passes.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_first_cell_text(bytes: &[u8], text: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    value["snapshot"]["cells"][0]["text"] = serde_json::json!(text);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// Rewrite one JSON sample's test ID, leaving everything else untouched.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_test_id(bytes: &[u8], test_id: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    value["test_id"] = serde_json::json!(test_id);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// Set the claimed image dimensions in a JSON sample, leaving the PNG
/// bytes and digests untouched. Dimension limits are checked before the
/// bytes decode, so an over-limit claim rejects as a resource limit.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_image_dims(bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    value["image"]["width"] = serde_json::json!(width);
    value["image"]["height"] = serde_json::json!(height);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// The stable per-root approval lock file, kept after every transaction.
fn lock_path(root: &Path) -> PathBuf {
    root.join(".tuiscotti-approval.lock")
}

/// Fresh tempdir with baseline and artifact roots.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn temp_store() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    let artifacts = dir.path().join("artifacts");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    (dir, baselines, artifacts)
}

/// Write baseline bytes for a test ID, making parent directories.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn write_baseline_file(root: &Path, test_id: &str, bytes: &[u8]) {
    let mut path = root.join(test_id);
    path.set_extension("tui.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("baseline parent creates");
    }
    std::fs::write(&path, bytes).expect("baseline writes");
}

/// Write candidate bytes to a file and return its path.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn candidate_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("candidate writes");
    path
}

/// Baseline bytes on disk for a test ID.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn read_baseline_file(root: &Path, test_id: &str) -> Vec<u8> {
    let mut path = root.join(test_id);
    path.set_extension("tui.json");
    std::fs::read(&path).expect("baseline reads")
}

/// Sorted file names directly inside a directory.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn entry_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("root lists")
        .map(|entry| {
            entry
                .expect("entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Every file under a root, relative and sorted, for zero-write assertions.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn all_files_below(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("tree lists") {
            let entry = entry.expect("entry reads");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(
                    path.strip_prefix(root)
                        .expect("path stays below root")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    found.sort();
    found
}

/// SHA-256 hex over raw bytes, matching the candidate and baseline keying.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

/// Unwrap a failing check outcome for evidence assertions.
#[expect(
    clippy::panic,
    reason = "test helper rejects an impossible passing outcome"
)]
fn failure_of(outcome: CheckOutcome) -> tuiscotti::check::CheckFailure {
    match outcome {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => panic!("check passed but the test needs a failure"),
    }
}

/// Unwrap a passing check outcome for digest assertions.
#[expect(
    clippy::panic,
    reason = "test helper rejects an impossible failing outcome"
)]
fn success_of(outcome: CheckOutcome) -> tuiscotti::check::CheckSuccess {
    match outcome {
        CheckOutcome::Pass(success) => success,
        CheckOutcome::Failure(_) => panic!("check failed but the test needs a pass"),
    }
}

/// Unwrap a successful approval for digest assertions.
#[expect(clippy::panic, reason = "test helper rejects a refused approval")]
fn accepted_of(
    outcome: Result<tuiscotti::approve::AcceptOutcome, ApproveError>,
) -> tuiscotti::approve::AcceptOutcome {
    match outcome {
        Ok(accepted) => accepted,
        Err(error) => panic!("approval failed but the test needs success: {error}"),
    }
}

/// True when the `CI` variable is present with any value, including empty:
/// the public approval API refuses every call by design.
fn ci_is_set() -> bool {
    std::env::var_os("CI").is_some()
}

/// File listing that tolerates a missing root: refusal happens before the
/// root is created, so a missing root lists as empty.
fn files_below_tolerant(root: &Path) -> Vec<String> {
    if !root.exists() {
        return Vec::new();
    }
    all_files_below(root)
}

/// CI probe for approval-writing tests: when `CI` is set, the public
/// `accept` call below must refuse with `CI_APPROVAL_FORBIDDEN` and write
/// nothing (the exit-3 mapping lives at the CLI boundary), so the probe
/// asserts that refusal and returns true; the caller returns immediately.
/// Without `CI` it returns false and the test runs its transaction
/// mechanics. The probe never changes the runner environment.
#[expect(clippy::panic, reason = "test helper rejects a non-refusing approval")]
fn ci_refusal_probed(
    store: &ApprovalStore,
    candidate: &Path,
    candidate_digest: &str,
    expected_baseline: &str,
) -> bool {
    if !ci_is_set() {
        return false;
    }
    let before = files_below_tolerant(store.baseline_root());
    match store.accept(candidate, candidate_digest, expected_baseline) {
        Err(error @ ApproveError::CiApprovalForbidden) => {
            assert_eq!(error.code(), "CI_APPROVAL_FORBIDDEN");
        }
        other => panic!("approval under CI must refuse, got {other:?}"),
    }
    assert_eq!(
        files_below_tolerant(store.baseline_root()),
        before,
        "CI refusal writes nothing"
    );
    true
}
/// Rewrite a pixels candidate's stored contract identity to a foreign
/// platform, keeping its own test ID in agreement with that contract.
#[expect(clippy::expect_used, reason = "rework validation fixture")]
fn with_foreign_platform(bytes: &[u8], platform: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    let old_id = value
        .get("test_id")
        .and_then(|id| id.as_str())
        .expect("candidate carries a test ID")
        .to_string();
    let logical = old_id
        .rsplit_once("/pixels-")
        .expect("candidate carries a pixels suffix")
        .0
        .to_string();
    value["render_contract"]["platform_key"] = serde_json::json!(platform);
    value["test_id"] = serde_json::json!(format!("{logical}/pixels-{platform}"));
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// R21: approval of a sample recorded on another platform does not trust
/// the stored pixels. Parsing validates the ID against the stored
/// contract identity, and approval separately attempts exact reproduction
/// by re-rendering the snapshot on the current host.
#[test]
fn r21_foreign_platform_candidate_approves_through_local_rerender() {
    let (temp, baselines, _artifacts) = temp_store();
    let local = pixel_sample_bytes("r21/login", &base_snapshot());
    let candidate_bytes = with_foreign_platform(&local, "far-far-away");
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.test_id(), "r21/login/pixels-far-far-away");
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(
        read_baseline_file(&baselines, "r21/login/pixels-far-far-away"),
        candidate_bytes
    );
}

/// C09: a missing baseline fails the frozen check without a baseline write,
/// then the prepared accept transaction writes the baseline.
#[test]
fn d04_c09_missing_baseline_approval_writes_baseline() {
    let (temp, baselines, artifacts) = temp_store();
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let failure = failure_of(frozen.check(&candidate_bytes).expect("check runs"));
    assert_eq!(failure.candidate_sha256(), candidate_digest);
    assert_eq!(failure.baseline_sha256(), "missing");
    assert_eq!(entry_names(&baselines), Vec::<String>::new());
    let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.test_id(), "views/login");
    assert_eq!(accepted.candidate_sha256(), candidate_digest);
    assert_eq!(accepted.previous_baseline_sha256(), "missing");
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
    assert!(accepted.baseline_path().is_file());
    assert_eq!(accepted.reason(), Some(MismatchReason::Missing));
    assert_eq!(accepted.changed_fields(), &["baseline".to_string()]);
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            "views/login.tui.json".to_string(),
        ],
        "a success writes exactly the baseline plus the stable lock file: \
         no evidence directory, no temporary files"
    );
    assert!(
        lock_path(&baselines).is_file(),
        "the stable lock file persists after the transaction"
    );
    let success = success_of(frozen.check(&candidate_bytes).expect("second check runs"));
    assert_eq!(success.baseline_sha256(), candidate_digest);
}

/// C09: a first approval for `views/login` creates an initially missing
/// baseline root and writes the baseline there.
#[test]
fn d04_c09_first_approval_under_missing_root() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    assert!(!baselines.exists(), "baseline root starts missing");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
}

/// C10: candidate and baseline digests agree with the supplied values, and
/// the frame and image digests agree with the stored bytes.
#[test]
fn d04_c10_digest_and_frame_image_agreement() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let candidate_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.test_id(), test_id);
    assert_eq!(accepted.candidate_sha256(), candidate_digest);
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(
        accepted.baseline_sha256(),
        sha256_hex(&read_baseline_file(&baselines, &test_id))
    );
    let sample = Sample::from_bytes(&candidate_bytes).expect("candidate parses");
    let image = sample.image().expect("pixel sample carries an image");
    let stored =
        Sample::from_bytes(&read_baseline_file(&baselines, &test_id)).expect("baseline parses");
    let stored_image = stored.image().expect("baseline carries an image");
    assert_eq!(stored_image.png_sha256(), image.png_sha256());
    assert_eq!(stored_image.rgba_sha256(), image.rgba_sha256());
    // The change summary rides in the outcome: a first approval reports the
    // missing baseline, and nothing but the baseline is persisted.
    assert_eq!(accepted.previous_baseline_sha256(), "missing");
    assert_eq!(accepted.reason(), Some(MismatchReason::Missing));
    assert_eq!(accepted.changed_fields(), &["baseline".to_string()]);
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            format!("{test_id}.tui.json"),
        ],
        "a success writes exactly the baseline plus the stable lock file"
    );
}

/// C10: approval re-renders through the candidate's specified profile: an
/// unresolvable profile rejects with `UNKNOWN_PROFILE` and writes nothing,
/// even though the stored frame and image digests validate.
#[test]
fn d04_c10_unknown_profile_rejected() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let first_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    let first = candidate_file(dir.path(), "first.tui.json", &first_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &first, &first_digest, "missing") {
        return;
    }
    accepted_of(store.accept(&first, &first_digest, "missing"));
    let second_bytes = with_profile(&first_bytes, "no-such-profile");
    assert_ne!(sha256_hex(&second_bytes), first_digest);
    let second_digest = sha256_hex(&second_bytes);
    let second = candidate_file(dir.path(), "second.tui.json", &second_bytes);
    match store.accept(&second, &second_digest, &first_digest) {
        Err(error @ ApproveError::UnknownProfile { .. }) => {
            assert_eq!(error.code(), "UNKNOWN_PROFILE");
            assert!(
                format!("{error}").contains("no-such-profile"),
                "rejection names the profile, got {error}"
            );
        }
        other => panic!("unknown profile must fail re-render, got {other:?}"),
    }
    assert_eq!(
        read_baseline_file(&baselines, &test_id),
        first_bytes,
        "rejections keep the previous baseline bytes"
    );
    assert!(
        !all_files_below(&baselines)
            .iter()
            .any(|name| name.contains(".tmp-accept-")),
        "rejections leave no temporary files"
    );
}

/// C10: re-rendered pixels that differ from the stored candidate pixels
/// reject with `INCONSISTENT_SAMPLE` and write no baseline. The rewritten
/// snapshot keeps valid structure and image digests, so only the
/// pixel comparison can refuse it.
#[test]
fn d04_c10_rerender_mismatch_rejected() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let candidate_bytes = with_first_cell_text(&pixel_sample_bytes(logical, &base_snapshot()), "b");
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(error @ ApproveError::InconsistentSample { .. }) => {
            assert_eq!(error.code(), "INCONSISTENT_SAMPLE");
        }
        other => panic!("divergent pixels must fail re-render, got {other:?}"),
    }
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// C10: a snapshot no specified face can render rejects with `MISSING_GLYPH`
/// and writes no baseline. The crab keeps valid structure and image
/// digests, so only the re-render can refuse it.
#[test]
fn d04_c10_missing_glyph_rejected() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let candidate_bytes =
        with_first_cell_text(&pixel_sample_bytes(logical, &base_snapshot()), "🦀");
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(error @ ApproveError::MissingGlyph { .. }) => {
            assert_eq!(error.code(), "MISSING_GLYPH");
        }
        other => panic!("unrenderable text must fail re-render, got {other:?}"),
    }
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// R2 digest carry: an unknown-profile candidate against a present baseline
/// rejects with `UNKNOWN_PROFILE` but still reports the digest the root
/// actually held, so the failure JSON names the observed baseline instead
/// of null. The rejection still writes nothing.
#[test]
fn d04_r2_unknown_profile_carries_observed_baseline() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let first_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    let first = candidate_file(dir.path(), "first.tui.json", &first_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &first, &first_digest, "missing") {
        return;
    }
    accepted_of(store.accept(&first, &first_digest, "missing"));
    let second_bytes = with_profile(&first_bytes, "no-such-profile");
    let second_digest = sha256_hex(&second_bytes);
    let second = candidate_file(dir.path(), "second.tui.json", &second_bytes);
    match store.accept(&second, &second_digest, &first_digest) {
        Err(error @ ApproveError::UnknownProfile { .. }) => {
            assert_eq!(error.code(), "UNKNOWN_PROFILE");
            assert_eq!(
                error.observed_baseline(),
                Some(first_digest.as_str()),
                "unknown-profile rejection reports the present baseline digest"
            );
        }
        other => panic!("unknown profile must fail re-render, got {other:?}"),
    }
    assert_eq!(
        read_baseline_file(&baselines, &test_id),
        first_bytes,
        "rejections keep the previous baseline bytes"
    );
}

/// R2 digest carry: a re-rendered pixel mismatch against a present baseline
/// rejects with `INCONSISTENT_SAMPLE` but still reports the digest the
/// root actually held. Contract verification stays pre-commit: the
/// divergent candidate is refused and the baseline is untouched.
#[test]
fn d04_r2_rerender_mismatch_carries_observed_baseline() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let first_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    let first = candidate_file(dir.path(), "first.tui.json", &first_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &first, &first_digest, "missing") {
        return;
    }
    accepted_of(store.accept(&first, &first_digest, "missing"));
    let second_bytes = with_first_cell_text(&first_bytes, "b");
    assert_ne!(
        sha256_hex(&second_bytes),
        first_digest,
        "fixture must diverge from the baseline"
    );
    let second_digest = sha256_hex(&second_bytes);
    let second = candidate_file(dir.path(), "second.tui.json", &second_bytes);
    match store.accept(&second, &second_digest, &first_digest) {
        Err(error @ ApproveError::InconsistentSample { .. }) => {
            assert_eq!(error.code(), "INCONSISTENT_SAMPLE");
            assert_eq!(
                error.observed_baseline(),
                Some(first_digest.as_str()),
                "rerender-mismatch rejection reports the present baseline digest"
            );
        }
        other => panic!("divergent pixels must fail re-render, got {other:?}"),
    }
    assert_eq!(
        read_baseline_file(&baselines, &test_id),
        first_bytes,
        "rejections keep the previous baseline bytes"
    );
}

/// R2 digest carry: a missing glyph against a present baseline rejects with
/// `MISSING_GLYPH` but still reports the digest the root actually held.
#[test]
fn d04_r2_missing_glyph_carries_observed_baseline() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let first_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    let first = candidate_file(dir.path(), "first.tui.json", &first_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &first, &first_digest, "missing") {
        return;
    }
    accepted_of(store.accept(&first, &first_digest, "missing"));
    let second_bytes = with_first_cell_text(&first_bytes, "🦀");
    let second_digest = sha256_hex(&second_bytes);
    let second = candidate_file(dir.path(), "second.tui.json", &second_bytes);
    match store.accept(&second, &second_digest, &first_digest) {
        Err(error @ ApproveError::MissingGlyph { .. }) => {
            assert_eq!(error.code(), "MISSING_GLYPH");
            assert_eq!(
                error.observed_baseline(),
                Some(first_digest.as_str()),
                "missing-glyph rejection reports the present baseline digest"
            );
        }
        other => panic!("unrenderable text must fail re-render, got {other:?}"),
    }
}

/// R2 digest carry: a re-render failure with no baseline present reports
/// the observed `missing` literal, not null, while still refusing the
/// commit and writing nothing but the stable lock file.
#[test]
fn d04_r2_rerender_failure_without_baseline_carries_missing() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let candidate_bytes = with_first_cell_text(&pixel_sample_bytes(logical, &base_snapshot()), "b");
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(error @ ApproveError::InconsistentSample { .. }) => {
            assert_eq!(error.code(), "INCONSISTENT_SAMPLE");
            assert_eq!(
                error.observed_baseline(),
                Some("missing"),
                "re-render failure without a baseline reports the missing literal"
            );
        }
        other => panic!("divergent pixels must fail re-render, got {other:?}"),
    }
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// C10: image bytes that disagree with their recorded digests fail approval
/// with a sample error, not a silent write.
#[test]
fn d04_c10_tampered_image_rejected_without_write() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let candidate_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let mut rewritten: serde_json::Value =
        serde_json::from_slice(&candidate_bytes).expect("candidate parses as JSON");
    let png = rewritten["image"]["png_base64"]
        .as_str()
        .expect("image carries base64 PNG");
    let mut flipped = png.to_string();
    let last = flipped.pop().expect("PNG base64 is not empty");
    flipped.push(if last == 'A' { 'B' } else { 'A' });
    rewritten["image"]["png_base64"] = serde_json::json!(flipped);
    let tampered = serde_json::to_vec(&rewritten).expect("tampered candidate serializes");
    let digest = sha256_hex(&tampered);
    let candidate = candidate_file(dir.path(), "tampered.tui.json", &tampered);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &digest, "missing") {
        return;
    }
    match store.accept(&candidate, &digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "INVALID_IMAGE");
        }
        other => panic!("tampered image must fail validation, got {other:?}"),
    }
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// C11: a stale expected digest is rejected, and the frozen baselines
/// survive the refusal untouched: pre- and post-digests and the file tree
/// are identical, with no lock file left behind.
#[test]
fn d04_c11_stale_baseline_rejected_baseline_untouched() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let first_bytes = state_sample_bytes("views/login", &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    write_baseline_file(&baselines, "views/login", &first_bytes);
    let before_files = all_files_below(&baselines);
    let revised = snapshot_with_cells(filled_cells("b"), SnapshotMetadata::default());
    let second_bytes = state_sample_bytes("views/login", &revised);
    let second_digest = sha256_hex(&second_bytes);
    assert_ne!(second_digest, first_digest);
    let second = candidate_file(dir.path(), "second.tui.json", &second_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &second, &second_digest, &"c".repeat(64)) {
        return;
    }
    match store.accept(&second, &second_digest, &"c".repeat(64)) {
        Err(ApproveError::StaleBaseline { expected, actual }) => {
            assert_eq!(expected, "c".repeat(64));
            assert_eq!(actual, first_digest);
        }
        other => panic!("stale baseline must fail arbitration, got {other:?}"),
    }
    assert_eq!(read_baseline_file(&baselines, "views/login"), first_bytes);
    assert_eq!(
        sha256_hex(&read_baseline_file(&baselines, "views/login")),
        first_digest
    );
    let mut expected_files = before_files;
    expected_files.push(".tuiscotti-approval.lock".to_string());
    expected_files.sort();
    assert_eq!(
        all_files_below(&baselines),
        expected_files,
        "refusals keep the baseline tree plus the stable lock file"
    );
    assert!(
        !all_files_below(&baselines)
            .iter()
            .any(|name| name.contains(".tmp-accept-")),
        "refusals leave no temporary files"
    );
}

/// C11: two approval attempts with the same expected digest run together;
/// exactly one conflicting replacement succeeds and the baseline holds the
/// winner's bytes.
#[test]
fn d04_c11_concurrent_accepts_single_winner() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let first_bytes = state_sample_bytes("views/login", &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    write_baseline_file(&baselines, "views/login", &first_bytes);
    let revised_a = snapshot_with_cells(filled_cells("b"), SnapshotMetadata::default());
    let bytes_a = state_sample_bytes("views/login", &revised_a);
    let digest_a = sha256_hex(&bytes_a);
    let revised_b = snapshot_with_cells(filled_cells("c"), SnapshotMetadata::default());
    let bytes_b = state_sample_bytes("views/login", &revised_b);
    let digest_b = sha256_hex(&bytes_b);
    let file_a = candidate_file(dir.path(), "a.tui.json", &bytes_a);
    let file_b = candidate_file(dir.path(), "b.tui.json", &bytes_b);
    let barrier = Arc::new(Barrier::new(2));
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &file_a, &digest_a, &first_digest) {
        return;
    }
    let worker = |candidate: PathBuf, digest: String, expected: String, barrier: Arc<Barrier>| {
        let roots = store.baseline_root().to_path_buf();
        thread::spawn(move || {
            barrier.wait();
            let store = ApprovalStore::new(&roots).expect("worker store opens");
            store.accept(&candidate, &digest, &expected)
        })
    };
    let handle_a = worker(
        file_a,
        digest_a.clone(),
        first_digest.clone(),
        Arc::clone(&barrier),
    );
    let handle_b = worker(
        file_b,
        digest_b.clone(),
        first_digest.clone(),
        Arc::clone(&barrier),
    );
    let result_a = handle_a.join().expect("worker A finishes");
    let result_b = handle_b.join().expect("worker B finishes");
    let outcomes = [result_a, result_b];
    let winners: Vec<_> = outcomes
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .collect();
    assert_eq!(
        winners.len(),
        1,
        "exactly one conflicting replacement succeeds"
    );
    let winner = winners[0];
    let loser = outcomes
        .iter()
        .find_map(|result| result.as_ref().err())
        .expect("one refusal");
    match loser {
        ApproveError::StaleBaseline { expected, actual } => {
            assert_eq!(expected, &first_digest);
            assert_eq!(actual.as_str(), winner.baseline_sha256());
        }
        other => panic!("the loser must see the winner's digest, got {other:?}"),
    }
    assert_eq!(read_baseline_file(&baselines, "views/login"), {
        if winner.candidate_sha256() == digest_a {
            bytes_a
        } else {
            assert_eq!(winner.candidate_sha256(), digest_b);
            bytes_b
        }
    });
}

/// Cross-process contender body: accept once, print the outcome line, and
/// always exit successfully so the parent can arbitrate the results.
#[expect(clippy::expect_used, reason = "cross-process child harness")]
fn cross_child_main() {
    let baselines = PathBuf::from(
        std::env::var_os("TUISCOTTI_D04_CROSS_BASELINES").expect("child baselines dir"),
    );
    let candidate = PathBuf::from(
        std::env::var_os("TUISCOTTI_D04_CROSS_CANDIDATE").expect("child candidate file"),
    );
    let digest = std::env::var("TUISCOTTI_D04_CROSS_DIGEST").expect("child candidate digest");
    let expected = std::env::var("TUISCOTTI_D04_CROSS_EXPECTED").expect("child expected digest");
    let store = ApprovalStore::new(&baselines).expect("child store opens");
    match store.accept(&candidate, &digest, &expected) {
        Ok(accepted) => println!("CROSS_WIN:{}", accepted.baseline_sha256()),
        Err(error) => println!("CROSS_LOSE:{}", error.code()),
    }
}

/// Spawn one contender child for the cross-process winner test.
#[expect(clippy::expect_used, reason = "test spawns child processes")]
fn spawn_contender(
    baselines: &Path,
    candidate: &Path,
    digest: &str,
    expected: &str,
) -> std::process::Child {
    std::process::Command::new(std::env::current_exe().expect("test binary resolves"))
        .arg("--exact")
        .arg("d04_c11_cross_process_accepts_single_winner")
        .arg("--nocapture")
        .env("TUISCOTTI_D04_CROSS_CHILD", "1")
        .env("TUISCOTTI_D04_CROSS_BASELINES", baselines)
        .env("TUISCOTTI_D04_CROSS_CANDIDATE", candidate)
        .env("TUISCOTTI_D04_CROSS_DIGEST", digest)
        .env("TUISCOTTI_D04_CROSS_EXPECTED", expected)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("contender child spawns")
}

/// C11: two child processes contend for one baseline root with the same
/// expected digest; exactly one conflicting replacement wins, the loser
/// sees the winner's digest as stale, and the lock file persists.
#[test]
fn d04_c11_cross_process_accepts_single_winner() {
    if std::env::var_os("TUISCOTTI_D04_CROSS_CHILD").is_some() {
        cross_child_main();
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let first_bytes = state_sample_bytes("views/login", &base_snapshot());
    let first_digest = sha256_hex(&first_bytes);
    write_baseline_file(&baselines, "views/login", &first_bytes);
    let bytes_a = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("b"), SnapshotMetadata::default()),
    );
    let digest_a = sha256_hex(&bytes_a);
    let bytes_b = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("c"), SnapshotMetadata::default()),
    );
    let digest_b = sha256_hex(&bytes_b);
    let file_a = candidate_file(dir.path(), "a.tui.json", &bytes_a);
    let file_b = candidate_file(dir.path(), "b.tui.json", &bytes_b);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &file_a, &digest_a, &first_digest) {
        return;
    }
    let child_a = spawn_contender(&baselines, &file_a, &digest_a, &first_digest);
    let child_b = spawn_contender(&baselines, &file_b, &digest_b, &first_digest);
    let out_a = child_a.wait_with_output().expect("contender A finishes");
    let out_b = child_b.wait_with_output().expect("contender B finishes");
    assert!(out_a.status.success(), "contender A exits cleanly");
    assert!(out_b.status.success(), "contender B exits cleanly");
    let text_a = String::from_utf8(out_a.stdout).expect("child output is UTF-8");
    let text_b = String::from_utf8(out_b.stdout).expect("child output is UTF-8");
    let wins = [text_a.as_str(), text_b.as_str()]
        .iter()
        .filter(|text| text.contains("CROSS_WIN:"))
        .count();
    assert_eq!(wins, 1, "exactly one contender wins: {text_a:?} {text_b:?}");
    let (winner_text, loser_text) = if text_a.contains("CROSS_WIN:") {
        (text_a, text_b)
    } else {
        (text_b, text_a)
    };
    assert!(
        loser_text.contains("CROSS_LOSE:STALE_BASELINE"),
        "the loser sees stale arbitration, got {loser_text:?}"
    );
    let winner_digest = winner_text
        .lines()
        .find_map(|line| line.strip_prefix("CROSS_WIN:"))
        .expect("winner reports its digest")
        .trim()
        .to_string();
    assert!(winner_digest == digest_a || winner_digest == digest_b);
    assert_eq!(
        sha256_hex(&read_baseline_file(&baselines, "views/login")),
        winner_digest,
        "the baseline holds the winner's bytes"
    );
    assert!(
        lock_path(&baselines).is_file(),
        "the stable lock file persists after contended approvals"
    );
}

/// C11: a fault before the last replacement keeps the previous baseline
/// bytes unchanged: a file blocking the baseline parent directory fails the
/// transaction without touching the approved baseline.
#[test]
fn d04_c11_fault_before_replace_keeps_previous() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let kept_bytes = state_sample_bytes("views/other", &base_snapshot());
    write_baseline_file(&baselines, "views/other", &kept_bytes);
    std::fs::write(baselines.join("login"), b"blocker").expect("blocker file writes");
    let before = all_files_below(&baselines);
    let candidate_bytes = state_sample_bytes("login/panel", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(error @ (ApproveError::Io { .. } | ApproveError::Check { .. })) => {
            assert_eq!(error.code(), "IO");
        }
        other => panic!("a blocked parent directory must fail I/O, got {other:?}"),
    }
    assert_eq!(read_baseline_file(&baselines, "views/other"), kept_bytes);
    let mut expected_files = before;
    expected_files.push(".tuiscotti-approval.lock".to_string());
    expected_files.sort();
    assert_eq!(
        all_files_below(&baselines),
        expected_files,
        "faults keep the previous tree plus the stable lock file"
    );
}

/// A present baseline skips test-ID and gate cross-comparison: a planted
/// baseline with a different embedded test ID (or a different gate) but an
/// equal snapshot still approves, and the outcome summary records no
/// `test-id` or `policy` mismatch. Only the digests arbitrate.
#[test]
fn d04_present_baseline_skips_id_gate_comparison() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    // Different embedded test ID, equal snapshot and gate.
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let planted_id = with_test_id(&candidate_bytes, "views/other");
    let planted_digest = sha256_hex(&planted_id);
    write_baseline_file(&baselines, "views/login", &planted_id);
    let artifacts = dir.path().join("artifacts");
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    assert_eq!(
        failure_of(frozen.check(&candidate_bytes).expect("check runs")).reason(),
        MismatchReason::TestId,
        "frozen comparison still cross-compares IDs"
    );
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    if ci_refusal_probed(&store, &candidate, &candidate_digest, &planted_digest) {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, &planted_digest));
    assert_eq!(accepted.previous_baseline_sha256(), planted_digest);
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
    assert_eq!(
        accepted.reason(),
        None,
        "a present baseline with an equal snapshot approves with an empty change summary"
    );
    assert!(accepted.changed_fields().is_empty());
    // Different gate shape: a pixel baseline under a state candidate path.
    let pixel_bytes = pixel_sample_bytes("views/login", &base_snapshot());
    let pixel_digest = sha256_hex(&pixel_bytes);
    write_baseline_file(&baselines, "views/login", &pixel_bytes);
    let second = accepted_of(store.accept(&candidate, &candidate_digest, &pixel_digest));
    assert_eq!(second.previous_baseline_sha256(), pixel_digest);
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
    assert_eq!(
        second.reason(),
        None,
        "a state candidate over a pixel baseline still approves with an empty change summary"
    );
    assert!(second.changed_fields().is_empty());
}

/// Digest mixtures: a wrong candidate digest fails first as stale, a wrong
/// baseline digest fails as stale, and both wrong still reports the
/// candidate first. Malformed digests fail as invalid arguments.
#[test]
fn d04_digest_mixtures_rejected_with_codes() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let wrong = "d".repeat(64);
    if ci_refusal_probed(&store, &candidate, &wrong, "missing") {
        return;
    }
    match store.accept(&candidate, &wrong, "missing") {
        Err(ApproveError::StaleCandidate { expected, actual }) => {
            assert_eq!(expected, wrong);
            assert_eq!(actual, candidate_digest);
        }
        other => panic!("a wrong candidate digest must fail first, got {other:?}"),
    }
    match store.accept(&candidate, &candidate_digest, &wrong) {
        Err(ApproveError::StaleBaseline { expected, actual }) => {
            assert_eq!(expected, wrong);
            assert_eq!(actual, "missing");
        }
        other => panic!("a wrong baseline digest must fail arbitration, got {other:?}"),
    }
    match store.accept(&candidate, &wrong, &wrong) {
        Err(ApproveError::StaleCandidate { .. }) => {}
        other => panic!("a doubly wrong pair reports the candidate first, got {other:?}"),
    }
    let shapes = [
        String::new(),
        "abc".to_string(),
        "A".repeat(64),
        "0".repeat(64),
    ];
    for bad in &shapes {
        match store.accept(&candidate, bad, "missing") {
            Err(error @ ApproveError::InvalidArgument { .. }) => {
                assert_eq!(error.code(), "INVALID_SAMPLE");
            }
            other => panic!("malformed candidate digest {bad:?} must fail, got {other:?}"),
        }
        match store.accept(&candidate, &candidate_digest, bad) {
            Err(error @ ApproveError::InvalidArgument { .. }) => {
                assert_eq!(error.code(), "INVALID_SAMPLE");
            }
            other => panic!("malformed baseline digest {bad:?} must fail, got {other:?}"),
        }
    }
}

/// Section 16.3 table, arguments and samples: every malformed, corrupt,
/// foreign-schema, over-limit, and image shape maps 1:1 with its Section
/// 16.3 code (process exits are a CLI-boundary concern).
#[test]
fn d04_error_codes_arguments_and_samples() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    // Unknown sample schema rejects as UNSUPPORTED_SCHEMA.
    let schema_bytes = with_schema(&candidate_bytes, "tuiscotti.sample.v99");
    let schema_digest = sha256_hex(&schema_bytes);
    let schema_file = candidate_file(dir.path(), "schema.tui.json", &schema_bytes);
    if ci_refusal_probed(&store, &schema_file, &schema_digest, "missing") {
        return;
    }
    match store.accept(&schema_file, &schema_digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "UNSUPPORTED_SCHEMA");
        }
        other => panic!("unknown schema must fail, got {other:?}"),
    }
    // An over-limit image dimension claim rejects as RESOURCE_LIMIT, as
    // does an overlong nested snapshot cell string (typed wire preflight).
    // The dimension claim is the live image resource-limit shape.
    let pixel_bytes = pixel_sample_bytes("views/login", &base_snapshot());
    let big_bytes = with_image_dims(&pixel_bytes, 40_000, 40);
    let big_digest = sha256_hex(&big_bytes);
    let big_file = candidate_file(dir.path(), "big.tui.json", &big_bytes);
    match store.accept(&big_file, &big_digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "RESOURCE_LIMIT");
        }
        other => panic!("over-limit dimensions must fail, got {other:?}"),
    }
    // Tampered image bytes reject as INVALID_IMAGE.
    let mut rewritten: serde_json::Value =
        serde_json::from_slice(&pixel_bytes).expect("candidate parses as JSON");
    let png = rewritten["image"]["png_base64"]
        .as_str()
        .expect("image carries base64 PNG");
    let mut flipped = png.to_string();
    let last = flipped.pop().expect("PNG base64 is not empty");
    flipped.push(if last == 'A' { 'B' } else { 'A' });
    rewritten["image"]["png_base64"] = serde_json::json!(flipped);
    let tampered = serde_json::to_vec(&rewritten).expect("tampered candidate serializes");
    let tampered_digest = sha256_hex(&tampered);
    let tampered_file = candidate_file(dir.path(), "tampered.tui.json", &tampered);
    match store.accept(&tampered_file, &tampered_digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "INVALID_IMAGE");
        }
        other => panic!("tampered image must fail, got {other:?}"),
    }
    // A missing candidate file rejects as IO.
    match store.accept(
        &dir.path().join("absent.tui.json"),
        &candidate_digest,
        "missing",
    ) {
        Err(error @ ApproveError::Io { .. }) => {
            assert_eq!(error.code(), "IO");
        }
        other => panic!("a missing candidate must fail I/O, got {other:?}"),
    }
    // An empty store root rejects as INVALID_STORE_PATH.
    match ApprovalStore::new(Path::new("")) {
        Err(error) => {
            assert_eq!(error.code(), "INVALID_STORE_PATH");
        }
        Ok(_) => panic!("an empty root must fail"),
    }
    // The corrupt and malformed shapes above never approved anything.
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// Section 16.3 table, re-render: profile, render, glyph, and consistency
/// shapes map 1:1. `RENDER_FAILED` maps a constructed upstream rejection
/// (its live source is proven producible in
/// `d04_rerender_failed_source_producible`; approval itself cannot hit it
/// live because a past-budget grid exceeds the sample file cap first).
#[test]
fn d04_error_codes_rerender() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let pixel_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let accept_rewritten = |name: &str, bytes: &[u8]| {
        let digest = sha256_hex(bytes);
        let file = candidate_file(dir.path(), name, bytes);
        (digest, file)
    };
    let (profile_digest, profile_file) = accept_rewritten(
        "profile.tui.json",
        &with_profile(&pixel_bytes, "no-such-profile"),
    );
    if ci_refusal_probed(&store, &profile_file, &profile_digest, "missing") {
        return;
    }
    match store.accept(&profile_file, &profile_digest, "missing") {
        Err(error) => {
            assert_eq!(error.code(), "UNKNOWN_PROFILE");
        }
        Ok(_) => panic!("unknown profile must fail"),
    }
    let (glyph_digest, glyph_file) =
        accept_rewritten("glyph.tui.json", &with_first_cell_text(&pixel_bytes, "🦀"));
    match store.accept(&glyph_file, &glyph_digest, "missing") {
        Err(error) => {
            assert_eq!(error.code(), "MISSING_GLYPH");
        }
        Ok(_) => panic!("unrenderable text must fail"),
    }
    let (pixels_digest, pixels_file) =
        accept_rewritten("pixels.tui.json", &with_first_cell_text(&pixel_bytes, "b"));
    match store.accept(&pixels_file, &pixels_digest, "missing") {
        Err(error) => {
            assert_eq!(error.code(), "INCONSISTENT_SAMPLE");
        }
        Ok(_) => panic!("divergent pixels must fail"),
    }
    let render_failed = ApproveError::RenderFailed {
        source: tui_test::RenderError::ImageTooLarge {
            cols: 1000,
            rows: 600,
        },
        observed_baseline: None,
    };
    assert_eq!(render_failed.code(), "RENDER_FAILED");
    assert!(render_failed.observed_baseline().is_none());
}

/// Section 16.3 table, arbitration and control: stale digests map 1:1, and
/// the CI and lock-timeout codes map on their constructed shapes (both are
/// produced live in their dedicated cross-process tests).
#[test]
fn d04_error_codes_arbitration_and_control() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &"d".repeat(64), "missing") {
        return;
    }
    match store.accept(&candidate, &"d".repeat(64), "missing") {
        Err(error) => {
            assert_eq!(error.code(), "STALE_CANDIDATE");
        }
        Ok(_) => panic!("a wrong candidate digest must fail"),
    }
    match store.accept(&candidate, &candidate_digest, &"d".repeat(64)) {
        Err(error) => {
            assert_eq!(error.code(), "STALE_BASELINE");
        }
        Ok(_) => panic!("a wrong baseline digest must fail"),
    }
    assert_eq!(
        ApproveError::CiApprovalForbidden.code(),
        "CI_APPROVAL_FORBIDDEN"
    );
    assert_eq!(ApproveError::LockTimeout.code(), "LOCK_TIMEOUT");
}

/// The `RENDER_FAILED` source is producible upstream: a structurally valid
/// snapshot past the render pixel budget fails `render_snapshot_png` with
/// `ImageTooLarge`, the shape approval maps to `RENDER_FAILED`.
#[test]
fn d04_rerender_failed_source_producible() {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let cells = vec![plain_cell("a", 1); 600_000];
    let snapshot = snapshot_with_grid(1000, 600, cells, SnapshotMetadata::default());
    assert!(
        matches!(
            tui_test::render_snapshot_png(&snapshot, &profile.render_options()),
            Err(tui_test::RenderError::ImageTooLarge { .. })
        ),
        "a 120M-pixel render past the 100M budget must fail"
    );
}

/// The candidate digest is tamper-evident: mutated candidate bytes fail
/// against the supplied digest with the recomputed value.
#[test]
fn d04_tamper_evident_candidate_rejected() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let mut candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let flip_at = candidate_bytes.len() / 2;
    candidate_bytes[flip_at] ^= 0xFF;
    let mutated_digest = sha256_hex(&candidate_bytes);
    assert_ne!(mutated_digest, candidate_digest);
    let candidate = candidate_file(dir.path(), "mutated.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(ApproveError::StaleCandidate { expected, actual }) => {
            assert_eq!(expected, candidate_digest);
            assert_eq!(actual, mutated_digest);
        }
        other => panic!("mutated bytes must fail the digest check, got {other:?}"),
    }
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "rejections write nothing but the stable lock file"
    );
}

/// A missing candidate file and a corrupt candidate file fail differently:
/// filesystem I/O versus a typed sample error.
#[test]
fn d04_missing_vs_corrupt_candidate_differ() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let missing = dir.path().join("no-such-candidate.tui.json");
    let digest = "e".repeat(64);
    if ci_refusal_probed(&store, &missing, &digest, "missing") {
        return;
    }
    match store.accept(&missing, &digest, "missing") {
        Err(error @ ApproveError::Io { .. }) => {
            assert_eq!(error.code(), "IO");
            assert!(
                format!("{error}").contains("no-such-candidate"),
                "missing file reports the unreadable path"
            );
        }
        other => panic!("a missing candidate must fail I/O, got {other:?}"),
    }
    let corrupt_bytes = b"{not a sample";
    let corrupt_digest = sha256_hex(corrupt_bytes);
    let corrupt = candidate_file(dir.path(), "corrupt.tui.json", corrupt_bytes);
    match store.accept(&corrupt, &corrupt_digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "INVALID_SAMPLE");
        }
        other => panic!("a corrupt candidate must fail validation, got {other:?}"),
    }
}

/// Accepting the same candidate twice with the prepared digests returns
/// identical results: the same success, a byte-identical baseline, and an
/// outcome whose summary moves only from the missing-baseline change to an
/// empty one.
#[test]
fn d04_accept_idempotent_same_candidate() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let first = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(first.previous_baseline_sha256(), "missing");
    assert_eq!(first.reason(), Some(MismatchReason::Missing));
    assert_eq!(first.changed_fields(), &["baseline".to_string()]);
    let second = accepted_of(store.accept(&candidate, &candidate_digest, &candidate_digest));
    assert_eq!(second.test_id(), first.test_id());
    assert_eq!(second.candidate_sha256(), first.candidate_sha256());
    assert_eq!(second.baseline_sha256(), first.baseline_sha256());
    assert_eq!(second.baseline_path(), first.baseline_path());
    assert_eq!(second.previous_baseline_sha256(), candidate_digest);
    assert_eq!(second.reason(), None);
    assert!(second.changed_fields().is_empty());
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            "views/login.tui.json".to_string(),
        ],
        "repeated accepts keep exactly the baseline plus the stable lock file"
    );
}

/// C12: the frozen store never writes: checks over a writable baseline root
/// add zero new files there, including no lock file.
#[test]
fn d04_c12_frozen_store_never_writes() {
    let (_temp, baselines, artifacts) = temp_store();
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let failure = failure_of(frozen.check(&candidate_bytes).expect("first check runs"));
    assert_eq!(failure.baseline_sha256(), "missing");
    assert_eq!(all_files_below(&baselines), Vec::<String>::new());
    write_baseline_file(&baselines, "views/login", &candidate_bytes);
    let success = success_of(frozen.check(&candidate_bytes).expect("second check runs"));
    assert_eq!(success.baseline_sha256(), candidate_digest);
    assert_eq!(
        all_files_below(&baselines),
        vec!["views/login.tui.json".to_string()]
    );
}

/// Read the CI child's task paths from its dedicated environment.
#[expect(clippy::expect_used, reason = "CI child harness reads its task paths")]
fn ci_child_paths() -> (PathBuf, PathBuf) {
    let baselines =
        PathBuf::from(std::env::var_os("TUISCOTTI_D04_CI_BASELINES").expect("child baselines dir"));
    let candidate = PathBuf::from(
        std::env::var_os("TUISCOTTI_D04_CI_CANDIDATE").expect("child candidate file"),
    );
    (baselines, candidate)
}

/// The CI child body: with `CI` set in its own environment, the programmatic
/// accept path must refuse with the code-3 error and write nothing.
#[expect(clippy::expect_used, reason = "CI child harness")]
#[expect(clippy::panic, reason = "CI child reports refusal failures")]
fn ci_child_main() {
    let (baselines, candidate) = ci_child_paths();
    let before = all_files_below(&baselines);
    let digest = sha256_hex(&std::fs::read(&candidate).expect("child candidate reads"));
    let store = ApprovalStore::new(&baselines).expect("child store opens");
    match store.accept(&candidate, &digest, "missing") {
        Err(error) => {
            assert!(
                matches!(error, ApproveError::CiApprovalForbidden),
                "CI refusal, got {error:?}"
            );
            assert_eq!(error.code(), "CI_APPROVAL_FORBIDDEN");
            println!("CI_REFUSED:{}", error.code());
        }
        Ok(_) => panic!("CI approval must fail"),
    }
    assert_eq!(
        all_files_below(&baselines),
        before,
        "CI refusal writes nothing"
    );
}

/// C12: every set `CI` value refuses approval with the code-3 error and
/// writes nothing. Each value runs in a dedicated child process; the
/// parallel runner environment is never changed.
#[test]
fn d04_c12_ci_child_accept_refused_without_writes() {
    if std::env::var_os("TUISCOTTI_D04_CI_CHILD").is_some() {
        ci_child_main();
        return;
    }
    for value in ["1", "true", "t", "TRUE", ""] {
        let (temp, baselines, _artifacts) = temp_store();
        let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
        let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
        let child =
            std::process::Command::new(std::env::current_exe().expect("test binary resolves"))
                .arg("--exact")
                .arg("d04_c12_ci_child_accept_refused_without_writes")
                .arg("--nocapture")
                .env("TUISCOTTI_D04_CI_CHILD", "1")
                .env("CI", value)
                .env("TUISCOTTI_D04_CI_BASELINES", &baselines)
                .env("TUISCOTTI_D04_CI_CANDIDATE", &candidate)
                .output()
                .expect("CI child runs");
        assert!(
            child.status.success(),
            "CI={value:?} child passes: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let stdout = String::from_utf8(child.stdout).expect("child output is UTF-8");
        assert!(
            stdout.contains("CI_REFUSED:CI_APPROVAL_FORBIDDEN"),
            "CI={value:?} reports its code, got {stdout:?}"
        );
        assert_eq!(all_files_below(&baselines), Vec::<String>::new());
    }
    let (temp, baselines, _artifacts) = temp_store();
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if !ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    }
}

/// C12: acceptance against a frozen tree is refused. A directory blocking
/// the lock path fails the transaction portably with a typed I/O error and
/// zero new files.
#[test]
fn d04_c12_blocked_lock_path_accept_refused() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    std::fs::create_dir(baselines.join(".tuiscotti-approval.lock")).expect("lock blocker creates");
    let before = all_files_below(&baselines);
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    match store.accept(&candidate, &candidate_digest, "missing") {
        Err(error @ ApproveError::Io { .. }) => {
            assert_eq!(error.code(), "IO");
        }
        other => panic!("a blocked lock path must fail I/O, got {other:?}"),
    }
    assert_eq!(all_files_below(&baselines), before);
}

/// C12: a filesystem read-only baseline root checks without writes, and
/// explicit approval against that root fails without writing.
#[cfg(unix)]
#[test]
fn d04_c12_readonly_root_checks_but_refuses_approval() {
    use std::os::unix::fs::PermissionsExt as _;
    let (temp, baselines, artifacts) = temp_store();
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let before = all_files_below(&baselines);
    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o555))
        .expect("root turns read-only");
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let checked = frozen.check(&baseline_bytes).expect("read-only check runs");
    assert!(checked.is_pass());
    let candidate = temp.path().join("candidate.tui.json");
    std::fs::write(&candidate, &baseline_bytes).expect("candidate writes");
    let digest = sha256_hex(&baseline_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &digest, &digest) {
        std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o755))
            .expect("root restores");
        return;
    }
    let outcome = store.accept(&candidate, &digest, &digest);
    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o755))
        .expect("root restores");
    match outcome {
        Err(error @ ApproveError::Io { .. }) => {
            assert_eq!(error.code(), "IO");
        }
        other => panic!("read-only approval must fail I/O, got {other:?}"),
    }
    assert_eq!(all_files_below(&baselines), before);
    assert!(
        !all_files_below(&baselines)
            .iter()
            .any(|name| name.contains("lock")),
        "no lock file under a read-only root"
    );
}

/// The lock lifecycle: no lock file before the first transaction, one exists
/// while checking runs, and it persists after release for the next holder
/// to lock. Locks are OS-held, so this observes one process only and
/// re-acquires the same file after release.
#[test]
fn d04_lock_lifecycle_stable_file_reused() {
    let temp = tempfile::tempdir().expect("tempdir creates");
    let baselines = temp.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidates = temp.path().join("candidates");
    std::fs::create_dir_all(&candidates).expect("candidate dir creates");
    let has_lock_file = || {
        all_files_below(&baselines)
            .iter()
            .any(|name| name.contains("lock"))
    };
    assert!(!has_lock_file(), "no lock file before the transaction");
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let probe_bytes = state_sample_bytes("views/probe", &base_snapshot());
    let probe_digest = sha256_hex(&probe_bytes);
    let probe = candidate_file(&candidates, "probe.tui.json", &probe_bytes);
    if ci_refusal_probed(&store, &probe, &probe_digest, "missing") {
        return;
    }
    let worker = thread::spawn({
        let baselines = baselines.clone();
        let candidates = candidates.clone();
        move || {
            let store = ApprovalStore::new(&baselines).expect("worker store opens");
            for index in 0..40u32 {
                let test_id = format!("views/panel-{index}");
                let bytes = state_sample_bytes(&test_id, &base_snapshot());
                let digest = sha256_hex(&bytes);
                let path = candidates.join(format!("candidate-{index}.tui.json"));
                std::fs::write(&path, &bytes).expect("worker candidate writes");
                store
                    .accept(&path, &digest, "missing")
                    .expect("worker accept succeeds");
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = false;
    while Instant::now() < deadline {
        if has_lock_file() {
            seen = true;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    worker.join().expect("worker finishes");
    assert!(seen, "a lock file exists while the transaction runs");
    assert!(
        has_lock_file(),
        "the stable lock file persists after release"
    );
    assert!(
        !all_files_below(&baselines)
            .iter()
            .any(|name| name.contains(".tmp-accept-")),
        "no temporary files survive the transactions"
    );
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let bytes = state_sample_bytes("views/final", &base_snapshot());
    let digest = sha256_hex(&bytes);
    let path = candidates.join("final.tui.json");
    std::fs::write(&path, &bytes).expect("final candidate writes");
    accepted_of(store.accept(&path, &digest, "missing"));
}

/// A stale lock file from a dead process never blocks: with the file
/// present but no holder, approval takes the OS lock promptly and the
/// transaction succeeds. An existence-based lock would wait out the
/// deadline on this residue instead.
#[test]
fn d04_lock_stale_file_does_not_block() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    std::fs::write(lock_path(&baselines), b"stale").expect("stale file writes");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let started = Instant::now();
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "a stale file never costs the deadline"
    );
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(
        read_baseline_file(&baselines, "views/login"),
        candidate_bytes
    );
}

/// Attempt-child body for the cross-process deadline test: accept once and
/// print the machine code.
#[expect(clippy::expect_used, reason = "cross-process child harness")]
fn attempt_child_main() {
    let baselines = PathBuf::from(
        std::env::var_os("TUISCOTTI_D04_ATTEMPT_BASELINES").expect("child baselines dir"),
    );
    let candidate = PathBuf::from(
        std::env::var_os("TUISCOTTI_D04_ATTEMPT_CANDIDATE").expect("child candidate file"),
    );
    let digest = std::env::var("TUISCOTTI_D04_ATTEMPT_DIGEST").expect("child candidate digest");
    let store = ApprovalStore::new(&baselines).expect("child store opens");
    match store.accept(&candidate, &digest, "missing") {
        Ok(_) => println!("ATTEMPT_OK"),
        Err(error) => println!("ATTEMPT:{}", error.code()),
    }
}

/// The lock deadline fires across processes: with the OS lock held in the
/// parent, a child accept waits the full 30 seconds and reports
/// `LOCK_TIMEOUT`, writing nothing. After release the same accept
/// succeeds, proving no residue blocks.
#[test]
fn d04_lock_timeout_while_held_cross_process() {
    if std::env::var_os("TUISCOTTI_D04_ATTEMPT_CHILD").is_some() {
        attempt_child_main();
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let holder = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path(&baselines))
        .expect("parent opens the stable file");
    holder.lock_exclusive().expect("parent holds the OS lock");
    let output = std::process::Command::new(std::env::current_exe().expect("test binary resolves"))
        .arg("--exact")
        .arg("d04_lock_timeout_while_held_cross_process")
        .arg("--nocapture")
        .env("TUISCOTTI_D04_ATTEMPT_CHILD", "1")
        .env("TUISCOTTI_D04_ATTEMPT_BASELINES", &baselines)
        .env("TUISCOTTI_D04_ATTEMPT_CANDIDATE", &candidate)
        .env("TUISCOTTI_D04_ATTEMPT_DIGEST", &candidate_digest)
        .output()
        .expect("attempt child runs");
    assert!(output.status.success(), "attempt child exits cleanly");
    let stdout = String::from_utf8(output.stdout).expect("child output is UTF-8");
    assert!(
        stdout.contains("ATTEMPT:LOCK_TIMEOUT"),
        "the held lock reports its code, got {stdout:?}"
    );
    assert_eq!(
        entry_names(&baselines),
        [".tuiscotti-approval.lock".to_string()],
        "the timed-out attempt writes nothing"
    );
    holder.unlock().expect("parent releases the OS lock");
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
}

/// The candidate file is read-only input: approval never writes it, and a
/// full check plus accept run succeeds under a read-only baseline root for
/// the check half.
#[test]
fn d04_candidate_input_stays_readonly() {
    let (temp, baselines, artifacts) = temp_store();
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(
        std::fs::read(&candidate).expect("candidate re-reads"),
        candidate_bytes,
        "approval never writes the candidate file"
    );
    let success = success_of(frozen.check(&candidate_bytes).expect("check runs"));
    assert_eq!(success.candidate_sha256(), candidate_digest);
}

/// Check and accept stay consistent: the frozen check's candidate digest is
/// the accepted digest, and the next check compares the accepted baseline.
#[test]
fn d04_check_accept_digest_consistency() {
    let (temp, baselines, artifacts) = temp_store();
    let frozen = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let missing = failure_of(frozen.check(&candidate_bytes).expect("first check runs"));
    assert_eq!(missing.candidate_sha256(), candidate_digest);
    assert_eq!(missing.baseline_sha256(), "missing");
    let candidate = candidate_file(temp.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.candidate_sha256(), missing.candidate_sha256());
    assert_eq!(accepted.baseline_sha256(), sha256_hex(&candidate_bytes));
    let passing = success_of(frozen.check(&candidate_bytes).expect("second check runs"));
    assert_eq!(passing.candidate_sha256(), accepted.candidate_sha256());
    assert_eq!(passing.baseline_sha256(), accepted.baseline_sha256());
}

/// Approval persists no volatile data: exactly the baseline plus the lock
/// file land on disk, and the outcome summary holds only deterministic
/// addressing, digests, and change fields.
#[test]
fn d04_approval_persists_no_volatile_data() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    let candidate_bytes = state_sample_bytes("views/login", &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            "views/login.tui.json".to_string(),
        ],
        "a success persists exactly the baseline plus the stable lock file"
    );
    let root_text = baselines.to_string_lossy().into_owned();
    let stored_text = String::from_utf8(read_baseline_file(&baselines, "views/login"))
        .expect("baseline is UTF-8");
    assert!(
        !stored_text.contains(&root_text),
        "persisted bytes hold no absolute paths"
    );
    assert!(
        !stored_text.contains(".tmp-"),
        "persisted bytes hold no temporary names"
    );
    assert!(
        !stored_text.contains(".tuiscotti-approval.lock"),
        "persisted bytes hold no lock file names"
    );
    assert_eq!(accepted.test_id(), "views/login");
    assert_eq!(accepted.candidate_sha256(), candidate_digest);
    assert_eq!(accepted.previous_baseline_sha256(), "missing");
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(accepted.reason(), Some(MismatchReason::Missing));
    assert_eq!(accepted.changed_fields(), &["baseline".to_string()]);
}

/// Rewrite the render contract of a JSON sample through a mutation closure,
/// leaving the snapshot and image bytes untouched.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn with_rewritten_contract(bytes: &[u8], rewrite: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).expect("candidate parses as JSON");
    rewrite(&mut value["render_contract"]);
    serde_json::to_vec(&value).expect("rewritten candidate serializes")
}

/// Approve one prohibited-ID candidate against a missing baseline: the
/// refusal reports `INVALID_SAMPLE` and writes nothing but the stable lock
/// file, with no escaped file anywhere under the tempdir.
#[expect(clippy::expect_used, reason = "test fixture setup")]
#[expect(clippy::panic, reason = "test helper rejects an accepted bad ID")]
fn reject_prohibited_id_missing(bad: &str, base: &[u8]) {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = with_test_id(base, bad);
    let digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &digest, "missing") {
        return;
    }
    match store.accept(&candidate, &digest, "missing") {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "INVALID_SAMPLE", "ID {bad:?}");
        }
        other => panic!("prohibited ID {bad:?} must fail, got {other:?}"),
    }
    let mut below = all_files_below(dir.path());
    below.retain(|name| name != "candidate.tui.json");
    assert_eq!(
        below,
        ["baselines/.tuiscotti-approval.lock".to_string()],
        "ID {bad:?} writes nothing but the stable lock file"
    );
}

/// Approve one prohibited-ID candidate with a present baseline for a valid
/// ID: the refusal reports `INVALID_SAMPLE` and the planted baseline
/// survives byte-identical.
#[expect(clippy::expect_used, reason = "test fixture setup")]
#[expect(clippy::panic, reason = "test helper rejects an accepted bad ID")]
fn reject_prohibited_id_present(bad: &str, base: &[u8], planted_id: &str) {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    write_baseline_file(&baselines, planted_id, base);
    let expected = sha256_hex(base);
    let candidate_bytes = with_test_id(base, bad);
    let digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &digest, &expected) {
        return;
    }
    match store.accept(&candidate, &digest, &expected) {
        Err(error @ ApproveError::Check { .. }) => {
            assert_eq!(error.code(), "INVALID_SAMPLE", "ID {bad:?}");
        }
        other => panic!("prohibited ID {bad:?} must fail, got {other:?}"),
    }
    assert_eq!(
        read_baseline_file(&baselines, planted_id),
        base,
        "ID {bad:?} leaves the planted baseline untouched"
    );
}

/// Rework B1: every prohibited test ID rejects with both a missing and a
/// present baseline, and no file outside the store is touched. Rejected IDs
/// are never normalized into a path.
#[test]
fn d04_rework_b1_prohibited_ids_rejected_missing_and_present() {
    let base = state_sample_bytes("views/login", &base_snapshot());
    let mut state_ids: Vec<String> = [
        "",
        "/",
        "/views/login",
        "views/login/",
        "views//login",
        ".",
        "..",
        "views/./login",
        "views/../login",
        "../evil",
        "a/../../evil",
        "views\\login",
        "views/Login",
        "views/log in",
        "views/log-in!",
        "views/lögin",
        "views/a.b",
        "con",
        "views/nul",
        "views/COM1",
        "views/com1.txt",
        "aux.txt",
        "login/pixels-x",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    state_ids.push("s".repeat(65));
    state_ids.push(format!("{}/x", "s".repeat(239)));
    for bad in &state_ids {
        reject_prohibited_id_missing(bad, &base);
        reject_prohibited_id_present(bad, &base, "views/login");
    }
    // Pixel-gate shapes: a logical ID without the platform suffix, a
    // traversal that carries the suffix, and a foreign suffix.
    let pixel_base = pixel_sample_bytes("views/login", &base_snapshot());
    let suffix = format!("pixels-{}", platform_key());
    let pixel_ids = [
        "views/login".to_string(),
        format!("../evil/{suffix}"),
        "views/login/pixels-far-far-away".to_string(),
    ];
    for bad in &pixel_ids {
        reject_prohibited_id_missing(bad, &pixel_base);
        reject_prohibited_id_present(bad, &pixel_base, &format!("views/login/{suffix}"));
    }
}

/// Rework B2: the atomic replacement is the final fallible filesystem
/// operation. A pixel-gate success persists exactly the baseline plus the
/// lock, and a stale pre-commit refusal leaves the baseline bytes and the
/// tree untouched with no temporary residue and no evidence directory.
#[test]
fn d04_rework_b2_replacement_is_final_fallible_write() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let candidate_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let candidate_digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &candidate, &candidate_digest, "missing") {
        return;
    }
    let accepted = accepted_of(store.accept(&candidate, &candidate_digest, "missing"));
    assert_eq!(accepted.baseline_sha256(), candidate_digest);
    assert_eq!(accepted.reason(), Some(MismatchReason::Missing));
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            format!("{test_id}.tui.json"),
        ],
        "a success persists exactly the baseline plus the stable lock file"
    );
    // A stale pre-commit refusal changes nothing.
    let revised = with_first_cell_text(&candidate_bytes, "b");
    let revised_digest = sha256_hex(&revised);
    let revised_file = candidate_file(dir.path(), "revised.tui.json", &revised);
    match store.accept(&revised_file, &revised_digest, &"e".repeat(64)) {
        Err(ApproveError::StaleBaseline { .. }) => {}
        other => panic!("a stale baseline must fail arbitration, got {other:?}"),
    }
    assert_eq!(read_baseline_file(&baselines, &test_id), candidate_bytes);
    assert_eq!(
        all_files_below(&baselines),
        vec![
            ".tuiscotti-approval.lock".to_string(),
            format!("{test_id}.tui.json"),
        ],
        "a pre-commit refusal keeps the baseline bytes and tree"
    );
    assert!(
        !all_files_below(&baselines)
            .iter()
            .any(|name| name.contains(".tmp-accept-") || name.contains(".tuiscotti-accept")),
        "no temporary or evidence residue survives"
    );
}

/// Approve one contract-rewritten candidate against a missing baseline and
/// return the outcome with its tempdir: the caller matches the refusal and
/// asserts the tree.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn refuse_rewritten_contract(
    base: &[u8],
    rewrite: impl FnOnce(&mut serde_json::Value),
) -> (
    Result<tuiscotti::approve::AcceptOutcome, ApproveError>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let candidate_bytes = with_rewritten_contract(base, rewrite);
    let digest = sha256_hex(&candidate_bytes);
    let candidate = candidate_file(dir.path(), "candidate.tui.json", &candidate_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    (store.accept(&candidate, &digest, "missing"), dir)
}

/// One named render-contract rewrite for the identity matrix.
type ContractRewrite = Box<dyn Fn(&mut serde_json::Value)>;

/// Font-identity mutations: each changes exactly one font field while the
/// snapshot and image bytes stay untouched.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn contract_font_mutations() -> Vec<(&'static str, ContractRewrite)> {
    vec![
        (
            "fonts digest",
            Box::new(|contract| {
                contract["fonts"][0]["digest"] = serde_json::json!("0".repeat(64));
            }),
        ),
        (
            "fonts face_index",
            Box::new(|contract| {
                let index = contract["fonts"][0]["face_index"]
                    .as_u64()
                    .expect("face index reads");
                contract["fonts"][0]["face_index"] = serde_json::json!(index + 1);
            }),
        ),
        (
            "fonts bold",
            Box::new(|contract| {
                let bold = contract["fonts"][0]["bold"].as_bool().expect("bold reads");
                contract["fonts"][0]["bold"] = serde_json::json!(!bold);
            }),
        ),
        (
            "fonts fallback_position",
            Box::new(|contract| {
                let position = contract["fonts"][0]["fallback_position"]
                    .as_u64()
                    .expect("fallback position reads");
                contract["fonts"][0]["fallback_position"] = serde_json::json!(position + 1);
            }),
        ),
    ]
}

/// Non-font material contract mutations: each changes exactly one identity
/// field while the snapshot and image bytes stay untouched.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn contract_field_mutations() -> Vec<(&'static str, ContractRewrite)> {
    vec![
        (
            "renderer_version",
            Box::new(|contract| {
                contract["renderer_version"] = serde_json::json!("0.0.0-fake");
            }),
        ),
        (
            "raster_policy",
            Box::new(|contract| {
                contract["raster_policy"] = serde_json::json!("pinned-v0");
            }),
        ),
        (
            "cell_width",
            Box::new(|contract| {
                contract["cell_width"] = serde_json::json!(11.0);
            }),
        ),
        (
            "cell_height",
            Box::new(|contract| {
                contract["cell_height"] = serde_json::json!(21.0);
            }),
        ),
        (
            "font_size",
            Box::new(|contract| {
                contract["font_size"] = serde_json::json!(17.0);
            }),
        ),
        (
            "baseline",
            Box::new(|contract| {
                contract["baseline"] = serde_json::json!(16.0);
            }),
        ),
        (
            "scale",
            Box::new(|contract| {
                contract["scale"] = serde_json::json!(2.0);
            }),
        ),
        (
            "title_bar",
            Box::new(|contract| {
                let shown = contract["title_bar"].as_bool().expect("title bar reads");
                contract["title_bar"] = serde_json::json!(!shown);
            }),
        ),
        (
            "palette_digest",
            Box::new(|contract| {
                contract["palette_digest"] = serde_json::json!("f".repeat(64));
            }),
        ),
        (
            "cursor_phase",
            Box::new(|contract| {
                let shown = contract["cursor_phase"].as_str() == Some("shown");
                contract["cursor_phase"] =
                    serde_json::json!(if shown { "hidden" } else { "shown" });
            }),
        ),
        (
            "blink",
            Box::new(|contract| {
                contract["blink"] = serde_json::json!(match contract["blink"].as_bool() {
                    Some(blink) => !blink,
                    None => true,
                });
            }),
        ),
    ]
}

/// Rework B4: every material render-contract field rejects independently
/// with identical PNG bytes. A recorded identity that disagrees with the
/// re-rendered identity fails pre-commit with `INCONSISTENT_SAMPLE` and
/// writes no baseline.
#[test]
fn d04_rework_b4_contract_fields_rejected_with_identical_pixels() {
    let base = pixel_sample_bytes("views/login", &base_snapshot());
    let probe_bytes = pixel_sample_bytes("views/login", &base_snapshot());
    let probe_digest = sha256_hex(&probe_bytes);
    let probe_dir = tempfile::tempdir().expect("tempdir creates");
    let probe_baselines = probe_dir.path().join("baselines");
    let probe_candidate = candidate_file(probe_dir.path(), "candidate.tui.json", &probe_bytes);
    let probe_store = ApprovalStore::new(&probe_baselines).expect("approval store opens");
    if ci_refusal_probed(&probe_store, &probe_candidate, &probe_digest, "missing") {
        return;
    }
    let mut mutations = contract_font_mutations();
    mutations.extend(contract_field_mutations());
    for (field, rewrite) in &mutations {
        let (outcome, dir) = refuse_rewritten_contract(&base, rewrite);
        match outcome {
            Err(error @ ApproveError::InconsistentSample { .. }) => {
                assert_eq!(error.code(), "INCONSISTENT_SAMPLE", "field {field}");
            }
            other => panic!("mutated contract field {field} must fail, got {other:?}"),
        }
        assert_eq!(
            entry_names(&dir.path().join("baselines")),
            [".tuiscotti-approval.lock".to_string()],
            "field {field} writes nothing but the stable lock file"
        );
    }
    // An unknown profile and an unsupported contract version fail pre-commit
    // with their own prescribed codes and also write no baseline.
    for (field, rewrite, code) in [
        (
            "profile",
            Box::new(|contract: &mut serde_json::Value| {
                contract["profile"] = serde_json::json!("no-such-profile");
            }) as ContractRewrite,
            "UNKNOWN_PROFILE",
        ),
        (
            "contract_version",
            Box::new(|contract: &mut serde_json::Value| {
                contract["contract_version"] = serde_json::json!(999u32);
            }),
            "INVALID_SAMPLE",
        ),
    ] {
        let (outcome, dir) = refuse_rewritten_contract(&base, rewrite);
        match outcome {
            Err(error) => {
                assert_eq!(error.code(), code, "field {field}");
            }
            Ok(_) => panic!("unsupported identity {field} must fail"),
        }
        assert_eq!(
            entry_names(&dir.path().join("baselines")),
            [".tuiscotti-approval.lock".to_string()],
            "field {field} writes nothing but the stable lock file"
        );
    }
}

/// Rework B4: a foreign platform key alone never rejects reproducible
/// pixels, and the change summary still compares candidate and baseline
/// contracts.
///
/// Under platform-parameterized validation the foreign platform travels
/// with its agreeing test-ID suffix (see [`with_foreign_platform`]): a
/// platform-only skew at one store path no longer parses, so the
/// contract-summary half skews the renderer version instead — identical
/// snapshot and image bytes, differing recorded identity.
#[test]
fn d04_rework_b4_foreign_platform_accepts_reproducible_pixels() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let local_bytes = pixel_sample_bytes(logical, &base_snapshot());
    let local_digest = sha256_hex(&local_bytes);
    let local = candidate_file(dir.path(), "local.tui.json", &local_bytes);
    let store = ApprovalStore::new(&baselines).expect("approval store opens");
    if ci_refusal_probed(&store, &local, &local_digest, "missing") {
        return;
    }
    // The same pixels recorded on a foreign platform still approve: the
    // foreign test ID agrees with its stored contract identity, and the
    // platform key is addressing metadata, not rendered identity.
    let foreign_id = "views/login/pixels-foreign-os-fake";
    let foreign_bytes = with_foreign_platform(&local_bytes, "foreign-os-fake");
    assert_ne!(foreign_bytes, local_bytes);
    let foreign_digest = sha256_hex(&foreign_bytes);
    let foreign = candidate_file(dir.path(), "foreign.tui.json", &foreign_bytes);
    let first = accepted_of(store.accept(&foreign, &foreign_digest, "missing"));
    assert_eq!(first.test_id(), foreign_id);
    assert_eq!(first.baseline_sha256(), foreign_digest);
    assert_eq!(first.reason(), Some(MismatchReason::Missing));
    assert_eq!(read_baseline_file(&baselines, foreign_id), foreign_bytes);
    // The recorded foreign platform now matches its own baseline: the
    // summary is empty and the pixels still reproduce.
    let same = accepted_of(store.accept(&foreign, &foreign_digest, &foreign_digest));
    assert_eq!(same.reason(), None);
    assert!(same.changed_fields().is_empty());
    // The change summary compares contracts even when pixels reproduce: a
    // baseline skewed only in its recorded renderer version (identical
    // snapshot and image bytes) approves with a `Contract` change, not a
    // silent pass.
    let skewed = with_rewritten_contract(&local_bytes, |contract| {
        contract["renderer_version"] = serde_json::json!("0.0.0-fake");
    });
    assert_ne!(skewed, local_bytes);
    let skewed_digest = sha256_hex(&skewed);
    write_baseline_file(&baselines, &test_id, &skewed);
    let accepted = accepted_of(store.accept(&local, &local_digest, &skewed_digest));
    assert_eq!(accepted.test_id(), test_id);
    assert_eq!(accepted.baseline_sha256(), local_digest);
    assert_eq!(
        accepted.reason(),
        Some(MismatchReason::Contract),
        "the change summary compares contracts even when pixels reproduce"
    );
    assert_eq!(accepted.changed_fields(), &["render_contract".to_string()]);
    // The genuine candidate now matches its own baseline: the summary is
    // empty and the pixels still reproduce.
    let second = accepted_of(store.accept(&local, &local_digest, &local_digest));
    assert_eq!(second.reason(), None);
    assert!(second.changed_fields().is_empty());
    assert_eq!(read_baseline_file(&baselines, &test_id), local_bytes);
}
