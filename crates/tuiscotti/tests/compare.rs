//! D03 comparison tests: frozen checking, candidate evidence, store rules.
//!
//! Covers the D03 subcases from `docs/migration/corpus.json`:
//!
//! - C07: metadata-only changes pass, visual changes fail with a reason.
//! - C08: equal RGBA passes, one changed channel fails via `compare_rgba`.
//! - C09: a missing baseline fails without a baseline write.
//! - C11: one candidate with two baseline revisions keeps both reports;
//!   concurrent checks stay isolated per baseline.
//!
//! Also: ID grammar rejections (full Windows-device list), equal and
//! contained root rejection, symlink-escape rejection, read-only baseline
//! roots (no lock file, no writes), evidence re-validation (typed errors),
//! and HTML escaping of hostile state. Barriers order concurrency;
//! tempdirs isolate the filesystem; no test changes process-wide state.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use base64::Engine as _;
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
    SnapshotParts, SnapshotScope, SnapshotSource, SnapshotUnderline, TerminalSnapshot,
};
use tuiscotti::check::{CheckError, CheckFailure, CheckOutcome, FrozenStore, MismatchReason};
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
        SnapshotUnderline::None,
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
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn snapshot_with_cells(cells: Vec<SnapshotCell>, metadata: SnapshotMetadata) -> TerminalSnapshot {
    TerminalSnapshot::try_new(SnapshotParts {
        cols: 4,
        rows: 2,
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

/// Observation metadata that differs from the default without moving state.
fn revised_metadata() -> SnapshotMetadata {
    SnapshotMetadata {
        captured_at_ms: 9,
        revision: 3,
        backend: "test-backend".to_string(),
        session: "test-session".to_string(),
        source: SnapshotSource::Live,
    }
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

/// SHA-256 hex over bytes, matching the candidate and baseline keying.
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

/// Unwrap a failing outcome for evidence assertions.
#[expect(
    clippy::panic,
    reason = "test helper rejects an impossible passing outcome"
)]
fn failure_of(outcome: CheckOutcome) -> CheckFailure {
    match outcome {
        CheckOutcome::Failure(failure) => failure,
        CheckOutcome::Pass(_) => panic!("check passed but the test needs a failure"),
    }
}

/// C07: capture metadata and sample observation change nothing; equal state
/// passes under the state gate.
#[test]
fn d03_c07_metadata_only_change_passes() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let revised = snapshot_with_cells(filled_cells("a"), revised_metadata());
    let actual_bytes = Sample::state("views/login", revised, Observation::noted("reviewed"))
        .expect("metadata sample is valid")
        .to_bytes()
        .expect("metadata sample serializes");
    assert_ne!(sha256_hex(&baseline_bytes), sha256_hex(&actual_bytes));
    match store
        .check(&actual_bytes)
        .expect("metadata-only check runs")
    {
        CheckOutcome::Pass(success) => {
            assert_eq!(success.test_id(), "views/login");
            assert_eq!(success.candidate_sha256(), sha256_hex(&actual_bytes));
            assert_eq!(success.baseline_sha256(), sha256_hex(&baseline_bytes));
        }
        CheckOutcome::Failure(_) => panic!("metadata-only change must pass"),
    }
}

/// C07: one changed cell fails with the state reason and saved evidence.
#[test]
fn d03_c07_state_change_fails_with_reason() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let mut changed = filled_cells("a");
    changed[3] = plain_cell("b", 1);
    let changed_snapshot = snapshot_with_cells(changed, SnapshotMetadata::default());
    let actual_bytes = state_sample_bytes("views/login", &changed_snapshot);
    let failure = failure_of(store.check(&actual_bytes).expect("state check runs"));
    assert_eq!(failure.reason(), MismatchReason::State);
    assert_eq!(failure.changed_cell_total(), 1);
    assert_eq!(failure.changed_cells().len(), 1);
    assert_eq!(failure.changed_cells()[0].col(), 3);
    assert_eq!(failure.changed_cells()[0].row(), 0);
    assert!(
        failure.changed_cells()[0]
            .fields()
            .contains(&"text".to_string())
    );
    assert!(failure.candidate_path().exists());
    assert!(failure.report_path().exists());
    let reread = std::fs::read(baselines.join("views/login.tui.json")).expect("baseline reads");
    assert_eq!(reread, baseline_bytes);
}

