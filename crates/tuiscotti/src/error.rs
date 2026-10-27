#[doc = include_str!("../docs/error.md")]
use std::fmt::{Display, Formatter};

#[doc = "Fallible-operation result over the facade Error."]
pub type Result<T> = std::result::Result<T, Error>;

#[doc = "Every failure a facade operation can report, with sources retained."]
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    #[doc = "Pure-view capture or conversion failed."]
    View(crate::view::ViewError),
    #[doc = "Sample construction, validation, or rendering failed."]
    Sample(crate::sample::SampleError),
    #[doc = "Frozen comparison or candidate evidence failed."]
    Check(crate::check::CheckError),
    #[doc = "Static HTML report rendering failed."]
    Report(crate::report::ReportError),
    #[doc = "Render profile lookup or build failed."]
    Profile(crate::render_profile::ProfileError),
    #[doc = "Approval transaction failed."]
    Approve(crate::approve::ApproveError),
    #[doc = "Filesystem failure."]
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
