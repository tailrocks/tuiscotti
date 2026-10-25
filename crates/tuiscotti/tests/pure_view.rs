//! D01 pure-view tests: one upstream snapshot model for production-view tests.
//!
//! Covers the D01 subcases from `docs/migration/corpus.json`:
//!
//! - C01: draw closure through `TestBackend`, buffer conversion with explicit
//!   cursor, and no child process or PTY started.
//! - C04: buffer conversion keeps the mandatory graphemes and continuations.
//! - C05: buffer conversion keeps styles, palette, defaults, underline,
//!   reverse video, and cursor state.
//!
//! Constructor tests target the upstream snapshot API directly, never the
//! downstream sample API (owned by D02).

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color as RatatuiColor, Modifier, Style};
use ratatui::widgets::Paragraph;
use tui_test::profile::Rgb;
use tui_test::terminal::cell::Attrs;
use tui_test::{
    SnapshotCell, SnapshotColor, SnapshotCursorShape, SnapshotError, SnapshotMetadata,
    SnapshotParts, SnapshotScope, SnapshotSource, SnapshotUnderline, TerminalSnapshot,
};
use tuiscotti::view::{ViewCursor, ViewError, ViewSettings, capture_view, snapshot_from_buffer};

/// Stand-in for an application production view function.
///
/// `capture_view` takes the draw closure as a caller-supplied parameter; the
/// closure calls the real view, never a test-only replacement.
fn login_view(frame: &mut Frame<'_>) {
    frame.render_widget(Paragraph::new("hello"), frame.area());
    frame.set_cursor_position(Position::new(5, 2));
}

fn visible_cursor(col: u32, row: u32) -> ViewCursor {
    ViewCursor::new(
        col,
        row,
        true,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
    )
}

fn hidden_cursor() -> ViewCursor {
    ViewCursor::new(
        0,
        0,
        false,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
    )
}

fn row_text(snapshot: &TerminalSnapshot, row: u32) -> Result<String, String> {
    let mut text = String::new();
    for col in 0..snapshot.cols() {
        let cell = snapshot
            .cell_at(col, row)
            .ok_or_else(|| format!("test grid lacks ({col}, {row})"))?;
        if !cell.is_continuation() {
            text.push_str(cell.text());
        }
    }
    Ok(text.trim_end().to_string())
}

#[test]
fn c01_draw_closure_renders_production_view() {
    let snapshot = capture_view(20, 5, &ViewSettings::builtin(), |frame| {
        login_view(frame);
    })
    .expect("capture_view over a production draw closure succeeds");
    assert_eq!(snapshot.cols(), 20);
    assert_eq!(snapshot.rows(), 5);
    assert_eq!(snapshot.scope(), SnapshotScope::Viewport);
    assert_eq!(snapshot.viewport_origin(), 0);
    assert_eq!(row_text(&snapshot, 0).expect("row text reads"), "hello");
    assert_eq!(row_text(&snapshot, 2).expect("row text reads"), "");
}

#[test]
fn c01_draw_closure_keeps_ratatui_cursor() {
    let snapshot = capture_view(20, 5, &ViewSettings::builtin(), |frame| {
        login_view(frame);
    })
    .expect("capture_view over a production draw closure succeeds");
    let cursor = snapshot.cursor();
    assert!(cursor.visible(), "draw result shows the cursor");
    assert!(cursor.in_capture());
    assert_eq!((cursor.col(), cursor.row()), (5, 2));
    assert_eq!(snapshot.cursor_local(), Some((5, 2)));
    assert_eq!(cursor.shape(), SnapshotCursorShape::Block);
    assert_eq!(cursor.color(), Rgb::new(192, 192, 192));
    assert_eq!(cursor.blink(), None, "blink intent stays unavailable");
}

