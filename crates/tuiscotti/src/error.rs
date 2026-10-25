//! Facade error: one typed `Error`/`Result` over every fallible operation.
//!
//! Each variant retains its typed source; nothing is erased to a string.
//! `?` converts automatically via the [`From`] impls.
//!
//! D07: the removed live engine, local renderer, and adapter crates have no
//! variants here. Live failures surface through the upstream `tui-test`
//! error types in application test code, never through this facade.

use std::fmt::{Display, Formatter};

/// Fallible-operation result over the facade [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure a facade operation can report, with sources retained.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Pure-view capture or conversion failed.
    View(crate::view::ViewError),
    /// Sample construction, validation, or rendering failed (D02).
    Sample(crate::sample::SampleError),
    /// Frozen comparison or candidate evidence failed (D03).
    Check(crate::check::CheckError),
    /// Static HTML report rendering failed (D03).
    Report(crate::report::ReportError),
    /// Render profile lookup or build failed (D02).
    Profile(crate::render_profile::ProfileError),
    /// Approval transaction failed (D04).
    Approve(crate::approve::ApproveError),
    /// Filesystem failure.
    Io(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::View(e) => write!(f, "{e}"),
            Self::Sample(e) => write!(f, "{e}"),
            Self::Check(e) => write!(f, "{e}"),
            Self::Report(e) => write!(f, "{e}"),
            Self::Profile(e) => write!(f, "{e}"),
            Self::Approve(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::View(e) => Some(e),
            Self::Sample(e) => Some(e),
            Self::Check(e) => Some(e),
            Self::Report(e) => Some(e),
            Self::Profile(e) => Some(e),
            Self::Approve(e) => Some(e),
            Self::Io(e) => Some(e),
        }
    }
}

macro_rules! convert {
    ($variant:ident, $type:ty) => {
        impl From<$type> for Error {
            fn from(e: $type) -> Self {
                Self::$variant(e)
            }
        }
    };
}

convert!(View, crate::view::ViewError);
convert!(Sample, crate::sample::SampleError);
convert!(Check, crate::check::CheckError);
convert!(Report, crate::report::ReportError);
convert!(Profile, crate::render_profile::ProfileError);
convert!(Approve, crate::approve::ApproveError);
convert!(Io, std::io::Error);
