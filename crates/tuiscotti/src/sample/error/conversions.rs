use super::{RenderError, SampleError};

impl From<RenderError> for SampleError {
    fn from(source: RenderError) -> Self {
        Self::Render { source }
    }
}

impl From<super::super::decode_error::DecodeError> for SampleError {
    fn from(error: super::super::decode_error::DecodeError) -> Self {
        use super::super::decode_error::DecodeError as D;
        match error {
            D::CodecAllocation { source } => Self::ZstdResourceLimit { source },
            D::Domain(error) => error,
            D::Resource {
                resource,
                claimed,
                limit,
                source,
            } => Self::ResourceLimit {
                resource,
                claimed,
                limit,
                source,
            },
            D::Allocation {
                site,
                requested,
                source,
            } => Self::Allocation {
                site,
                requested,
                source,
            },
            D::ContextAllocation { site } => Self::CodecAllocation { site },
            D::UnsupportedContainerVersion(version) => {
                Self::UnsupportedContainerVersion { version }
            }
            D::UnsupportedCodec(codec) => Self::UnsupportedCodec { codec },
            D::UnsupportedMessageVersion(version) => Self::UnsupportedMessageVersion { version },
            D::UnsupportedSchema(version) => Self::UnsupportedSchema { version },
            D::Invariant { detail } => Self::DecodeInvariant { detail },
            D::Format(source) => Self::BinaryFormat { source },
        }
    }
}

impl From<super::super::decode_error::EncodeError> for SampleError {
    fn from(error: super::super::decode_error::EncodeError) -> Self {
        use super::super::decode_error::EncodeError as E;
        match error {
            E::Limit {
                resource,
                claimed,
                limit,
            } => Self::ResourceLimit {
                resource,
                claimed,
                limit,
                source: None,
            },
            E::Allocation {
                site,
                requested,
                source,
            } => Self::Allocation {
                site,
                requested,
                source,
            },
            E::ContextAllocation { site } => Self::CodecAllocation { site },
            E::Codec { source } => Self::EncodeCodec { source },
            E::Invariant { detail } => Self::EncodeInvariant { detail },
        }
    }
}