#[test]
fn c01_draw_closure_without_cursor_stays_hidden() {
    let settings = ViewSettings::builtin().with_cursor(ViewCursor::new(
        3,
        1,
        false,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
    ));
    let snapshot = capture_view(10, 3, &settings, |frame| {
        frame.render_widget(Paragraph::new("x"), frame.area());
    })
    .expect("capture_view without a cursor request succeeds");
    let cursor = snapshot.cursor();
    assert!(!cursor.visible());
    assert!(cursor.in_capture());
    assert_eq!((cursor.col(), cursor.row()), (3, 1));
    assert_eq!(snapshot.cursor_local(), Some((3, 1)));
}

#[test]
fn c01_snapshot_carries_caller_supplied_view_settings() {
    let snapshot = capture_view(4, 2, &ViewSettings::builtin(), |frame| {
        login_view(frame);
    })
    .expect("capture_view over a production draw closure succeeds");
    assert_eq!(snapshot.default_foreground(), Rgb::new(192, 192, 192));
    assert_eq!(snapshot.default_background(), Rgb::new(0, 0, 0));
    assert_eq!(snapshot.default_cursor(), Rgb::new(192, 192, 192));
    assert_eq!(snapshot.palette()[1], Rgb::new(128, 0, 0));
    assert_eq!(snapshot.palette()[200], Rgb::new(255, 0, 215));
    assert_eq!(snapshot.title(), None);
    assert_eq!(snapshot.metadata().source, SnapshotSource::Offline);
    for cell in snapshot.cells() {
        assert_eq!(cell.link_uri(), None, "hyperlinks stay unavailable");
        assert_eq!(cell.link_id(), None, "hyperlink identity stays unavailable");
    }
}

#[test]
fn c01_buffer_conversion_uses_explicit_cursor() {
    let buffer = Buffer::empty(Rect::new(0, 0, 6, 2));
    let settings = ViewSettings::builtin().with_cursor(visible_cursor(4, 1));
    let snapshot = snapshot_from_buffer(&buffer, &settings)
        .expect("buffer conversion with explicit cursor succeeds");
    assert_eq!(snapshot.cols(), 6);
    assert_eq!(snapshot.rows(), 2);
    assert_eq!(snapshot.cells().len(), 12);
    let cursor = snapshot.cursor();
    assert!(cursor.visible());
    assert!(cursor.in_capture());
    assert_eq!(snapshot.cursor_local(), Some((4, 1)));
}

#[test]
fn c01_out_of_grid_cursor_is_not_clamped() {
    let buffer = Buffer::empty(Rect::new(0, 0, 6, 2));
    let settings = ViewSettings::builtin().with_cursor(visible_cursor(99, 99));
    let snapshot = snapshot_from_buffer(&buffer, &settings)
        .expect("buffer conversion with an outside cursor succeeds");
    let cursor = snapshot.cursor();
    assert!(!cursor.in_capture());
    assert_eq!((cursor.col(), cursor.row()), (99, 99));
    assert_eq!(snapshot.cursor_local(), None);
}

#[test]
fn c01_empty_grid_fails_without_panic() {
    let buffer = Buffer::empty(Rect::new(0, 0, 0, 0));
    let result = snapshot_from_buffer(&buffer, &ViewSettings::builtin());
    assert!(result.is_err(), "empty buffer area fails conversion");
    let result = capture_view(0, 5, &ViewSettings::builtin(), |_| {});
    assert!(result.is_err(), "empty capture width fails conversion");
}

