//! Shared sample fixtures for the offline CLI tests: snapshots and
//! versioned samples built through the public library API, plus
//! tempdir stores and tree listings.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
    SnapshotParts, SnapshotScope, TerminalSnapshot,
};
use tuiscotti::render_profile::{PROFILE_NAME, platform_key, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::ViewSettings;

/// The fixed built-in view palette: no child or session.
fn test_palette() -> [Rgb; 256] {
    *ViewSettings::builtin().palette()
}

/// Hidden cursor fixture.
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
#[expect(clippy::expect_used, reason = "fixture inputs are statically valid")]
pub(crate) fn plain_cell(text: &str, width: u16) -> SnapshotCell {
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

/// Snapshot fixture: a 4-column, 2-row grid filled with one grapheme.
pub(crate) fn base_snapshot() -> TerminalSnapshot {
    let cells: Vec<SnapshotCell> = (0..8).map(|_| plain_cell("a", 1)).collect();
    snapshot_with_cells(cells)
}

/// Snapshot fixture with explicit cells on the 4x2 grid.
#[expect(clippy::expect_used, reason = "fixture snapshots are valid")]
pub(crate) fn snapshot_with_cells(cells: Vec<SnapshotCell>) -> TerminalSnapshot {
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
        metadata: SnapshotMetadata::default(),
    })
    .expect("fixture snapshot is valid")
}

/// State sample bytes for a test ID.
#[expect(clippy::expect_used, reason = "fixture samples are valid")]
pub(crate) fn state_sample_bytes(test_id: &str, snapshot: &TerminalSnapshot) -> Vec<u8> {
    Sample::state(test_id, snapshot.clone(), Observation::empty())
        .expect("fixture state sample is valid")
        .to_bytes()
        .expect("fixture state sample serializes")
}

/// State-and-pixels sample bytes for a logical test ID on this
/// platform; returns the full test ID plus the bytes.
#[expect(clippy::expect_used, reason = "fixture samples are valid")]
pub(crate) fn pixel_sample_bytes(
    logical_id: &str,
    snapshot: &TerminalSnapshot,
) -> (String, Vec<u8>) {
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let test_id = format!("{logical_id}/pixels-{}", platform_key());
    let bytes = Sample::with_pixels(&test_id, snapshot, Observation::empty(), profile)
        .expect("fixture pixel sample is valid")
        .to_bytes()
        .expect("fixture pixel sample serializes");
    (test_id, bytes)
}

/// SHA-256 over sample file bytes: the candidate and baseline digests.
#[must_use]
pub(crate) fn candidate_id(bytes: &[u8]) -> String {
    Sample::candidate_id(bytes)
}

/// Fresh tempdir with baseline and artifact roots.
#[expect(clippy::expect_used, reason = "test tempdirs must create")]
pub(crate) fn temp_store() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creates");
    let baselines = dir.path().join("baselines");
    let artifacts = dir.path().join("artifacts");
    std::fs::create_dir_all(&baselines).expect("baseline root creates");
    (dir, baselines, artifacts)
}

/// Write baseline bytes for a test ID, making parent directories.
#[expect(clippy::expect_used, reason = "test baselines must write")]
pub(crate) fn write_baseline_file(root: &Path, test_id: &str, bytes: &[u8]) {
    let mut path = root.join(test_id);
    path.set_extension("tui.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("baseline parent creates");
    }
    std::fs::write(&path, bytes).expect("baseline writes");
}

/// Write bytes to a file inside a directory, returning its path.
#[expect(clippy::expect_used, reason = "test candidates must write")]
pub(crate) fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("test parent creates");
    }
    let mut file = std::fs::File::create(&path).expect("test file creates");
    file.write_all(bytes).expect("test file writes");
    path
}

/// Every file under a root, relative and sorted.
#[expect(clippy::expect_used, reason = "test trees must list")]
pub(crate) fn all_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("root lists")
            .map(|entry| entry.expect("entry reads").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    out.sort();
    out
}
