use std::fmt::{Display, Formatter};

use tui_test::RASTER_POLICY_PINNED_V1;

use super::{CONTRACT_VERSION, PngBitDepth, PngColorType, SAMPLE_SCHEMA, SampleError};

impl Display for SampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContainerVersion { version } => {
                write!(f, "unsupported container version {version}")
            }
            Self::UnsupportedCodec { codec } => write!(f, "unsupported codec {codec}"),
            Self::UnsupportedMessageVersion { version } => {
                write!(f, "unsupported message version {version}")
            }
            Self::CodecAllocation { site } => write!(f, "codec allocation failed at {site}"),
            Self::ZstdResourceLimit { source } => write!(f, "codec allocation failed: {source}"),
            Self::EncodeCodec { source } => write!(f, "sample compression failed: {source}"),
            Self::BinaryFormat { source } => write!(f, "invalid binary sample: {source}"),
            Self::DecodeInvariant { detail } => write!(f, "sample decoding invariant: {detail}"),
            Self::EncodeInvariant { detail } => write!(f, "sample encoding invariant: {detail}"),
            Self::EncodeAllocation { source } => {
                write!(f, "sample encoding allocation failed: {source}")
            }
            Self::ResourceLimit {
                resource,
                claimed,
                limit,
                ..
            } => write!(f, "{resource} {claimed:?} exceeds {limit}"),
            Self::Allocation {
                site, requested, ..
            } => write!(f, "allocation failed at {site} for {requested} bytes"),
            Self::TooLarge { what, bytes, limit } => {
                write!(f, "{what} is {bytes} bytes, above the {limit} byte limit")
            }
            Self::InvalidTestId { detail } => write!(f, "invalid test ID: {detail}"),
            Self::GateMismatch { detail } => write!(f, "gate mismatch: {detail}"),
            Self::UnsupportedSchema { version } => write!(
                f,
                "unsupported sample schema {version:?}; expected {SAMPLE_SCHEMA:?}"
            ),
            Self::UnknownField { detail } => write!(f, "unknown sample field: {detail}"),
            Self::InvalidFormat { source } => write!(f, "invalid sample encoding: {source}"),
            Self::InvalidSnapshot { detail } => write!(f, "invalid embedded snapshot: {detail}"),
            Self::DimensionTooLarge {
                width,
                height,
                limit,
            } => fmt_dimension_too_large(f, *width, *height, *limit),
            Self::RgbaTooLarge { bytes, limit } => fmt_rgba_too_large(f, *bytes, *limit),
            Self::DimensionMismatch { detail } => write!(f, "image dimension mismatch: {detail}"),
            Self::MissingDifferenceImage { detail } => {
                write!(f, "missing difference image: {detail}")
            }
            Self::PngCodec { source } => write!(f, "sample PNG does not decode: {source}"),
            Self::PngResourceLimit { limit, .. } => {
                write!(f, "PNG decoder counter bytes None exceeds {limit}")
            }
            Self::PngColor { color, depth } => fmt_png_color(f, *color, *depth),
            Self::ZeroImageDimensions => {
                f.write_str("image dimension mismatch: image dimensions are zero")
            }
            Self::ImageDimensionsMismatch { claimed, actual } => {
                fmt_image_dimensions_mismatch(f, *claimed, *actual)
            }
            Self::PngInvalid { detail } => write!(f, "invalid sample PNG: {detail}"),
            Self::PngDecode { source } => write!(f, "sample PNG does not decode: {source}"),
            Self::Base64Invalid { source } => {
                write!(f, "sample image is not valid base64: {source}")
            }
            Self::DigestMismatch {
                field,
                expected,
                actual,
            } => write!(f, "{field} {expected} disagrees with recomputed {actual}"),
            Self::Render { source } => write!(f, "sample render failed: {source}"),
            Self::UnexpectedPolicy { policy } => write!(
                f,
                "render recorded raster policy {policy:?}; expected {RASTER_POLICY_PINNED_V1:?}"
            ),
            Self::UnsupportedContract { version } => write!(
                f,
                "unsupported render contract version {version}; expected {CONTRACT_VERSION}"
            ),
            Self::Io { source } => write!(f, "sample file I/O failed: {source}"),
        }
    }
}

impl std::error::Error for SampleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZstdResourceLimit { source } | Self::EncodeCodec { source } => Some(source),
            Self::BinaryFormat { source } => Some(source),
            Self::EncodeAllocation { source } | Self::Allocation { source, .. } => Some(source),
            Self::ResourceLimit {
                source: Some(source),
                ..
            } => Some(source.as_ref()),
            Self::InvalidFormat { source } => Some(source),
            Self::PngDecode { source } => Some(source),
            Self::PngCodec { source } | Self::PngResourceLimit { source, .. } => Some(source),
            Self::Base64Invalid { source } => Some(source),
            Self::Render { source } => Some(source),
            Self::Io { source } => Some(source),
            _ => None,
        }
    }
}

fn fmt_png_color(
    f: &mut Formatter<'_>,
    color: super::PngColorType,
    depth: super::PngBitDepth,
) -> std::fmt::Result {
    let mapped = match (color, depth) {
        (PngColorType::Grayscale, PngBitDepth::Eight) => Some(image::ColorType::L8),
        (PngColorType::Grayscale, PngBitDepth::Sixteen) => Some(image::ColorType::L16),
        (PngColorType::GrayscaleAlpha, PngBitDepth::Eight) => Some(image::ColorType::La8),
        (PngColorType::GrayscaleAlpha, PngBitDepth::Sixteen) => Some(image::ColorType::La16),
        (PngColorType::Rgb, PngBitDepth::Eight) => Some(image::ColorType::Rgb8),
        (PngColorType::Rgb, PngBitDepth::Sixteen) => Some(image::ColorType::Rgb16),
        (PngColorType::Rgba, PngBitDepth::Sixteen) => Some(image::ColorType::Rgba16),
        _ => None,
    };
    match mapped {
        Some(color) => write!(
            f,
            "invalid sample PNG: PNG color type is {color:?}; only 8-bit RGBA is supported"
        ),
        None => write!(
            f,
            "invalid sample PNG: PNG color type is {color:?} at {depth:?}; only 8-bit RGBA is supported"
        ),
    }
}

fn fmt_dimension_too_large(
    f: &mut Formatter<'_>,
    width: u32,
    height: u32,
    limit: u32,
) -> std::fmt::Result {
    write!(
        f,
        "image dimensions {width}x{height} pass the {limit} pixel per-dimension limit"
    )
}

fn fmt_rgba_too_large(f: &mut Formatter<'_>, bytes: u64, limit: u64) -> std::fmt::Result {
    write!(
        f,
        "decoded RGBA is {bytes} bytes, above the {limit} byte limit"
    )
}

fn fmt_image_dimensions_mismatch(
    f: &mut Formatter<'_>,
    claimed: (u32, u32),
    actual: (u32, u32),
) -> std::fmt::Result {
    write!(
        f,
        "image dimension mismatch: sample claims {}x{} but the PNG decodes to {}x{}",
        claimed.0, claimed.1, actual.0, actual.1
    )
}