/// Pure-view execution starts no child process.
///
/// The conversion path takes only an in-memory buffer and caller-supplied
/// settings, so there is nothing to spawn. This test pins that property at
/// runtime: file-descriptor and child-process censuses around repeated
/// captures must not move. The compile-time guarantee is structural: the
/// production `view` module imports only Ratatui and upstream snapshot
/// types, never process, PTY, or session APIs.
#[cfg(unix)]
#[test]
fn c01_pure_view_starts_no_child_process() {
    fn open_fd_count() -> usize {
        let path = if std::path::Path::new("/proc/self/fd").exists() {
            "/proc/self/fd"
        } else {
            "/dev/fd"
        };
        match std::fs::read_dir(path) {
            Ok(entries) => entries.count(),
            Err(_) => 0,
        }
    }

    #[cfg(target_os = "linux")]
    fn child_pids() -> Vec<u32> {
        let self_pid = std::process::id();
        let mut children = Vec::new();
        let Ok(proc) = std::fs::read_dir("/proc") else {
            return children;
        };
        for entry in proc.flatten() {
            let name = entry.file_name();
            let Some(pid) = name.to_str().and_then(|text| text.parse::<u32>().ok()) else {
                continue;
            };
            if pid == self_pid {
                continue;
            }
            let status_path = format!("/proc/{pid}/status");
            let Ok(status) = std::fs::read_to_string(status_path) else {
                continue;
            };
            let is_child = status.lines().any(|line| {
                line.strip_prefix("PPid:")
                    .and_then(|value| value.trim().parse::<u32>().ok())
                    .is_some_and(|ppid| ppid == self_pid)
            });
            if is_child {
                children.push(pid);
            }
        }
        children.sort_unstable();
        children
    }

    let fds_before = open_fd_count();
    #[cfg(target_os = "linux")]
    let children_before = child_pids();
    for _ in 0..50 {
        let snapshot = capture_view(20, 5, &ViewSettings::builtin(), |frame| {
            login_view(frame);
        })
        .expect("repeated pure-view capture succeeds");
        assert_eq!(row_text(&snapshot, 0).expect("row text reads"), "hello");
        let buffer = Buffer::empty(Rect::new(0, 0, 20, 5));
        snapshot_from_buffer(&buffer, &ViewSettings::builtin())
            .expect("repeated buffer conversion succeeds");
    }
    assert_eq!(
        open_fd_count(),
        fds_before,
        "pure-view capture opens no file descriptor"
    );
    #[cfg(target_os = "linux")]
    assert_eq!(
        child_pids(),
        children_before,
        "pure-view capture starts no child process"
    );
}

/// Fill one buffer row with the mandatory corpus items for C04.
fn unicode_corpus_buffer() -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
    buffer.set_stringn(0, 0, "hi", 10, Style::default());
    buffer.set_stringn(2, 0, "─│┌", 8, Style::default());
    buffer.set_stringn(5, 0, "e\u{301}", 5, Style::default());
    buffer.set_stringn(6, 0, "a\u{308}", 4, Style::default());
    buffer.set_stringn(1, 1, "東", 9, Style::default());
    buffer.set_stringn(4, 1, "b", 6, Style::default().add_modifier(Modifier::BOLD));
    buffer.set_stringn(
        5,
        1,
        "i",
        5,
        Style::default().add_modifier(Modifier::ITALIC),
    );
    buffer.set_stringn(
        6,
        1,
        "B",
        4,
        Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
    );
    buffer.set_stringn(8, 2, "東", 2, Style::default());
    buffer
}

#[test]
fn c04_buffer_conversion_keeps_graphemes() {
    let buffer = unicode_corpus_buffer();
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("unicode corpus conversion succeeds");
    assert_eq!(snapshot.cols(), 10);
    assert_eq!(snapshot.rows(), 3);
    let ascii = snapshot.cell_at(0, 0).expect("ascii cell exists");
    assert_eq!(ascii.text(), "h");
    assert_eq!(ascii.width(), 1);
    let box_cell = snapshot.cell_at(2, 0).expect("box-drawing cell exists");
    assert_eq!(box_cell.text(), "─");
    let acute = snapshot.cell_at(5, 0).expect("combining cell exists");
    assert_eq!(acute.text(), "e\u{301}");
    assert_eq!(acute.width(), 1);
    let diaeresis = snapshot.cell_at(6, 0).expect("diaeresis cell exists");
    assert_eq!(diaeresis.text(), "a\u{308}");
    let wide = snapshot.cell_at(1, 1).expect("wide leading cell exists");
    assert_eq!(wide.text(), "東");
    assert_eq!(wide.width(), 2);
    let continuation = snapshot.cell_at(2, 1).expect("continuation cell exists");
    assert!(continuation.is_continuation());
    assert_eq!(continuation.text(), "");
    assert_eq!(continuation.width(), 0);
    let bold = snapshot.cell_at(4, 1).expect("bold cell exists");
    assert!(bold.has(Attrs::BOLD));
    assert!(!bold.has(Attrs::ITALIC));
    let italic = snapshot.cell_at(5, 1).expect("italic cell exists");
    assert!(italic.has(Attrs::ITALIC));
    assert!(!italic.has(Attrs::BOLD));
    let bold_italic = snapshot.cell_at(6, 1).expect("bold-italic cell exists");
    assert!(bold_italic.has(Attrs::BOLD));
    assert!(bold_italic.has(Attrs::ITALIC));
}

