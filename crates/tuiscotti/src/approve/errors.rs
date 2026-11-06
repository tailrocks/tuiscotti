use std::fmt::{Display, Formatter};

use tui_test::{PinnedFontError, RenderError};

use crate::check::CheckError;
use crate::sample::SampleError;

#[derive(Debug)]
#[non_exhaustive]
#[doc = "Every failure an approval transaction can report, with sources retained."]
pub enum ApproveError {
    #[doc = "The `CI` variable is set, so approval is refused before any write."]
    CiApprovalForbidden,
    #[doc = "A digest argument is missing, malformed, or a zero placeholder."]
    InvalidArgument {
        #[doc = "Which argument failed and how."]
        detail: String,
    },
    #[doc = "The candidate bytes disagree with the supplied candidate digest."]
    StaleCandidate {
        #[doc = "The supplied candidate digest."]
        expected: String,
        #[doc = "The digest of the bytes actually read."]
        actual: String,
    },
    #[doc = "The present baseline disagrees with the expected previous digest."]
    StaleBaseline {
        #[doc = "The expected previous digest, or `missing` for a first approval."]
        expected: String,
        #[doc = "The digest of the bytes actually present, or `missing` when absent."]
        actual: String,
    },
    #[doc = "The approval lock stayed held past the acquisition deadline."]
    LockTimeout,
    #[doc = "The candidate names a render profile outside the D02 registry."]
    UnknownProfile {
        #[doc = "The rejected profile name."]
        requested: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The upstream renderer refused the candidate snapshot during the re-render."]
    RenderFailed {
        #[doc = "The upstream rejection, with its source chain."]
        source: RenderError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "No specified font can render mandatory text in the candidate snapshot."]
    MissingGlyph {
        #[doc = "The upstream missing-glyph rejection."]
        source: PinnedFontError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The re-rendered pixels differ from the stored candidate pixels."]
    InconsistentSample {
        #[doc = "What disagreed (dimensions and compared byte counts)."]
        detail: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "A baseline or evidence path is empty, unresolvable, or escapes its root."]
    InvalidStorePath {
        #[doc = "What the path broke."]
        detail: String,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "The inner frozen comparison, candidate validation, or record serialization failed."]
    Check {
        #[doc = "The frozen-store failure, with its source chain."]
        source: CheckError,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
    #[doc = "A filesystem failure outside sample validation."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
        #[doc = "The baseline digest observed under the lock, or `None` before the baseline read."]
        observed_baseline: Option<String>,
    },
}

impl Display for ApproveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CiApprovalForbidden => {
                write!(formatter, "approval is forbidden while CI is set")
            }
            Self::InvalidArgument { detail } => {
                write!(formatter, "invalid approval argument: {detail}")
            }
            Self::StaleCandidate { expected, actual } => write!(
                formatter,
                "candidate changed: expected digest {expected}, read bytes digest {actual}"
            ),
            Self::StaleBaseline { expected, actual } => write!(
                formatter,
                "baseline changed: expected previous digest {expected}, present digest {actual}"
            ),
            Self::LockTimeout => write!(formatter, "approval lock deadline passed"),
            Self::UnknownProfile { requested, .. } => {
                write!(formatter, "unknown render profile {requested:?}")
            }
            Self::RenderFailed { source, .. } => {
                write!(formatter, "candidate re-render failed: {source}")
            }
            Self::MissingGlyph { source, .. } => {
                write!(
                    formatter,
                    "candidate re-render needs an unavailable glyph: {source}"
                )
            }
            Self::InconsistentSample { detail, .. } => {
                write!(formatter, "candidate re-render is inconsistent: {detail}")
            }
            Self::InvalidStorePath { detail, .. } => {
                write!(formatter, "invalid store path: {detail}")
            }
            Self::Check { source, .. } => write!(formatter, "{source}"),
            Self::Io { source, .. } => write!(formatter, "approval I/O failed: {source}"),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Check { source, .. } => Some(source),
            Self::RenderFailed { source, .. } => Some(source),
            Self::MissingGlyph { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<CheckError> for ApproveError {
    fn from(source: CheckError) -> Self {
        Self::Check {
            source,
            observed_baseline: None,
        }
    }
}

impl From<SampleError> for ApproveError {
    fn from(source: SampleError) -> Self {
        Self::Check {
            source: CheckError::from(source),
            observed_baseline: None,
        }
    }
}

impl From<std::io::Error> for ApproveError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            source,
            observed_baseline: None,
        }
    }
}

