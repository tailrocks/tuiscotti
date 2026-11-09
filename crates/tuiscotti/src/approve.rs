#![doc = include_str!("../docs/approve.md")]

mod changes;
mod digest;
mod errors;
mod outcome;
mod paths;
mod render;
mod transaction;

pub use errors::{ApproveError, sample_code};
pub use outcome::AcceptOutcome;
pub use transaction::ApprovalStore;