/// C08: equal decoded RGBA passes under the pixel gate.
#[test]
fn d03_c08_equal_pixels_pass() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let baseline_bytes = pixel_sample_bytes(logical, &base_snapshot());
    write_baseline_file(&baselines, &test_id, &baseline_bytes);
    match store.check(&baseline_bytes).expect("pixel check runs") {
        CheckOutcome::Pass(success) => {
            assert_eq!(success.baseline_sha256(), sha256_hex(&baseline_bytes));
        }
        CheckOutcome::Failure(_) => panic!("equal pixels must pass"),
    }
}

/// Flip the least significant bit of one red channel in the sample image,
/// keeping state and contract equal with fresh digests.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn mutate_one_channel(sample_bytes: &[u8]) -> Vec<u8> {
    use image::ImageEncoder as _;
    let mut value: serde_json::Value =
        serde_json::from_slice(sample_bytes).expect("sample parses as JSON");
    let encoded = value["image"]["png_base64"]
        .as_str()
        .expect("sample carries base64 PNG")
        .to_string();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .expect("sample PNG decodes from base64");
    let decoded = image::load_from_memory(&raw).expect("sample PNG decodes");
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    let mut pixels = rgba.into_raw();
    pixels[0] ^= 0x01;
    let mut fresh = Vec::new();
    image::codecs::png::PngEncoder::new(&mut fresh)
        .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
        .expect("mutated PNG encodes");
    value["image"]["png_base64"] =
        serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(&fresh));
    value["image"]["png_sha256"] = serde_json::Value::String(sha256_hex(&fresh));
    value["image"]["rgba_sha256"] = serde_json::Value::String(sha256_hex(&pixels));
    serde_json::to_vec(&value).expect("mutated sample serializes")
}

/// C08: one changed channel fails with the pixel reason and a diff image.
#[test]
fn d03_c08_one_channel_pixel_fails() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let logical = "views/login";
    let test_id = format!("{logical}/pixels-{}", platform_key());
    let baseline_bytes = pixel_sample_bytes(logical, &base_snapshot());
    write_baseline_file(&baselines, &test_id, &baseline_bytes);
    let actual_bytes = mutate_one_channel(&baseline_bytes);
    let failure = failure_of(store.check(&actual_bytes).expect("pixel check runs"));
    assert_eq!(failure.reason(), MismatchReason::Pixels);
    assert_eq!(failure.pixel_changed(), 1);
    assert_eq!(failure.changed_cell_total(), 0);
    assert!(failure.evidence_dir().join("diff.png").exists());
    assert!(failure.evidence_dir().join("expected.png").exists());
    assert!(failure.evidence_dir().join("actual.png").exists());
}

/// C09: a missing baseline fails without making the baseline directory.
#[test]
fn d03_c09_missing_baseline_writes_no_baseline() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let before = entry_names(&baselines);
    let actual_bytes = state_sample_bytes("views/login", &base_snapshot());
    let failure = failure_of(store.check(&actual_bytes).expect("missing check runs"));
    assert_eq!(failure.reason(), MismatchReason::Missing);
    assert_eq!(failure.baseline_sha256(), "missing");
    assert_eq!(entry_names(&baselines), before);
    assert!(!baselines.join("views").exists());
    assert!(failure.candidate_path().exists());
    let check_dir = failure.evidence_dir();
    assert!(check_dir.ends_with("checks/missing"));
    assert!(check_dir.join("result.json").exists());
    assert!(check_dir.join("report.html").exists());
    assert!(!check_dir.join("expected.tui.json").exists());
    let report = std::fs::read_to_string(check_dir.join("report.html")).expect("report reads");
    assert!(report.contains("The expected image is missing"));
}

