//! Audited PNG 0.18.1 and fdeflate 0.3.7 sources embedded for bounded sample decoding.
//!
//! Source files are derived from the crates.io registry archives with SHA-256 checksums
//! recorded in `PROVENANCE.md`. They retain their upstream licenses. The PNG decoder
//! transformation allocation sites carry the reviewed 1024-byte closure-size assertion.

#![allow(
    clippy::all,
    clippy::pedantic,
    dead_code,
    missing_docs,
    rust_2018_idioms,
    unused_imports,
    unused_variables,
    reason = "preserve upstream decoder source and its lint baseline in this private embedded module"
)]

#[path = "fdeflate/lib.rs"]
pub(crate) mod fdeflate;
#[path = "png/lib.rs"]
pub(crate) mod png;
