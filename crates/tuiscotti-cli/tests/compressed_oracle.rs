//! Pre-format behavioral oracle for sample state digests and frozen checks.
//!
//! These expectations describe semantic outputs, not serialized sample bytes,
//! so the cases remain valid when sample persistence changes format.
//!
//! `SnapshotParts` covers snapshot metadata without changing canonical state.
//! The old-format semantic results for cases that need invalid private
//! comparisons are recorded in `fixtures/pre_format_oracle.tsv`; transient
//! legacy fixture mutations captured them and the permanent test does not
//! regenerate them. The committed test has no sample JSON reader. A future binary
//! test can construct equivalent samples from schema fixtures. A valid
//! private contract projection is retained as reference data; byte-for-byte
//! checks need an in-crate seam after public serde is removed. A valid
//! shared-ID gate mismatch and a dimensions-only mismatch are unreachable:
//! sample validation binds the gate to the ID suffix and each image to its
//! snapshot grid before comparison.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "fixed oracle fixtures make setup failures test failures"
)]

#[path = "compressed_oracle/cases.rs"]
mod cases;
#[path = "compressed_oracle/contracts.rs"]
mod contracts;
#[path = "compressed_oracle/support.rs"]
mod support;