#[test]
fn c04_wide_character_at_last_column_boundary() {
    let buffer = unicode_corpus_buffer();
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("boundary corpus conversion succeeds");
    let edge = snapshot.cell_at(8, 2).expect("boundary wide cell exists");
    assert_eq!(edge.text(), "東");
    assert_eq!(edge.width(), 2);
    let covered = snapshot.cell_at(9, 2).expect("covered cell exists");
    assert!(covered.is_continuation());
}

#[test]
fn c04_overflowing_wide_character_stays_blank() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 1));
    buffer.set_stringn(9, 0, "東", 1, Style::default());
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("overflow corpus conversion succeeds");
    let cell = snapshot.cell_at(9, 0).expect("last column cell exists");
    assert_eq!(cell.text(), " ");
    assert!(!cell.is_continuation());
}

/// A narrow grapheme hand-written into a wide cell's covered column is an
/// invalid leading and continuation combination. The cells are individually
/// valid, so the upstream constructor would accept them; conversion rejects
/// the buffer itself with a typed error naming the covered coordinates.
#[test]
fn c04_grapheme_in_covered_column_fails_closed() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    buffer.set_stringn(0, 0, "東", 4, Style::default());
    buffer[(1, 0)].set_symbol("x");
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let result = snapshot_from_buffer(&buffer, &settings);
    assert!(
        matches!(result, Err(ViewError::CoveredCellOccupied { x: 1, y: 0 })),
        "narrow grapheme in a covered column fails closed"
    );
}

/// A wide leading cell hand-written into the last column overhangs the row:
/// its continuation would fall outside the grid. Conversion rejects the
/// buffer itself with a typed error naming the leading coordinates.
#[test]
fn c04_wide_leading_cell_overhanging_row_end_fails_closed() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer[(2, 0)].set_symbol("東");
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let result = snapshot_from_buffer(&buffer, &settings);
    assert!(
        matches!(
            result,
            Err(ViewError::LeadingCellOverhang {
                x: 2,
                y: 0,
                width: 2
            })
        ),
        "wide leading cell overhanging the row end fails closed"
    );
}

#[test]
fn c04_cursor_over_combined_and_wide_cells() {
    let buffer = unicode_corpus_buffer();
    let over_combined = ViewSettings::builtin().with_cursor(visible_cursor(5, 0));
    let snapshot = snapshot_from_buffer(&buffer, &over_combined)
        .expect("cursor over a combined cell converts");
    assert_eq!(snapshot.cursor_local(), Some((5, 0)));
    assert_eq!(
        snapshot.cell_at(5, 0).expect("combined cell exists").text(),
        "e\u{301}"
    );
    let over_wide = ViewSettings::builtin().with_cursor(visible_cursor(1, 1));
    let snapshot =
        snapshot_from_buffer(&buffer, &over_wide).expect("cursor over a wide cell converts");
    assert_eq!(snapshot.cursor_local(), Some((1, 1)));
    let over_continuation = ViewSettings::builtin().with_cursor(visible_cursor(2, 1));
    let snapshot = snapshot_from_buffer(&buffer, &over_continuation)
        .expect("cursor over a continuation cell converts");
    assert_eq!(snapshot.cursor_local(), Some((2, 1)));
    assert!(
        snapshot
            .cell_at(2, 1)
            .expect("continuation cell exists")
            .is_continuation()
    );
}