/// C11: one candidate with two baseline revisions keeps both reports.
#[test]
fn d03_c11_two_revisions_both_reports_available() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("b"), SnapshotMetadata::default()),
    );
    let revision_a = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &revision_a);
    let first = failure_of(store.check(&candidate_bytes).expect("first check runs"));
    let digest_a = sha256_hex(&revision_a);
    assert_eq!(first.baseline_sha256(), digest_a);
    let revision_c = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("c"), SnapshotMetadata::default()),
    );
    write_baseline_file(&baselines, "views/login", &revision_c);
    let second = failure_of(store.check(&candidate_bytes).expect("second check runs"));
    let digest_c = sha256_hex(&revision_c);
    assert_eq!(second.baseline_sha256(), digest_c);
    let first_report = std::fs::read_to_string(first.report_path()).expect("first report reads");
    let second_report = std::fs::read_to_string(second.report_path()).expect("second report reads");
    assert!(first_report.contains(&digest_a));
    assert!(!first_report.contains(&digest_c));
    assert!(second_report.contains(&digest_c));
    assert!(!second_report.contains(&digest_a));
    assert!(first.report_path().exists());
    assert!(second.report_path().exists());
}

/// C11: concurrent checks against different baselines stay isolated.
#[test]
fn d03_c11_concurrent_checks_isolated() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let base_a = dir.path().join("base-a");
    let base_c = dir.path().join("base-c");
    let artifacts = dir.path().join("artifacts");
    std::fs::create_dir_all(&base_a).expect("base-a creates");
    std::fs::create_dir_all(&base_c).expect("base-c creates");
    let candidate_bytes = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("b"), SnapshotMetadata::default()),
    );
    let revision_a = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&base_a, "views/login", &revision_a);
    let revision_c = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(filled_cells("c"), SnapshotMetadata::default()),
    );
    write_baseline_file(&base_c, "views/login", &revision_c);
    let barrier = Arc::new(Barrier::new(2));
    let spawn_check =
        |root: PathBuf, artifacts: PathBuf, candidate: Vec<u8>, barrier: Arc<Barrier>| {
            thread::spawn(move || {
                let store = FrozenStore::new(&root, &artifacts).expect("frozen store opens");
                barrier.wait();
                failure_of(store.check(&candidate).expect("concurrent check runs"))
            })
        };
    let first_handle = spawn_check(
        base_a,
        artifacts.clone(),
        candidate_bytes.clone(),
        Arc::clone(&barrier),
    );
    let second_handle = spawn_check(base_c, artifacts, candidate_bytes, Arc::clone(&barrier));
    let first = first_handle.join().expect("first thread joins");
    let second = second_handle.join().expect("second thread joins");
    let report_a = std::fs::read_to_string(first.report_path()).expect("report A reads");
    let report_c = std::fs::read_to_string(second.report_path()).expect("report C reads");
    assert!(report_a.contains(&first.baseline_sha256().to_string()));
    assert!(!report_a.contains(&second.baseline_sha256().to_string()));
    assert!(report_c.contains(&second.baseline_sha256().to_string()));
    assert!(!report_c.contains(&first.baseline_sha256().to_string()));
}

/// Replace the test ID inside valid sample bytes without re-validation.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn sample_bytes_with_id(sample_bytes: &[u8], test_id: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(sample_bytes).expect("sample parses as JSON");
    value["test_id"] = serde_json::Value::String(test_id.to_string());
    serde_json::to_vec(&value).expect("surgery serializes")
}

/// The full Section 14.1 reserved device list for this-platform coverage.
fn device_names() -> Vec<String> {
    let mut names = vec![
        "con".to_string(),
        "prn".to_string(),
        "aux".to_string(),
        "nul".to_string(),
    ];
    for port in 1..=9 {
        names.push(format!("com{port}"));
        names.push(format!("lpt{port}"));
    }
    names
}

/// IDs that parse as samples must fail with the strict grammar rejection.
#[expect(clippy::panic, reason = "test helper rejects an accepted bad ID")]
fn assert_strict_rejection(store: &FrozenStore, sample_bytes: &[u8], test_id: &str) {
    let bytes = sample_bytes_with_id(sample_bytes, test_id);
    match store.check(&bytes) {
        Err(CheckError::InvalidTestId { .. }) => {}
        Err(CheckError::Sample(_)) => {
            assert!(
                Sample::from_bytes(&bytes).is_err(),
                "test ID {test_id:?} parses but escapes strict rejection"
            );
        }
        Err(other) => panic!("test ID {test_id:?} fails with {other:?}"),
        Ok(_) => panic!("test ID {test_id:?} must fail"),
    }
}