impl ApproveError {
    #[must_use]
    #[doc = "The stable machine code for this failure, shared with the CLI map."]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CiApprovalForbidden => "CI_APPROVAL_FORBIDDEN",
            Self::InvalidArgument { .. } => "INVALID_SAMPLE",
            Self::StaleCandidate { .. } => "STALE_CANDIDATE",
            Self::StaleBaseline { .. } => "STALE_BASELINE",
            Self::LockTimeout => "LOCK_TIMEOUT",
            Self::UnknownProfile { .. } => "UNKNOWN_PROFILE",
            Self::RenderFailed { .. } => "RENDER_FAILED",
            Self::MissingGlyph { .. } => "MISSING_GLYPH",
            Self::InconsistentSample { .. } => "INCONSISTENT_SAMPLE",
            Self::InvalidStorePath { .. } => "INVALID_STORE_PATH",
            Self::Check { source, .. } => match source {
                CheckError::Sample(source) => sample_code(source),
                CheckError::InvalidTestId { .. } | CheckError::InvalidStorePath { .. } => {
                    "INVALID_STORE_PATH"
                }
                CheckError::Pixel(_) => "INVALID_IMAGE",
                CheckError::EvidenceCorrupt { .. }
                | CheckError::Image { .. }
                | CheckError::Io { .. }
                | CheckError::Report(_) => "IO",
            },
            Self::Io { .. } => "IO",
        }
    }

    #[must_use]
    #[doc = "The digest the baseline root actually held when this failure was reported."]
    pub fn observed_baseline(&self) -> Option<&str> {
        match self {
            Self::StaleBaseline { actual, .. } => Some(actual),
            Self::UnknownProfile {
                observed_baseline, ..
            }
            | Self::RenderFailed {
                observed_baseline, ..
            }
            | Self::MissingGlyph {
                observed_baseline, ..
            }
            | Self::InconsistentSample {
                observed_baseline, ..
            }
            | Self::InvalidStorePath {
                observed_baseline, ..
            }
            | Self::Check {
                observed_baseline, ..
            }
            | Self::Io {
                observed_baseline, ..
            } => observed_baseline.as_deref(),
            _ => None,
        }
    }

    #[must_use]
    pub(super) fn with_observed_baseline(mut self, observed: &str) -> Self {
        let (Self::UnknownProfile {
            observed_baseline: slot,
            ..
        }
        | Self::RenderFailed {
            observed_baseline: slot,
            ..
        }
        | Self::MissingGlyph {
            observed_baseline: slot,
            ..
        }
        | Self::InconsistentSample {
            observed_baseline: slot,
            ..
        }
        | Self::InvalidStorePath {
            observed_baseline: slot,
            ..
        }
        | Self::Check {
            observed_baseline: slot,
            ..
        }
        | Self::Io {
            observed_baseline: slot,
            ..
        }) = &mut self
        else {
            return self;
        };
        *slot = Some(observed.to_string());
        self
    }
}

#[must_use]
#[doc = "The stable machine code for one validated sample failure, shared with the CLI map."]
pub fn sample_code(source: &SampleError) -> &'static str {
    match source {
        SampleError::TooLarge { .. }
        | SampleError::DimensionTooLarge { .. }
        | SampleError::RgbaTooLarge { .. }
        | SampleError::ResourceLimit { .. }
        | SampleError::Allocation { .. }
        | SampleError::CodecAllocation { .. }
        | SampleError::ZstdResourceLimit { .. }
        | SampleError::EncodeAllocation { .. }
        | SampleError::PngResourceLimit { .. } => "RESOURCE_LIMIT",
        SampleError::UnsupportedSchema { .. }
        | SampleError::UnsupportedContainerVersion { .. }
        | SampleError::UnsupportedCodec { .. }
        | SampleError::UnsupportedMessageVersion { .. } => "UNSUPPORTED_SCHEMA",
        SampleError::PngInvalid { .. }
        | SampleError::PngDecode { .. }
        | SampleError::Base64Invalid { .. }
        | SampleError::DigestMismatch { .. }
        | SampleError::DimensionMismatch { .. }
        | SampleError::PngCodec { .. }
        | SampleError::PngColor { .. }
        | SampleError::ZeroImageDimensions
        | SampleError::ImageDimensionsMismatch { .. } => "INVALID_IMAGE",
        SampleError::Render { source } => match source {
            RenderError::PinnedFont(PinnedFontError::MissingGlyph { .. }) => "MISSING_GLYPH",
            _ => "RENDER_FAILED",
        },
        SampleError::Io { .. } => "IO",
        _ => "INVALID_SAMPLE",
    }
}