#[test]
fn c05_buffer_conversion_keeps_color_references() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    buffer.set_stringn(
        0,
        0,
        "a",
        4,
        Style::default()
            .fg(RatatuiColor::Indexed(1))
            .bg(RatatuiColor::Indexed(200)),
    );
    buffer.set_stringn(
        1,
        0,
        "b",
        3,
        Style::default().fg(RatatuiColor::Rgb(10, 20, 30)),
    );
    buffer.set_stringn(2, 0, "c", 2, Style::default());
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("style corpus conversion succeeds");
    let indexed = snapshot.cell_at(0, 0).expect("indexed cell exists");
    assert_eq!(indexed.fg(), SnapshotColor::Indexed(1));
    assert_eq!(indexed.bg(), SnapshotColor::Indexed(200));
    let rgb = snapshot.cell_at(1, 0).expect("rgb cell exists");
    assert_eq!(rgb.fg(), SnapshotColor::Rgb(10, 20, 30));
    assert_eq!(rgb.bg(), SnapshotColor::Default);
    let reset = snapshot.cell_at(2, 0).expect("reset cell exists");
    assert_eq!(reset.fg(), SnapshotColor::Default);
    assert_eq!(reset.bg(), SnapshotColor::Default);
    assert_ne!(
        reset.fg(),
        SnapshotColor::Indexed(7),
        "default stays distinct from an explicit palette reference"
    );
}

#[test]
fn c05_named_ansi_colors_map_to_fixed_slots() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    buffer.set_stringn(
        0,
        0,
        "a",
        4,
        Style::default()
            .fg(RatatuiColor::Red)
            .bg(RatatuiColor::Gray),
    );
    buffer.set_stringn(
        1,
        0,
        "b",
        3,
        Style::default()
            .fg(RatatuiColor::LightRed)
            .bg(RatatuiColor::White),
    );
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("named color conversion succeeds");
    let first = snapshot.cell_at(0, 0).expect("first cell exists");
    assert_eq!(first.fg(), SnapshotColor::Indexed(1));
    assert_eq!(first.bg(), SnapshotColor::Indexed(7));
    let second = snapshot.cell_at(1, 0).expect("second cell exists");
    assert_eq!(second.fg(), SnapshotColor::Indexed(9));
    assert_eq!(second.bg(), SnapshotColor::Indexed(15));
}

#[test]
fn c05_palette_and_default_overrides_reach_snapshot() {
    let buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    let mut palette = *ViewSettings::builtin().palette();
    palette[7] = Rgb::new(9, 9, 9);
    let settings = ViewSettings::builtin()
        .with_palette(palette)
        .with_defaults(Rgb::new(1, 2, 3), Rgb::new(4, 5, 6))
        .with_cursor(hidden_cursor());
    let snapshot = snapshot_from_buffer(&buffer, &settings).expect("override conversion succeeds");
    assert_eq!(snapshot.palette()[7], Rgb::new(9, 9, 9));
    assert_eq!(snapshot.palette()[1], Rgb::new(128, 0, 0));
    assert_eq!(snapshot.default_foreground(), Rgb::new(1, 2, 3));
    assert_eq!(snapshot.default_background(), Rgb::new(4, 5, 6));
}