/// ID grammar rejections, including the full Windows-device list.
#[test]
fn d03_id_grammar_rejections() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let sample_bytes = state_sample_bytes("views/login", &base_snapshot());
    for invalid in [
        "",
        "/views/login",
        "views/login/",
        "views//login",
        "views/./login",
        "views/../login",
        ".",
        "..",
        "views\\login",
        "views/Login",
        "views/log in",
    ] {
        assert_strict_rejection(&store, &sample_bytes, invalid);
    }
    for name in device_names() {
        assert_eq!(device_names().len(), 22);
        assert_strict_rejection(&store, &sample_bytes, &name);
        assert_strict_rejection(&store, &sample_bytes, &format!("views/{name}"));
        assert_strict_rejection(&store, &sample_bytes, &name.to_ascii_uppercase());
    }
}

/// Equal and contained roots are rejected before any comparison.
#[test]
fn d03_equal_and_contained_roots_rejected() {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let outer = dir.path().join("outer");
    let inner = outer.join("inner");
    std::fs::create_dir_all(&inner).expect("roots create");
    for (base, artifacts) in [
        (outer.clone(), outer.clone()),
        (outer.clone(), inner.clone()),
        (inner.clone(), outer.clone()),
    ] {
        match FrozenStore::new(&base, &artifacts) {
            Err(CheckError::InvalidStorePath { .. }) => {}
            Err(other) => panic!("roots fail with {other:?}"),
            Ok(_) => panic!("equal or contained roots must fail"),
        }
    }
}

/// A symlink in a store-relative path cannot redirect evidence writes.
#[cfg(unix)]
#[test]
fn d03_symlink_escape_rejected() {
    use std::os::unix::fs::symlink;
    let (_temp, baselines, artifacts) = temp_store();
    let outside = tempfile::tempdir().expect("outside tempdir creates");
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    symlink(outside.path(), artifacts.join("evil")).expect("symlink creates");
    let actual_bytes = state_sample_bytes("evil/x", &base_snapshot());
    match store.check(&actual_bytes) {
        Err(CheckError::InvalidStorePath { .. }) => {}
        Err(other) => panic!("symlinked artifact path fails with {other:?}"),
        Ok(_) => panic!("symlinked artifact path must fail"),
    }
    symlink(outside.path(), baselines.join("runaway")).expect("symlink creates");
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("runaway/x", &base_snapshot());
    std::fs::write(outside.path().join("x.tui.json"), &baseline_bytes)
        .expect("outside baseline writes");
    let mut changed = filled_cells("a");
    changed[0] = plain_cell("b", 1);
    let changed_snapshot = snapshot_with_cells(changed, SnapshotMetadata::default());
    let actual_bytes = state_sample_bytes("runaway/x", &changed_snapshot);
    match store.check(&actual_bytes) {
        Err(CheckError::InvalidStorePath { .. }) => {}
        Err(other) => panic!("symlinked baseline path fails with {other:?}"),
        Ok(_) => panic!("symlinked baseline path must fail"),
    }
}

/// A read-only baseline root still checks: no writes, no lock file.
#[cfg(unix)]
#[test]
fn d03_read_only_baseline_root_checks_without_writes() {
    use std::os::unix::fs::PermissionsExt as _;
    let (_temp, baselines, artifacts) = temp_store();
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let before = entry_names(&baselines);
    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o555))
        .expect("root turns read-only");
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let outcome = store.check(&baseline_bytes).expect("read-only check runs");
    assert!(outcome.is_pass());
    let after = entry_names(&baselines);
    std::fs::set_permissions(&baselines, std::fs::Permissions::from_mode(0o755))
        .expect("root restores");
    assert_eq!(after, before);
    assert!(!after.iter().any(|name| name.contains("lock")));
}

