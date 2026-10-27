//! tuiscotti: offline visual-regression workflow for Ratatui views.
//!
//! A pure-view test renders an application view in the test process through
//! [`view`] (no shell, child process, PTY, daemon, or terminal session) and
//! checks the resulting [`sample`] against frozen baselines with [`check`].
//! New or changed samples pass explicit digest-arbitrated review in
//! [`approve`]; [`report`] renders static HTML evidence. [`error`] unifies
//! the fallible operations of these five modules.
//!
//! Live terminal tests use `tui_test::Session` directly from application
//! test code. tuiscotti ships no session, locator engine, input API,
//! operation enum, renderer, or approval adapter, and it does not re-export
//! the upstream API: small conversion functions at the visual boundary
//! ([`view::capture_view`], [`view::snapshot_from_buffer`]) are the only
//! bridge between a Ratatui view and the upstream snapshot model.

pub mod approve;
pub mod check;
pub mod error;
pub mod render;
pub mod report;
pub mod sample;
pub mod view;

/// Render profile selecting the pinned upstream raster policy and fonts.
///
/// The registry file keeps its packet name `profile.rs`; the module is
/// `render_profile` so the name records that the profile describes the
/// upstream pinned renderer, not a downstream renderer (D07 removed the
/// local glyph rasterizer and general image renderer).
#[path = "profile.rs"]
pub mod render_profile;

pub use approve::{AcceptOutcome, ApprovalStore, ApproveError};
pub use check::{
    CHECK_SCHEMA, CheckError, CheckFailure, CheckOutcome, CheckSuccess, FrozenStore,
    MISSING_BASELINE, MismatchReason, validate_test_id,
};
pub use error::{Error, Result};
pub use render::{RenderError, RenderOutcome, render_sample};
pub use report::{
    IndexError, IndexOutcome, PixelSummary, ReportInput, ReportedCell, escape_html, render_report,
    write_report_index,
};
pub use sample::{
    CONTRACT_VERSION, ContractFont, CursorPhase, Gate, ImageFormat, MAX_CELL_TEXT_BYTES,
    MAX_IMAGE_DIM, MAX_RGBA_BYTES, MAX_SAMPLE_CELLS, MAX_SAMPLE_FILE_BYTES, MAX_TEST_ID_BYTES,
    MAX_TEST_ID_SEGMENT_BYTES, RenderContract, SAMPLE_SCHEMA, Sample, SampleError, SampleImage,
    SampleSchema,
};
pub use view::{
    ViewCursor, ViewError, ViewResult, ViewSettings, capture_view, snapshot_from_buffer,
};
