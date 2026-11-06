//! Common types shared between the encoder and decoder
use crate::sample::embedded_png::png::Filter;
use std::{convert::TryFrom, fmt};

mod animation;
pub(crate) use animation::{AnimationControl, FrameControl};
mod compression;
pub(crate) use compression::{Compression, DeflateCompression};
mod colorimetry;
pub(crate) use colorimetry::{
    CodingIndependentCodePoints, ContentLightLevelInfo, MasteringDisplayColorVolume, ScaledFloat,
    SourceChromaticities, SrgbRenderingIntent,
};
mod info;
pub(crate) use info::Info;

/// Describes how a pixel is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ColorType {
    /// 1 grayscale sample.
    Grayscale = 0,
    /// 1 red sample, 1 green sample, 1 blue sample.
    Rgb = 2,
    /// 1 sample for the palette index.
    Indexed = 3,
    /// 1 grayscale sample, then 1 alpha sample.
    GrayscaleAlpha = 4,
    /// 1 red sample, 1 green sample, 1 blue sample, and finally, 1 alpha sample.
    Rgba = 6,
}

impl ColorType {
    /// Returns the number of samples used per pixel encoded in this way.
    pub(crate) fn samples(self) -> usize {
        self.samples_u8().into()
    }

    pub(crate) fn samples_u8(self) -> u8 {
        use self::ColorType::*;
        match self {
            Grayscale | Indexed => 1,
            Rgb => 3,
            GrayscaleAlpha => 2,
            Rgba => 4,
        }
    }

    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub(crate) fn from_u8(n: u8) -> Option<ColorType> {
        match n {
            0 => Some(ColorType::Grayscale),
            2 => Some(ColorType::Rgb),
            3 => Some(ColorType::Indexed),
            4 => Some(ColorType::GrayscaleAlpha),
            6 => Some(ColorType::Rgba),
            _ => None,
        }
    }

    pub(crate) fn checked_raw_row_length(self, depth: BitDepth, width: u32) -> Option<usize> {
        // No overflow can occur in 64 bits, we multiply 32-bit with 5 more bits.
        let bits = u64::from(width) * u64::from(self.samples_u8()) * u64::from(depth.into_u8());
        TryFrom::try_from(1 + bits.div_ceil(8)).ok()
    }

    pub(crate) fn raw_row_length_from_width(self, depth: BitDepth, width: u32) -> usize {
        let samples = width as usize * self.samples();
        1 + match depth {
            BitDepth::Sixteen => samples * 2,
            BitDepth::Eight => samples,
            subbyte => {
                let samples_per_byte = 8 / subbyte as usize;
                let whole = samples / samples_per_byte;
                let fract = usize::from(samples % samples_per_byte > 0);
                whole + fract
            }
        }
    }

    pub(crate) fn is_combination_invalid(self, bit_depth: BitDepth) -> bool {
        // Section 11.2.2 of the PNG standard disallows several combinations
        // of bit depth and color type
        ((bit_depth == BitDepth::One || bit_depth == BitDepth::Two || bit_depth == BitDepth::Four)
            && (self == ColorType::Rgb
                || self == ColorType::GrayscaleAlpha
                || self == ColorType::Rgba))
            || (bit_depth == BitDepth::Sixteen && self == ColorType::Indexed)
    }

    pub(crate) fn bits_per_pixel(&self, bit_depth: BitDepth) -> usize {
        self.samples() * bit_depth as usize
    }

    pub(crate) fn bytes_per_pixel(&self, bit_depth: BitDepth) -> usize {
        // If adjusting this for expansion or other transformation passes, remember to keep the old
        // implementation for bpp_in_prediction, which is internal to the png specification.
        self.samples() * ((bit_depth as usize + 7) >> 3)
    }
}

/// Bit depth of the PNG file.
/// Specifies the number of bits per sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum BitDepth {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

/// Internal count of bytes per pixel.
/// This is used for filtering which never uses sub-byte units. This essentially reduces the number
/// of possible byte chunk lengths to a very small set of values appropriate to be defined as an
/// enum.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum BytesPerPixel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Six = 6,
    Eight = 8,
}

