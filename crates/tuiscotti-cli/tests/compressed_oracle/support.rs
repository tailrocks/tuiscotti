use std::path::Path;
use std::process::{Command, Output, Stdio};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use sha2::{Digest, Sha256};
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursor, SnapshotCursorShape, SnapshotMetadata,
    SnapshotParts, SnapshotScope, SnapshotSource, SnapshotUnderline, TerminalSnapshot,
};
use tuiscotti::approve::ApprovalStore;
use tuiscotti::check::{CheckOutcome, MismatchReason};
use tuiscotti::render_profile::{PROFILE_NAME, resolve_profile};
use tuiscotti::sample::{Observation, Sample};
use tuiscotti::view::{ViewSettings, capture_view, snapshot_from_buffer};

const HEX: &[u8; 16] = b"0123456789abcdef";

pub(super) fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub(super) fn state_sample(cell: char, note: Option<&str>) -> Sample {
    let observation = note.map_or_else(Observation::empty, Observation::noted);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(0, 0)].set_char(cell);
    let snapshot =
        snapshot_from_buffer(&buffer, &ViewSettings::builtin()).expect("state snapshot is valid");
    Sample::state("oracle/state", snapshot, observation).expect("state sample builds")
}

pub(super) fn snapshot_with_capture_time(captured_at_ms: u64) -> TerminalSnapshot {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(0, 0)].set_char('A');
    let captured = snapshot_from_buffer(&buffer, &ViewSettings::builtin())
        .expect("metadata snapshot source is valid");
    let mut metadata = captured.metadata().clone();
    metadata.captured_at_ms = captured_at_ms;
    TerminalSnapshot::try_new(SnapshotParts {
        cols: captured.cols(),
        rows: captured.rows(),
        scope: captured.scope(),
        viewport_origin: captured.viewport_origin(),
        cells: captured.cells().to_vec(),
        palette: *captured.palette(),
        default_foreground: captured.default_foreground(),
        default_background: captured.default_background(),
        default_cursor: captured.default_cursor(),
        cursor: captured.cursor().clone(),
        title: captured.title().map(str::to_string),
        metadata,
    })
    .expect("metadata-only snapshot remains valid")
}

#[expect(
    clippy::too_many_lines,
    reason = "the fixture keeps every rich SnapshotParts value together"
)]
pub(super) fn rich_snapshot(changed: bool) -> TerminalSnapshot {
    let palette = std::array::from_fn(|index| {
        let index = u8::try_from(index).expect("palette index is u8");
        Rgb::new(index, index.wrapping_mul(3), 255_u8.wrapping_sub(index))
    });
    let mut palette = palette;
    if changed {
        palette[17] = Rgb::new(1, 2, 3);
    }
    let mut attrs = Attrs::empty();
    attrs |= Attrs::BOLD | Attrs::DIM | Attrs::ITALIC | Attrs::INVERSE;
    attrs |= Attrs::INVISIBLE | Attrs::STRIKE | Attrs::BLINK;
    if changed {
        attrs.remove(Attrs::DIM | Attrs::INVERSE);
    }
    let cell = SnapshotCell::new(
        if changed { "Ω" } else { "A" }.to_string(),
        1,
        if changed {
            SnapshotColor::Rgb(11, 12, 13)
        } else {
            SnapshotColor::Indexed(17)
        },
        if changed {
            SnapshotColor::Indexed(23)
        } else {
            SnapshotColor::Rgb(21, 22, 23)
        },
        attrs,
        if changed {
            SnapshotUnderline::Curly
        } else {
            SnapshotUnderline::Double
        },
        SnapshotColor::Indexed(if changed { 24 } else { 18 }),
        Some(if changed {
            "https://example.test/changed".to_string()
        } else {
            "https://example.test/oracle".to_string()
        }),
        Some(if changed { "link-b" } else { "link-a" }.to_string()),
    )
    .expect("rich cell builds");
    let blank = SnapshotCell::new(
        " ".to_string(),
        1,
        SnapshotColor::Default,
        SnapshotColor::Default,
        Attrs::empty(),
        SnapshotUnderline::None,
        SnapshotColor::Default,
        None,
        None,
    )
    .expect("blank cell builds");
    let cursor = SnapshotCursor::new(
        u32::from(changed),
        0,
        true,
        changed,
        if changed {
            SnapshotCursorShape::Underline
        } else {
            SnapshotCursorShape::Bar
        },
        if changed {
            Rgb::new(31, 32, 33)
        } else {
            Rgb::new(41, 42, 43)
        },
        Some(changed),
    );
    let metadata = SnapshotMetadata {
        captured_at_ms: if changed { 5678 } else { 1234 },
        revision: if changed { 8 } else { 7 },
        backend: "oracle-backend".to_string(),
        session: "oracle-session".to_string(),
        source: SnapshotSource::Live,
    };
    TerminalSnapshot::try_new(SnapshotParts {
        cols: 2,
        rows: 1,
        scope: if changed {
            SnapshotScope::ActiveScreen
        } else {
            SnapshotScope::Viewport
        },
        viewport_origin: 0,
        cells: vec![cell, blank],
        palette,
        default_foreground: if changed {
            Rgb::new(51, 52, 53)
        } else {
            Rgb::new(61, 62, 63)
        },
        default_background: if changed {
            Rgb::new(71, 72, 73)
        } else {
            Rgb::new(81, 82, 83)
        },
        default_cursor: if changed {
            Rgb::new(91, 92, 93)
        } else {
            Rgb::new(101, 102, 103)
        },
        cursor,
        title: Some(if changed {
            "oracle changed title".to_string()
        } else {
            "oracle title".to_string()
        }),
        metadata,
    })
    .expect("rich snapshot builds")
}