#[test]
fn c05_underline_reverse_and_blink_convert() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
    buffer.set_stringn(
        0,
        0,
        "u",
        5,
        Style::default().add_modifier(Modifier::UNDERLINED),
    );
    buffer.set_stringn(
        1,
        0,
        "r",
        4,
        Style::default().add_modifier(Modifier::REVERSED),
    );
    buffer.set_stringn(
        2,
        0,
        "h",
        3,
        Style::default().add_modifier(Modifier::HIDDEN),
    );
    buffer.set_stringn(
        3,
        0,
        "s",
        2,
        Style::default().add_modifier(Modifier::CROSSED_OUT),
    );
    buffer.set_stringn(
        4,
        0,
        "b",
        1,
        Style::default().add_modifier(Modifier::SLOW_BLINK | Modifier::DIM),
    );
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot = snapshot_from_buffer(&buffer, &settings).expect("modifier conversion succeeds");
    let underlined = snapshot.cell_at(0, 0).expect("underlined cell exists");
    assert_eq!(underlined.underline(), SnapshotUnderline::Single);
    assert_eq!(underlined.underline_color(), SnapshotColor::Default);
    let reversed = snapshot.cell_at(1, 0).expect("reversed cell exists");
    assert!(reversed.has(Attrs::INVERSE));
    assert_eq!(reversed.underline(), SnapshotUnderline::None);
    let hidden = snapshot.cell_at(2, 0).expect("hidden cell exists");
    assert!(hidden.has(Attrs::INVISIBLE));
    let struck = snapshot.cell_at(3, 0).expect("struck cell exists");
    assert!(struck.has(Attrs::STRIKE));
    let blinking = snapshot.cell_at(4, 0).expect("blinking cell exists");
    assert!(blinking.has(Attrs::BLINK));
    assert!(blinking.has(Attrs::DIM));
}

#[test]
fn c05_explicit_underline_color_converts() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer.set_stringn(
        0,
        0,
        "a",
        3,
        Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(RatatuiColor::Rgb(255, 0, 0)),
    );
    buffer.set_stringn(
        1,
        0,
        "b",
        2,
        Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(RatatuiColor::Indexed(5)),
    );
    buffer.set_stringn(
        2,
        0,
        "c",
        1,
        Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(RatatuiColor::Reset),
    );
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("underline color conversion succeeds");
    let rgb = snapshot.cell_at(0, 0).expect("rgb underline cell exists");
    assert_eq!(rgb.underline(), SnapshotUnderline::Single);
    assert_eq!(rgb.underline_color(), SnapshotColor::Rgb(255, 0, 0));
    let indexed = snapshot
        .cell_at(1, 0)
        .expect("indexed underline cell exists");
    assert_eq!(indexed.underline_color(), SnapshotColor::Indexed(5));
    let reset = snapshot.cell_at(2, 0).expect("reset underline cell exists");
    assert_eq!(reset.underline_color(), SnapshotColor::Default);
}

#[test]
fn c05_cursor_shapes_and_visibility_convert() {
    let buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    for shape in [
        SnapshotCursorShape::Block,
        SnapshotCursorShape::Underline,
        SnapshotCursorShape::Bar,
    ] {
        let settings = ViewSettings::builtin().with_cursor(ViewCursor::new(
            1,
            0,
            true,
            shape,
            Rgb::new(10, 20, 30),
        ));
        let snapshot =
            snapshot_from_buffer(&buffer, &settings).expect("cursor shape conversion succeeds");
        let cursor = snapshot.cursor();
        assert!(cursor.visible());
        assert_eq!(cursor.shape(), shape);
        assert_eq!(cursor.color(), Rgb::new(10, 20, 30));
        assert_eq!(snapshot.default_cursor(), Rgb::new(10, 20, 30));
    }
    let hidden = ViewSettings::builtin().with_cursor(ViewCursor::new(
        1,
        0,
        false,
        SnapshotCursorShape::Bar,
        Rgb::new(10, 20, 30),
    ));
    let snapshot =
        snapshot_from_buffer(&buffer, &hidden).expect("hidden cursor conversion succeeds");
    assert!(!snapshot.cursor().visible());
    assert_eq!(snapshot.cursor().shape(), SnapshotCursorShape::Bar);
}

#[test]
fn c05_buffer_area_origin_normalizes_to_snapshot_origin() {
    let buffer = Buffer::empty(Rect::new(4, 2, 3, 2));
    let settings = ViewSettings::builtin().with_cursor(visible_cursor(1, 1));
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("offset buffer conversion succeeds");
    assert_eq!(snapshot.cols(), 3);
    assert_eq!(snapshot.rows(), 2);
    assert_eq!(snapshot.cells().len(), 6);
    assert_eq!(snapshot.viewport_origin(), 0);
    assert_eq!(snapshot.cursor_local(), Some((1, 1)));
}