impl BitDepth {
    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub(crate) fn from_u8(n: u8) -> Option<BitDepth> {
        match n {
            1 => Some(BitDepth::One),
            2 => Some(BitDepth::Two),
            4 => Some(BitDepth::Four),
            8 => Some(BitDepth::Eight),
            16 => Some(BitDepth::Sixteen),
            _ => None,
        }
    }

    pub(crate) fn into_u8(self) -> u8 {
        self as u8
    }
}

/// Pixel dimensions information
#[derive(Clone, Copy, Debug)]
pub(crate) struct PixelDimensions {
    /// Pixels per unit, X axis
    pub(crate) xppu: u32,
    /// Pixels per unit, Y axis
    pub(crate) yppu: u32,
    /// Either *Meter* or *Unspecified*
    pub(crate) unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
/// Physical unit of the pixel dimensions
pub(crate) enum Unit {
    Unspecified = 0,
    Meter = 1,
}

impl Unit {
    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub(crate) fn from_u8(n: u8) -> Option<Unit> {
        match n {
            0 => Some(Unit::Unspecified),
            1 => Some(Unit::Meter),
            _ => None,
        }
    }
}

/// How to reset buffer of an animated png (APNG) at the end of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum DisposeOp {
    /// Leave the buffer unchanged.
    None = 0,
    /// Clear buffer with the background color.
    Background = 1,
    /// Reset the buffer to the state before the current frame.
    Previous = 2,
}

impl DisposeOp {
    /// u8 -> Self. Using enum_primitive or transmute is probably the right thing but this will do for now.
    pub(crate) fn from_u8(n: u8) -> Option<DisposeOp> {
        match n {
            0 => Some(DisposeOp::None),
            1 => Some(DisposeOp::Background),
            2 => Some(DisposeOp::Previous),
            _ => None,
        }
    }
}

impl fmt::Display for DisposeOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            DisposeOp::None => "DISPOSE_OP_NONE",
            DisposeOp::Background => "DISPOSE_OP_BACKGROUND",
            DisposeOp::Previous => "DISPOSE_OP_PREVIOUS",
        };
        write!(f, "{}", name)
    }
}

/// How pixels are written into the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum BlendOp {
    /// Pixels overwrite the value at their position.
    Source = 0,
    /// The new pixels are blended into the current state based on alpha.
    Over = 1,
}

impl BlendOp {
    /// u8 -> Self. Using enum_primitive or transmute is probably the right thing but this will do for now.
    pub(crate) fn from_u8(n: u8) -> Option<BlendOp> {
        match n {
            0 => Some(BlendOp::Source),
            1 => Some(BlendOp::Over),
            _ => None,
        }
    }
}

impl fmt::Display for BlendOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            BlendOp::Source => "BLEND_OP_SOURCE",
            BlendOp::Over => "BLEND_OP_OVER",
        };
        write!(f, "{}", name)
    }
}

impl BytesPerPixel {
    pub(crate) fn from_usize(bpp: usize) -> Self {
        match bpp {
            1 => BytesPerPixel::One,
            2 => BytesPerPixel::Two,
            3 => BytesPerPixel::Three,
            4 => BytesPerPixel::Four,
            6 => BytesPerPixel::Six,   // Only rgb×16bit
            8 => BytesPerPixel::Eight, // Only rgba×16bit
            _ => unreachable!("Not a possible byte rounded pixel width"),
        }
    }

    pub(crate) fn into_usize(self) -> usize {
        self as usize
    }
}

