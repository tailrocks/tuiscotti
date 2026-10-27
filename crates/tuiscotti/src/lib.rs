#![doc = include_str!("../docs/lib.md")]
#[doc = "Explicit digest-arbitrated review for new or changed samples."]
pub mod approve;
#[doc = "Frozen-baseline checks of rendered samples."]
pub mod check;
#[doc = "Facade error unifying every fallible operation."]
pub mod error;
#[doc = "Offline rendering of saved samples."]
pub mod render;
#[doc = "Static HTML comparison reports and evidence index."]
pub mod report;
#[doc = "Saved sample construction and validation."]
pub mod sample;
#[doc = "Pure-view rendering of application views in the test process."]
pub mod view;

#[doc = "Render profile selecting the pinned upstream raster policy and fonts."]
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
