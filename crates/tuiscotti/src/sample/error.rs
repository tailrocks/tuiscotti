use std::collections::TryReserveError;
use std::fmt::{Display, Formatter};

use tui_test::RenderError;

use super::{CONTRACT_VERSION, SAMPLE_SCHEMA};

/// The PNG color type reported by sample-image validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PngColorType {
    /// Grayscale samples.
    Grayscale,
    /// Red, green, and blue samples.
    Rgb,
    /// A palette index sample.
    Indexed,
    /// Grayscale and alpha samples.
    GrayscaleAlpha,
    /// Red, green, blue, and alpha samples.
    Rgba,
}

/// The sample PNG's encoded bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PngBitDepth {
    /// One bit per sample.
    One,
    /// Two bits per sample.
    Two,
    /// Four bits per sample.
    Four,
    /// Eight bits per sample.
    Eight,
    /// Sixteen bits per sample.
    Sixteen,
}

impl From<super::embedded_png::png::ColorType> for PngColorType {
    fn from(value: super::embedded_png::png::ColorType) -> Self {
        use super::embedded_png::png::ColorType as C;
        match value {
            C::Grayscale => Self::Grayscale,
            C::Rgb => Self::Rgb,
            C::Indexed => Self::Indexed,
            C::GrayscaleAlpha => Self::GrayscaleAlpha,
            C::Rgba => Self::Rgba,
        }
    }
}

impl From<super::embedded_png::png::BitDepth> for PngBitDepth {
    fn from(value: super::embedded_png::png::BitDepth) -> Self {
        use super::embedded_png::png::BitDepth as B;
        match value {
            B::One => Self::One,
            B::Two => Self::Two,
            B::Four => Self::Four,
            B::Eight => Self::Eight,
            B::Sixteen => Self::Sixteen,
        }
    }
}

/// A PNG decoding failure retained without exposing the private decoder implementation.
#[derive(Debug)]
pub struct PngDecoderError {
    source: super::embedded_png::png::DecodingError,
}

impl PngDecoderError {
    pub(crate) fn new(source: super::embedded_png::png::DecodingError) -> Self {
        Self { source }
    }
}

impl Display for PngDecoderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl std::error::Error for PngDecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

const _: () = assert!(std::mem::size_of::<PngDecoderError>() <= 4096);