/// Corrupt saved evidence is a typed error, never a panic or overwrite.
#[test]
fn d03_corrupt_saved_evidence_is_typed_error() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let mut changed = filled_cells("a");
    changed[0] = plain_cell("b", 1);
    let changed_snapshot = snapshot_with_cells(changed, SnapshotMetadata::default());
    let actual_bytes = state_sample_bytes("views/login", &changed_snapshot);
    let failure = failure_of(store.check(&actual_bytes).expect("first check runs"));
    std::fs::write(
        failure.evidence_dir().join("expected.tui.json"),
        b"not json",
    )
    .expect("evidence corrupts");
    match store.check(&actual_bytes) {
        Err(CheckError::EvidenceCorrupt { .. }) => {}
        Err(other) => panic!("corrupt evidence fails with {other:?}"),
        Ok(_) => panic!("corrupt evidence must fail"),
    }
}

/// Hostile state text cannot break markup or execute in the report.
#[test]
fn d03_html_escapes_hostile_state() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let hostile_snapshot = TerminalSnapshot::try_new(SnapshotParts {
        cols: 4,
        rows: 2,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells: hostile_cells(),
        palette: test_palette(),
        default_foreground: Rgb::new(192, 192, 192),
        default_background: Rgb::new(0, 0, 0),
        default_cursor: Rgb::new(192, 192, 192),
        cursor: hidden_cursor(),
        title: Some("\"><img src=x onerror=alert(2)>".to_string()),
        metadata: SnapshotMetadata::default(),
    })
    .expect("hostile snapshot is valid");
    let actual_bytes = state_sample_bytes("views/login", &hostile_snapshot);
    let failure = failure_of(store.check(&actual_bytes).expect("hostile check runs"));
    assert_eq!(failure.reason(), MismatchReason::State);
    let report = std::fs::read_to_string(failure.report_path()).expect("report reads");
    assert!(report.contains("&lt;script&gt;"));
    assert!(!report.contains("<script"));
    assert!(!report.contains("\"><img src=x"));
    assert!(!report.contains(artifacts.to_string_lossy().as_ref()));
}

/// Hostile cells shared by the escaping test: one script cell, seven plain.
#[expect(clippy::expect_used, reason = "test fixture setup")]
fn hostile_cells() -> Vec<SnapshotCell> {
    let mut cells = filled_cells("a");
    cells[0] = SnapshotCell::new(
        "<script>alert('x')</script>".to_string(),
        1,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        SnapshotUnderline::None,
        SnapshotColor::Default,
        Some("\"><img src=x onerror=alert(2)>".to_string()),
        None,
    )
    .expect("hostile cell is valid");
    cells
}

/// No `.tmp-*` file may survive beside saved evidence.
#[expect(clippy::expect_used, reason = "test helper reads evidence entries")]
fn assert_no_temp_residue(dir: &Path) {
    for entry in std::fs::read_dir(dir).expect("evidence lists") {
        let name = entry
            .expect("entry reads")
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.starts_with(".tmp-"),
            "temporary file {name:?} left behind in {}",
            dir.display()
        );
    }
}

/// The failing-check report saves atomically: complete bytes, stable across
/// a re-check, with no temporary file left behind in the evidence dir.
#[test]
fn d03_report_html_saves_atomically_without_temp_residue() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let mut changed = filled_cells("a");
    changed[0] = plain_cell("b", 1);
    let changed_snapshot = snapshot_with_cells(changed, SnapshotMetadata::default());
    let actual_bytes = state_sample_bytes("views/login", &changed_snapshot);
    let failure = failure_of(store.check(&actual_bytes).expect("state check runs"));
    let first = std::fs::read(failure.report_path()).expect("report reads");
    let text = String::from_utf8(first.clone()).expect("report is UTF-8");
    assert!(text.starts_with("<!DOCTYPE html>"));
    assert!(text.ends_with("</html>\n"));
    assert!(text.contains("views/login"));
    assert!(text.contains(&sha256_hex(&actual_bytes)));
    assert!(text.contains(failure.baseline_sha256()));
    assert_no_temp_residue(failure.evidence_dir());
    let recheck = failure_of(store.check(&actual_bytes).expect("recheck runs"));
    assert_eq!(recheck.report_path(), failure.report_path());
    let second = std::fs::read(failure.report_path()).expect("report rereads");
    assert_eq!(second, first);
    assert_no_temp_residue(failure.evidence_dir());
}