/// The built-in palette copies the migration API record byte for byte.
///
/// The canonical digest pins all 256 entries plus the foreground, background,
/// and cursor defaults, so any transcription drift fails here.
#[test]
fn view_palette_matches_recorded_digest() {
    use sha2::{Digest, Sha256};
    let settings = ViewSettings::builtin();
    let palette: Vec<String> = settings
        .palette()
        .iter()
        .map(|color| color.to_hex())
        .collect();
    let document = serde_json::json!({
        "background": settings.default_background().to_hex(),
        "cursor": settings.cursor().color().to_hex(),
        "foreground": settings.default_foreground().to_hex(),
        "palette": palette,
    });
    let encoded = serde_json::to_string(&document).expect("palette document encodes");
    let digest = Sha256::digest(encoded.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(hex, "{byte:02x}").expect("hex formatting into a string succeeds");
    }
    assert_eq!(
        hex,
        "92757b147e00dec394b10b7be2fa42f782f0d30325a5922570080f91d60f641e"
    );
}

fn snapshot_parts(cells: Vec<SnapshotCell>, cols: u32, rows: u32) -> SnapshotParts {
    SnapshotParts {
        cols,
        rows,
        scope: SnapshotScope::Viewport,
        viewport_origin: 0,
        cells,
        palette: *ViewSettings::builtin().palette(),
        default_foreground: Rgb::new(192, 192, 192),
        default_background: Rgb::new(0, 0, 0),
        default_cursor: Rgb::new(192, 192, 192),
        cursor: tui_test::SnapshotCursor::new(
            0,
            0,
            true,
            true,
            SnapshotCursorShape::Block,
            Rgb::new(192, 192, 192),
            None,
        ),
        title: None,
        metadata: SnapshotMetadata::default(),
    }
}

fn plain_cell(text: &str, width: u16) -> Result<SnapshotCell, SnapshotError> {
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
}

#[test]
fn upstream_constructor_rejects_empty_dimensions() {
    let parts = snapshot_parts(Vec::new(), 0, 0);
    assert!(matches!(
        TerminalSnapshot::try_new(parts),
        Err(SnapshotError::EmptyDimensions)
    ));
}

#[test]
fn upstream_constructor_rejects_cell_count_mismatch() {
    let parts = snapshot_parts(
        vec![plain_cell("a", 1).expect("plain test cell builds")],
        2,
        2,
    );
    assert!(matches!(
        TerminalSnapshot::try_new(parts),
        Err(SnapshotError::CellCountMismatch { .. })
    ));
}

#[test]
fn upstream_constructor_rejects_leading_continuation() {
    let cells = vec![
        SnapshotCell::new(
            String::new(),
            0,
            SnapshotColor::Default,
            SnapshotColor::Default,
            Attrs::empty(),
            SnapshotUnderline::None,
            SnapshotColor::Default,
            None,
            None,
        )
        .expect("empty test cell builds"),
        plain_cell("a", 1).expect("plain test cell builds"),
    ];
    let parts = snapshot_parts(cells, 2, 1);
    assert!(matches!(
        TerminalSnapshot::try_new(parts),
        Err(SnapshotError::BadContinuation { col: 0, row: 0 })
    ));
}

#[test]
fn upstream_constructor_rejects_sized_continuation() {
    let cells = vec![
        plain_cell("a", 1).expect("plain test cell builds"),
        SnapshotCell::new(
            String::new(),
            1,
            SnapshotColor::Default,
            SnapshotColor::Default,
            Attrs::empty(),
            SnapshotUnderline::None,
            SnapshotColor::Default,
            None,
            None,
        )
        .expect("sized empty test cell builds"),
    ];
    let parts = snapshot_parts(cells, 2, 1);
    assert!(matches!(
        TerminalSnapshot::try_new(parts),
        Err(SnapshotError::BadContinuation { col: 1, row: 0 })
    ));
}