#[derive(Debug)]
#[doc = "Every failure a sample operation can report, with sources retained."]
#[non_exhaustive]
pub enum SampleError {
    #[doc = "The binary container version is unsupported."]
    UnsupportedContainerVersion {
        #[doc = "The version."]
        version: u16,
    },
    #[doc = "The binary codec is unsupported."]
    UnsupportedCodec {
        #[doc = "The codec identifier."]
        codec: u16,
    },
    #[doc = "The binary message version is unsupported."]
    UnsupportedMessageVersion {
        #[doc = "The version."]
        version: u32,
    },
    #[doc = "Codec context allocation failed."]
    CodecAllocation {
        #[doc = "The allocation site."]
        site: &'static str,
    },
    #[doc = "Zstandard decoder allocation failed."]
    ZstdResourceLimit {
        #[doc = "The codec failure."]
        source: super::decode_error::ZstdError,
    },
    #[doc = "Compression failed."]
    EncodeCodec {
        #[doc = "The codec failure."]
        source: super::decode_error::ZstdError,
    },
    #[doc = "Binary sample encoding failed validation."]
    BinaryFormat {
        #[doc = "The binary decoding failure."]
        source: super::decode_error::BinaryFormatError,
    },
    #[doc = "Binary decoding failed an internal invariant."]
    DecodeInvariant {
        #[doc = "The invariant."]
        detail: &'static str,
    },
    #[doc = "Encoding failed an internal invariant."]
    EncodeInvariant {
        #[doc = "The invariant."]
        detail: &'static str,
    },
    #[doc = "An encoding allocation failed."]
    EncodeAllocation {
        #[doc = "The allocation failure."]
        source: TryReserveError,
    },
    #[doc = "A decoder or encoder resource limit was exceeded."]
    ResourceLimit {
        #[doc = "The resource being measured."]
        resource: &'static str,
        #[doc = "The observed amount, when known."]
        claimed: Option<u64>,
        #[doc = "The enforced limit."]
        limit: u64,
        #[doc = "The underlying PNG limit failure, when present."]
        source: Option<Box<PngDecoderError>>,
    },
    #[doc = "A fallible allocation failed."]
    Allocation {
        #[doc = "The allocation site."]
        site: &'static str,
        #[doc = "The requested number of bytes."]
        requested: u64,
        #[doc = "The allocation failure."]
        source: TryReserveError,
    },
    #[doc = "The encoded file passes the 64 MiB limit."]
    TooLarge {
        #[doc = "What was measured."]
        what: &'static str,
        #[doc = "The rejected byte count."]
        bytes: u64,
        #[doc = "The accepted limit."]
        limit: u64,
    },
    #[doc = "A test ID breaks its shape rules."]
    InvalidTestId {
        #[doc = "What the test ID broke."]
        detail: String,
    },
    #[doc = "A gate and its evidence disagree, or a test ID and gate disagree."]
    GateMismatch {
        #[doc = "What disagreed."]
        detail: String,
    },
    #[doc = "The schema identifier is not the supported sample schema."]
    UnsupportedSchema {
        #[doc = "The rejected schema value."]
        version: String,
    },
    #[doc = "The encoded sample carries a field the schema does not define."]
    UnknownField {
        #[doc = "The upstream serde report, kept verbatim."]
        detail: String,
    },
    #[doc = "The encoded envelope is not valid JSON for this schema."]
    InvalidFormat {
        #[doc = "The decoding failure."]
        source: serde_json::Error,
    },
    #[doc = "The embedded snapshot fails upstream validation."]
    InvalidSnapshot {
        #[doc = "The upstream validation report, kept verbatim."]
        detail: String,
    },
    #[doc = "An image dimension passes its limit."]
    DimensionTooLarge {
        #[doc = "The rejected width."]
        width: u32,
        #[doc = "The rejected height."]
        height: u32,
        #[doc = "The accepted per-dimension limit."]
        limit: u32,
    },
    #[doc = "Decoded RGBA bytes pass the 256 MiB limit."]
    RgbaTooLarge {
        #[doc = "The rejected byte count."]
        bytes: u64,
        #[doc = "The accepted limit."]
        limit: u64,
    },
    #[doc = "Claimed image dimensions disagree with decoded PNG dimensions."]
    DimensionMismatch {
        #[doc = "What disagreed."]
        detail: String,
    },
    #[doc = "Upstream pixel comparison reports changed pixels without a difference image."]
    MissingDifferenceImage {
        #[doc = "What the comparison reported."]
        detail: String,
    },
    #[doc = "The embedded PNG does not decode as 8-bit RGBA."]
    PngInvalid {
        #[doc = "What decoding found."]
        detail: String,
    },
    #[doc = "PNG decoding failed with a concrete decoder error."]
    PngCodec {
        #[doc = "The PNG decoder failure."]
        source: PngDecoderError,
    },
    #[doc = "The PNG decoder exceeded its allocation counter."]
    PngResourceLimit {
        #[doc = "The counter limit."]
        limit: u64,
        #[doc = "The PNG decoder failure."]
        source: PngDecoderError,
    },
    #[doc = "The PNG does not decode to 8-bit RGBA."]
    PngColor {
        #[doc = "The decoded PNG color type."]
        color: PngColorType,
        #[doc = "The decoded PNG bit depth."]
        depth: PngBitDepth,
    },
    #[doc = "The image dimensions are zero."]
    ZeroImageDimensions,
    #[doc = "Claimed image dimensions disagree with decoded PNG dimensions."]
    ImageDimensionsMismatch {
        #[doc = "Claimed width and height."]
        claimed: (u32, u32),
        #[doc = "Decoded width and height."]
        actual: (u32, u32),
    },
    #[doc = "PNG decoding failed."]
    PngDecode {
        #[doc = "The decoding failure."]
        source: image::ImageError,
    },
    #[doc = "Embedded base64 does not decode."]
    Base64Invalid {
        #[doc = "The decoding failure."]
        source: base64::DecodeError,
    },
    #[doc = "A recorded digest disagrees with recomputed bytes."]
    DigestMismatch {
        #[doc = "Which digest disagreed (`png_sha256` or `rgba_sha256`)."]
        field: &'static str,
        #[doc = "The recorded value."]
        expected: String,
        #[doc = "The recomputed value."]
        actual: String,
    },
    #[doc = "The upstream renderer refused the snapshot."]
    Render {
        #[doc = "The upstream rejection, with its source chain."]
        source: RenderError,
    },
    #[doc = "The upstream pinned raster policy is not the recorded strict policy."]
    UnexpectedPolicy {
        #[doc = "The policy the render recorded."]
        policy: String,
    },
    #[doc = "The downstream contract version is not the supported one."]
    UnsupportedContract {
        #[doc = "The rejected contract version."]
        version: u32,
    },
    #[doc = "Filesystem failure while reading or writing a sample file."]
    Io {
        #[doc = "The filesystem failure."]
        source: std::io::Error,
    },
}

mod conversions;
mod display;