pub(super) fn pixel_sample(cell: char) -> Sample {
    let id = format!(
        "oracle/visual/pixels-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let profile = resolve_profile(PROFILE_NAME).expect("pinned profile resolves");
    let snapshot = capture_view(2, 1, &ViewSettings::builtin(), |frame| {
        frame.buffer_mut()[(0, 0)].set_char(cell);
    })
    .expect("pixel snapshot is valid");
    Sample::with_pixels(&id, &snapshot, Observation::empty(), profile).expect("pixel sample builds")
}

pub(super) fn run_cli(args: &[&str], remove_ci: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tuiscotti"));
    command
        .args(args)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    if remove_ci {
        command.env_remove("CI");
    }
    command.output().expect("CLI process starts")
}

pub(super) fn accept(sample_path: &Path, baseline_root: &Path, bytes: &[u8]) {
    let candidate_digest = sha256(bytes);
    let output = run_cli(
        &[
            "accept",
            "--candidate",
            sample_path.to_str().expect("candidate path is utf8"),
            "--candidate-sha256",
            &candidate_digest,
            "--expect-baseline",
            "missing",
            "--baselines",
            baseline_root.to_str().expect("baseline path is utf8"),
            "--json",
        ],
        true,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("accept result is JSON");
    assert_eq!(result["status"], "pass");
    assert_eq!(result["candidate_sha256"], candidate_digest);
    assert_eq!(result["baseline_sha256"], candidate_digest);
    let installed = result["output_path"]["value"]
        .as_str()
        .expect("accepted baseline path is present");
    assert_eq!(
        std::fs::read(installed).expect("accepted baseline reads"),
        bytes,
        "accept installs the exact reviewed candidate bytes"
    );
}

fn approval_summary() {
    let root = tempfile::tempdir().expect("approval helper root builds");
    let baseline_root = root.path().join("baselines");
    let baseline_path = root.path().join("baseline.sample");
    let candidate_path = root.path().join("candidate.sample");
    std::fs::create_dir_all(&baseline_root).expect("approval baseline root builds");
    let store = ApprovalStore::new(&baseline_root).expect("approval store opens");
    let baseline = Sample::state(
        "oracle/rich",
        rich_snapshot(false),
        Observation::noted("old observation"),
    )
    .expect("approval baseline builds")
    .to_bytes()
    .expect("approval baseline encodes");
    std::fs::write(&baseline_path, &baseline).expect("approval baseline candidate writes");
    let first = store
        .accept(&baseline_path, &sha256(&baseline), "missing")
        .expect("initial approval succeeds");
    assert_eq!(first.baseline_sha256(), sha256(&baseline));
    assert_eq!(
        std::fs::read(first.baseline_path()).expect("initial installed baseline reads"),
        baseline
    );

    let candidate = Sample::state(
        "oracle/rich",
        rich_snapshot(true),
        Observation::noted("new observation"),
    )
    .expect("changed approval sample builds")
    .to_bytes()
    .expect("changed approval sample encodes");
    std::fs::write(&candidate_path, &candidate).expect("changed approval candidate writes");
    let outcome = store
        .accept(
            &candidate_path,
            &sha256(&candidate),
            first.baseline_sha256(),
        )
        .expect("changed approval succeeds");
    assert_eq!(outcome.previous_baseline_sha256(), first.baseline_sha256());
    assert_eq!(outcome.reason(), Some(MismatchReason::State));
    assert_eq!(outcome.candidate_sha256(), outcome.baseline_sha256());
    assert_eq!(
        std::fs::read(outcome.baseline_path()).expect("changed installed baseline reads"),
        candidate
    );
    assert_eq!(
        outcome.changed_fields(),
        [
            "scope",
            "palette",
            "default_foreground",
            "default_background",
            "default_cursor",
            "cursor.col",
            "cursor.visible",
            "cursor.shape",
            "cursor.color",
            "cursor.blink",
            "title",
        ]
    );
    println!(
        "approval summary reason={} fields={}",
        outcome.reason().expect("reason exists"),
        outcome.changed_fields().join(",")
    );
}

#[test]
fn approval_change_summary_subprocess_helper() {
    if std::env::var_os("TUISCOTTI_ORACLE_APPROVAL_HELPER").is_none_or(|value| value != "1") {
        return;
    }
    approval_summary();
}

pub(super) fn verify_approval_summary_subprocess() {
    let output = Command::new(std::env::current_exe().expect("test executable path exists"))
        .args([
            "--exact",
            "support::approval_change_summary_subprocess_helper",
            "--nocapture",
        ])
        .env_remove("CI")
        .env("TUISCOTTI_ORACLE_APPROVAL_HELPER", "1")
        .output()
        .expect("approval summary subprocess starts");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "approval summary reason=state fields=scope,palette,default_foreground,default_background,default_cursor,cursor.col,cursor.visible,cursor.shape,cursor.color,cursor.blink,title"
    ));
}

pub(super) fn expect_failure(
    outcome: CheckOutcome,
    reason: MismatchReason,
) -> tuiscotti::CheckFailure {
    match outcome {
        CheckOutcome::Failure(failure) => {
            assert_eq!(failure.reason(), reason);
            failure
        }
        CheckOutcome::Pass(_) => panic!("expected a {reason} check failure"),
    }
}