/// Snapshot fixture on an explicit grid, filled with one grapheme.
#[expect(clippy::expect_used, reason = "rework validation fixture")]
fn snapshot_grid(cols: u32, rows: u32, fill: &str) -> TerminalSnapshot {
    let count =
        usize::try_from(cols).expect("grid fits") * usize::try_from(rows).expect("grid fits");
    let cells = vec![plain_cell(fill, 1); count];
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
        metadata: SnapshotMetadata::default(),
    })
    .expect("grid snapshot is valid")
}

/// R1: grids of different sizes diff the union of occupied cells only.
/// A 6x2 actual against a 2x6 baseline covers 12 + 12 - 4 = 20 cells;
/// the 16 cells outside both grids must not count as changed.
#[test]
fn r1_mismatched_grids_count_union_only() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/grid", &snapshot_grid(2, 6, "b"));
    write_baseline_file(&baselines, "views/grid", &baseline_bytes);
    let actual_bytes = state_sample_bytes("views/grid", &snapshot_grid(6, 2, "a"));
    let failure = failure_of(store.check(&actual_bytes).expect("grid check runs"));
    assert_eq!(failure.reason(), MismatchReason::State);
    assert_eq!(failure.changed_cell_total(), 20);
    assert_eq!(failure.changed_cells().len(), 20);
    assert!(
        !failure
            .changed_cells()
            .iter()
            .any(|cell| cell.actual_text().is_empty() && cell.baseline_text().is_empty()),
        "no (None, None) phantom cell is reported"
    );
}

/// R1: the total stays exact past the 64-detail cap on mismatched grids.
/// A 20x4 actual against a 4x20 baseline covers 80 + 80 - 16 = 144 cells.
#[test]
fn r1_mismatched_grids_keep_exact_total_past_detail_cap() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/wide", &snapshot_grid(4, 20, "b"));
    write_baseline_file(&baselines, "views/wide", &baseline_bytes);
    let actual_bytes = state_sample_bytes("views/wide", &snapshot_grid(20, 4, "a"));
    let failure = failure_of(store.check(&actual_bytes).expect("wide check runs"));
    assert_eq!(failure.changed_cell_total(), 144);
    assert_eq!(failure.changed_cells().len(), 64);
}

/// R4: an oversize baseline file fails as a resource limit, not a silent
/// read. The 64 MiB + 1 byte file must never fully load into a sample,
/// while exactly 64 MiB still loads (trailing JSON whitespace parses).
#[test]
fn r4_oversize_baseline_fails_as_resource_limit() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_snapshot = base_snapshot();
    let candidate_bytes = state_sample_bytes("views/big", &baseline_snapshot);
    let mut exact = state_sample_bytes("views/big", &baseline_snapshot);
    exact.resize(64 * 1024 * 1024, b' ');
    write_baseline_file(&baselines, "views/big", &exact);
    assert!(
        matches!(
            store.check(&candidate_bytes),
            Ok(CheckOutcome::Pass(_))
        ),
        "exact-limit baseline loads and passes"
    );
    let mut big = state_sample_bytes("views/big", &baseline_snapshot);
    big.resize(64 * 1024 * 1024 + 1, b' ');
    write_baseline_file(&baselines, "views/big", &big);
    let error = store
        .check(&candidate_bytes)
        .expect_err("oversize baseline must fail");
    assert!(
        matches!(
            error,
            CheckError::Sample(tuiscotti::sample::SampleError::TooLarge { .. })
        ),
        "oversize baseline is a resource limit, got {error:?}"
    );
}

