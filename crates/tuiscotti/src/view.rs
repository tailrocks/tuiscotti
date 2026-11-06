#![doc = include_str!("../docs/view.md")]

mod capture;
mod convert;
mod palette;
mod types;

pub use capture::{capture_view, snapshot_from_buffer};
pub use types::{ViewCursor, ViewError, ViewResult, ViewSettings};