bitflags::bitflags! {
    /// Output transformations
    ///
    /// Many flags from libpng are not yet supported. A PR discussing/adding them would be nice.
    ///
    #[doc = "
    ```c
    /// Discard the alpha channel
    const STRIP_ALPHA         = 0x0002; // read only
    /// Expand 1; 2 and 4-bit samples to bytes
    const PACKING             = 0x0004; // read and write
    /// Change order of packed pixels to LSB first
    const PACKSWAP            = 0x0008; // read and write
    /// Invert monochrome images
    const INVERT_MONO         = 0x0020; // read and write
    /// Normalize pixels to the sBIT depth
    const SHIFT               = 0x0040; // read and write
    /// Flip RGB to BGR; RGBA to BGRA
    const BGR                 = 0x0080; // read and write
    /// Flip RGBA to ARGB or GA to AG
    const SWAP_ALPHA          = 0x0100; // read and write
    /// Byte-swap 16-bit samples
    const SWAP_ENDIAN         = 0x0200; // read and write
    /// Change alpha from opacity to transparency
    const INVERT_ALPHA        = 0x0400; // read and write
    const STRIP_FILLER        = 0x0800; // write only
    const STRIP_FILLER_BEFORE = 0x0800; // write only
    const STRIP_FILLER_AFTER  = 0x1000; // write only
    const GRAY_TO_RGB         = 0x2000; // read only
    const EXPAND_16           = 0x4000; // read only
    /// Similar to STRIP_16 but in libpng considering gamma?
    /// Not entirely sure the documentation says it is more
    /// accurate but doesn't say precisely how.
    const SCALE_16            = 0x8000; // read only
    ```
    "]
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub(crate) struct Transformations: u32 {
        /// No transformation
        const IDENTITY            = 0x00000; // read and write */
        /// Strip 16-bit samples to 8 bits
        const STRIP_16            = 0x00001; // read only */
        /// Expand paletted images to RGB; expand grayscale images of
        /// less than 8-bit depth to 8-bit depth; and expand tRNS chunks
        /// to alpha channels.
        const EXPAND              = 0x00010; // read only */
        /// Expand paletted images to include an alpha channel. Implies `EXPAND`.
        const ALPHA               = 0x10000; // read only */
    }
}

impl Transformations {
    /// Transform every input to 8bit grayscale or color.
    ///
    /// This sets `EXPAND` and `STRIP_16` which is similar to the default transformation used by
    /// this library prior to `0.17`.
    pub(crate) fn normalize_to_color8() -> Transformations {
        Transformations::EXPAND | Transformations::STRIP_16
    }
}

/// Instantiate the default transformations, the identity transform.
impl Default for Transformations {
    fn default() -> Transformations {
        Transformations::IDENTITY
    }
}

#[derive(Debug)]
pub(crate) struct ParameterError {
    inner: ParameterErrorKind,
}

#[derive(Debug)]
pub(crate) enum ParameterErrorKind {
    /// A provided buffer must be have the exact size to hold the image data. Where the buffer can
    /// be allocated by the caller, they must ensure that it has a minimum size as hinted previously.
    /// Even though the size is calculated from image data, this does counts as a parameter error
    /// because they must react to a value produced by this library, which can have been subjected
    /// to limits.
    ImageBufferSize { expected: usize, actual: usize },
    /// A bit like return `None` from an iterator.
    /// We use it to differentiate between failing to seek to the next image in a sequence and the
    /// absence of a next image. This is an error of the caller because they should have checked
    /// the number of images by inspecting the header data returned when opening the image. This
    /// library will perform the checks necessary to ensure that data was accurate or error with a
    /// format error otherwise.
    PolledAfterEndOfImage,
    /// Attempt to continue decoding after a fatal, non-resumable error was reported (e.g. after
    /// [`DecodingError::Format`]).  The only case when it is possible to resume after an error
    /// is an `UnexpectedEof` scenario - see [`DecodingError::IoError`].
    PolledAfterFatalError,
}

impl From<ParameterErrorKind> for ParameterError {
    fn from(inner: ParameterErrorKind) -> Self {
        ParameterError { inner }
    }
}

impl fmt::Display for ParameterError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use ParameterErrorKind::*;
        match self.inner {
            ImageBufferSize { expected, actual } => {
                write!(fmt, "wrong data size, expected {} got {}", expected, actual)
            }
            PolledAfterEndOfImage => write!(fmt, "End of image has been reached"),
            PolledAfterFatalError => {
                write!(fmt, "A fatal decoding error has been encounted earlier")
            }
        }
    }
}