/// R4: a baseline path that is not a missing file but an unreadable
/// directory reports I/O, never a missing baseline.
#[test]
fn r4_unreadable_baseline_path_reports_io_not_missing() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let candidate_bytes = state_sample_bytes("views/dir", &base_snapshot());
    let mut path = baselines.join("views/dir");
    path.set_extension("tui.json");
    std::fs::create_dir_all(&path).expect("directory blocks the baseline path");
    let error = store
        .check(&candidate_bytes)
        .expect_err("directory baseline must fail");
    assert!(
        matches!(error, CheckError::Io { .. }),
        "directory baseline is I/O, got {error:?}"
    );
}

/// R22: a truncated `report.html` fails re-validation as corrupt evidence
/// instead of being silently trusted.
#[test]
fn r22_truncated_report_fails_as_corrupt_evidence() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let mut changed = filled_cells("a");
    changed[0] = plain_cell("b", 1);
    let actual_bytes =
        state_sample_bytes(
            "views/login",
            &snapshot_with_cells(changed, SnapshotMetadata::default())
        );
    let failure = failure_of(store.check(&actual_bytes).expect("state check runs"));
    std::fs::write(failure.report_path(), b"<html>truncated").expect("report truncates");
    let error = store
        .check(&actual_bytes)
        .expect_err("truncated report must fail");
    assert!(
        matches!(error, CheckError::EvidenceCorrupt { .. }),
        "truncated report is corrupt evidence, got {error:?}"
    );
}

/// R22: a `report.html` from another baseline revision disagrees with the
/// expected bytes and fails as corrupt evidence, without overwrite.
#[test]
fn r22_wrong_baseline_report_fails_without_overwrite() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let revision_a = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &revision_a);
    let candidate_bytes = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(
            {
                let mut changed = filled_cells("a");
                changed[0] = plain_cell("b", 1);
                changed
            },
            SnapshotMetadata::default(),
        ),
    );
    let first = failure_of(store.check(&candidate_bytes).expect("first check runs"));
    let first_report = std::fs::read(first.report_path()).expect("first report reads");
    let revision_c = state_sample_bytes(
        "views/login",
        &snapshot_with_cells(
            {
                let mut changed = filled_cells("c");
                changed[0] = plain_cell("d", 1);
                changed
            },
            SnapshotMetadata::default()),
    );
    write_baseline_file(&baselines, "views/login", &revision_c);
    let second = failure_of(store.check(&candidate_bytes).expect("second check runs"));
    assert_ne!(first.report_path(), second.report_path());
    let second_report = std::fs::read(second.report_path()).expect("second report reads");
    assert_ne!(first_report, second_report);
    std::fs::write(first.report_path(), &second_report).expect("wrong report plants");
    write_baseline_file(&baselines, "views/login", &revision_a);
    let error = store
        .check(&candidate_bytes)
        .expect_err("wrong-baseline report must fail");
    assert!(
        matches!(error, CheckError::EvidenceCorrupt { .. }),
        "wrong-baseline report is corrupt evidence, got {error:?}"
    );
    assert_eq!(
        std::fs::read(first.report_path()).expect("planted report rereads"),
        second_report,
        "conflicting immutable evidence is never overwritten"
    );
}

/// R22: a non-regular `report.html` destination fails instead of being
/// trusted or replaced.
#[test]
fn r22_nonregular_report_destination_fails() {
    let (_temp, baselines, artifacts) = temp_store();
    let store = FrozenStore::new(&baselines, &artifacts).expect("frozen store opens");
    let baseline_bytes = state_sample_bytes("views/login", &base_snapshot());
    write_baseline_file(&baselines, "views/login", &baseline_bytes);
    let mut changed = filled_cells("a");
    changed[0] = plain_cell("b", 1);
    let actual_bytes =
        state_sample_bytes(
            "views/login",
            &snapshot_with_cells(changed, SnapshotMetadata::default())
        );
    let failure = failure_of(store.check(&actual_bytes).expect("state check runs"));
    std::fs::remove_file(failure.report_path()).expect("report removes");
    std::fs::create_dir(failure.report_path()).expect("directory blocks the report path");
    let error = store
        .check(&actual_bytes)
        .expect_err("directory report must fail");
    assert!(
        matches!(error, CheckError::EvidenceCorrupt { .. }),
        "directory report is corrupt evidence, got {error:?}"
    );
}