#[test]
fn upstream_constructor_rejects_out_of_bounds_cursor() {
    let cells = vec![
        plain_cell("a", 1).expect("plain test cell builds"),
        plain_cell("b", 1).expect("plain test cell builds"),
    ];
    let mut parts = snapshot_parts(cells, 2, 1);
    parts.cursor = tui_test::SnapshotCursor::new(
        7,
        0,
        true,
        true,
        SnapshotCursorShape::Block,
        Rgb::new(192, 192, 192),
        None,
    );
    assert!(matches!(
        TerminalSnapshot::try_new(parts),
        Err(SnapshotError::CursorOutOfBounds { .. })
    ));
}

#[test]
fn upstream_serialization_round_trip_keeps_state() {
    let buffer = unicode_corpus_buffer();
    let settings = ViewSettings::builtin().with_cursor(visible_cursor(5, 0));
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("unicode corpus conversion succeeds");
    let encoded = serde_json::to_string(&snapshot).expect("snapshot encodes");
    let decoded: TerminalSnapshot = serde_json::from_str(&encoded).expect("snapshot decodes");
    assert_eq!(decoded, snapshot);
    assert_eq!(decoded.cell_at(1, 1).expect("wide cell").text(), "東");
}

#[test]
fn upstream_deserialization_rejects_unknown_schema() {
    let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let snapshot =
        snapshot_from_buffer(&buffer, &settings).expect("single cell conversion succeeds");
    let mut value = serde_json::to_value(&snapshot).expect("snapshot converts to JSON");
    value["schema"] = serde_json::json!(9999);
    let encoded = serde_json::to_string(&value).expect("tampered JSON encodes");
    let result: Result<TerminalSnapshot, _> = serde_json::from_str(&encoded);
    assert!(result.is_err(), "unknown schema fails closed");
}

#[test]
fn upstream_snapshot_ignores_observation_metadata() {
    let buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let first = snapshot_from_buffer(&buffer, &settings).expect("first conversion succeeds");
    let second = snapshot_from_buffer(&buffer, &settings).expect("second conversion succeeds");
    assert_eq!(first, second);
    let mut parts = snapshot_parts(first.cells().to_vec(), first.cols(), first.rows());
    parts.cursor = first.cursor().clone();
    parts.metadata = SnapshotMetadata {
        captured_at_ms: 123,
        revision: 4,
        backend: "probe".to_string(),
        session: "probe".to_string(),
        source: SnapshotSource::Live,
    };
    let relabeled = TerminalSnapshot::try_new(parts).expect("relabeled snapshot builds");
    assert_eq!(
        relabeled, first,
        "metadata changes never change canonical state"
    );
}

/// R2: a zero-width cell outside any wide-cell coverage is not a
/// continuation. Conversion rejects it with the cell coordinates.
#[test]
fn r2_uncovered_zero_width_cell_fails_closed() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer[(1, 0)].set_symbol("");
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let result = snapshot_from_buffer(&buffer, &settings);
    assert!(
        matches!(
            result,
            Err(ViewError::UncoveredZeroWidthCell { x: 1, y: 0 })
        ),
        "uncovered zero-width cell fails closed, got {result:?}"
    );
}

/// R2: a covered zero-width cell stays a valid continuation, and a wide
/// grapheme clipped at the row edge stays a blank narrow leading cell.
#[test]
fn r2_covered_and_clipped_edges_still_convert() {
    let settings = ViewSettings::builtin().with_cursor(hidden_cursor());
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer.set_stringn(0, 0, "東", 3, Style::default());
    let snapshot = snapshot_from_buffer(&buffer, &settings).expect("covered edge converts");
    assert_eq!(snapshot.cell_at(0, 0).expect("leading exists").text(), "東");
    assert!(snapshot
        .cell_at(1, 0)
        .expect("continuation exists")
        .is_continuation());
    let mut clipped = Buffer::empty(Rect::new(0, 0, 1, 1));
    clipped.set_stringn(0, 0, "東", 1, Style::default());
    let snapshot = snapshot_from_buffer(&clipped, &settings).expect("clipped edge converts");
    let cell = snapshot.cell_at(0, 0).expect("clipped cell exists");
    assert!(!cell.is_continuation(), "clipped edge is a narrow leading cell");
}
