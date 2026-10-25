//! tuiscotti: Rust TUI visual-regression toolkit (facade).
//!
//! Two capture paths share one canonical screen:
//! - **Pure view tests** ([`ratatui::render`]): production Ratatui view +
//!   viewport → [`Screen`]. No PTY, no subprocess.
//! - **Interactive tests** (`Tui`, feature `pty`): the real executable in a
//!   real PTY — `Session` drives input, locators, waits, [`Screen`] capture.
//! - **Piped tests** ([`Command`]): piped stdio runs → truthful
//!   [`ProcessOutput`] (exit/signal/timeout/output-limit stay distinct).
//!
//! All three assert through [`assert_snapshot!`] (canonical state) and
//! [`assert_screenshot!`] (canonical + PNG as one sample) with native Insta
//! review, caller-fixed metadata, and evidence before failure.
//!
//! Ordinary tests use only the top-level names. Advanced primitives live in
//! the purposeful modules below (`tui`, `command`, `locate`, `proto`, ...),
//! which also preserve every pre-G6 path used out of crate.

pub mod approve;
pub mod check;
pub mod error;
// D01 migration path: pure-view capture into the single upstream snapshot
// model. This module owns the new reduced surface; the surrounding
// re-exports below stay until D07 removes the replaced engine.
pub mod view;
// D02 sample format (GOAL 13.1/13.3) and render profile (GOAL 13.2)
// over that snapshot model. The registry file keeps its packet name
// `profile.rs`; the module is `render_profile` because `profile` still
// names the replaced renderer until D07 removes it.
#[path = "profile.rs"]
pub mod render_profile;
pub mod report;
pub mod sample;

pub use approve::{ACCEPT_SCHEMA, AcceptOutcome, ApprovalStore, ApproveError, EVIDENCE_SCHEMA};
pub use check::{
    CHECK_SCHEMA, CheckError, CheckFailure, CheckOutcome, CheckSuccess, FrozenStore,
    MISSING_BASELINE, MismatchReason, validate_test_id,
};
pub use error::{Error, Result};
pub use report::{PixelSummary, ReportInput, ReportedCell, escape_html, render_report};
// `sample::Observation` and `render_profile::Profile` stay behind their
// module paths: those top-level names still belong to the replaced engine.
pub use sample::{
    CONTRACT_VERSION, ContractFont, CursorPhase, Gate, ImageFormat, MAX_CELL_TEXT_BYTES,
    MAX_IMAGE_DIM, MAX_RGBA_BYTES, MAX_SAMPLE_CELLS, MAX_SAMPLE_FILE_BYTES, MAX_TEST_ID_BYTES,
    MAX_TEST_ID_SEGMENT_BYTES, RenderContract, SAMPLE_SCHEMA, Sample, SampleError, SampleImage,
    SampleSchema,
};
pub use view::{
    ViewCursor, ViewError, ViewResult, ViewSettings, capture_view, snapshot_from_buffer,
};

// Daily facade: launch, capture, query, assert.
pub use tuiscotti_core::locate::Locator;
pub use tuiscotti_core::screen::{CaptureProvenance, CaptureReason, Maybe, Observation, Region};
pub use tuiscotti_core::screen::{RegionPolicy, Screen, ScreenError, TermState};
pub use tuiscotti_insta::assert::{FrozenError, Policy};
pub use tuiscotti_insta::{assert_screenshot, assert_snapshot};
pub use tuiscotti_render::profile::Profile;
pub use tuiscotti_render::render::Renderer;
#[cfg(feature = "pty")]
pub use tuiscotti_runtime::bound_locator::{ActionError, BoundLocator};
pub use tuiscotti_runtime::command::{
    Command, ProcessOutput, SpawnError, SpawnErrorKind, Termination,
};
#[cfg(feature = "pty")]
pub use tuiscotti_runtime::keys::KeyChord;
#[cfg(feature = "pty")]
pub use tuiscotti_runtime::tui::{
    CancelToken, ExitStatus, ExitWait, Key, KeyEventKind, KeyMods, MouseButton, MouseMods, Session,
    SessionMeta, Signal, Tui, TuiError, WaitError,
};

// Purposeful advanced modules (also the pre-G6 compatibility surface).
pub use tuiscotti_core::{frame, locate, names, ratatui, screen, semant};
pub use tuiscotti_insta::{assert, insta_proto};
pub use tuiscotti_render::{diff, export, formats, profile, render};
#[cfg(feature = "pty")]
pub use tuiscotti_runtime::{bound_locator, keys, tui, tui_shell, waits};
pub use tuiscotti_runtime::{
    command, grouped, import_compat, mcp, observe, proto, runner, snapshot,
};

// Pre-G6 top-level compatibility re-exports (kept: used out of crate).
pub use tuiscotti_core::frame::{
    Cell, Color, Cursor, CursorStyle, Frame, FrameError, Mods, Provenance, Rgb, UnderlineStyle,
};
pub use tuiscotti_render::profile::{
    FallbackFace, FontFaces, VENDORED_CJK_FONT, VENDORED_CJK_FONT_SHA256, VENDORED_FACES,
    VENDORED_FALLBACK_FACES, VENDORED_FONT, VENDORED_FONT_BOLD, VENDORED_FONT_BOLD_ITALIC,
    VENDORED_FONT_ITALIC, VENDORED_SYMBOLS_FONT, VENDORED_SYMBOLS_FONT_SHA256,
    VENDORED_SYMBOLS2_FONT, VENDORED_SYMBOLS2_FONT_SHA256,
};
pub use tuiscotti_runtime::grouped::{
    ArtifactPaths, ArtifactSet, GroupedOutcome, GroupedStore, InvalidName,
};
