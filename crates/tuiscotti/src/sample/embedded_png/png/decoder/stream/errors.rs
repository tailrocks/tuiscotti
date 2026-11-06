use crate::sample::embedded_png::png::chunk::ChunkType;
use crate::sample::embedded_png::png::common::{BitDepth, ColorType, ParameterError};
use crate::sample::embedded_png::png::text_metadata::TextDecodingError;
use std::collections::TryReserveError;
use std::io;

#[derive(Debug)]
/// Result of the decoding process
pub(crate) enum Decoded {
    /// Nothing decoded yet
    Nothing,

    /// A chunk header (length and type fields) has been read.
    ChunkBegin(u32, ChunkType),

    /// Chunk has been read successfully.
    ChunkComplete(ChunkType),

    /// An ancillary chunk has been read but it was in the wrong place, had corrupt contents, or had
    /// an invalid CRC.
    BadAncillaryChunk(ChunkType),

    /// Skipped an ancillary chunk because it was unrecognized or the decoder was configured to skip
    /// this type of chunk.
    SkippedAncillaryChunk(ChunkType),

    /// Decoded raw image data.
    ImageData,

    /// The last of a consecutive chunk of IDAT was done.
    /// This is distinct from ChunkComplete which only marks that some IDAT chunk was completed but
    /// not that no additional IDAT chunk follows.
    ImageDataFlushed,
}

/// Any kind of error during PNG decoding.
///
/// This enumeration provides a very rough analysis on the origin of the failure. That is, each
/// variant corresponds to one kind of actor causing the error. It should not be understood as a
/// direct blame but can inform the search for a root cause or if such a search is required.
#[derive(Debug)]
pub(crate) enum DecodingError {
    /// An error in IO of the underlying reader.
    ///
    /// Note that some IO errors may be recoverable - decoding may be retried after the
    /// error is resolved.  For example, decoding from a slow stream of data (e.g. decoding from a
    /// network stream) may occasionally result in [std::io::ErrorKind::UnexpectedEof] kind of
    /// error, but decoding can resume when more data becomes available.
    IoError(io::Error),
    /// The input image was not a valid PNG.
    ///
    /// There isn't a lot that can be done here, except if the program itself was responsible for
    /// creating this image then investigate the generator. This is internally implemented with a
    /// large Enum. If You are interested in accessing some of the more exact information on the
    /// variant then we can discuss in an issue.
    Format(FormatError),
    /// An interface was used incorrectly.
    ///
    /// This is used in cases where it's expected that the programmer might trip up and stability
    /// could be affected. For example when:
    ///
    /// * The decoder is polled for more animation frames despite being done (or not being animated
    ///   in the first place).
    /// * The output buffer does not have the required size.
    ///
    /// As a rough guideline for introducing new variants parts of the requirements are dynamically
    /// derived from the (untrusted) input data while the other half is from the caller. In the
    /// above cases the number of frames respectively the size is determined by the file while the
    /// number of calls
    ///
    /// If you're an application you might want to signal that a bug report is appreciated.
    Parameter(ParameterError),
    /// The image would have required exceeding the limits configured with the decoder.
    ///
    /// Note that Your allocations, e.g. when reading into a pre-allocated buffer, is __NOT__
    /// considered part of the limits. Nevertheless, required intermediate buffers such as for
    /// singular lines is checked against the limit.
    ///
    /// Note that this is a best-effort basis.
    LimitsExceeded,
    /// A checked allocation request failed.
    Allocation {
        /// The bounded allocation site.
        site: &'static str,
        /// The requested layout size in bytes.
        requested: usize,
        /// The allocator's reserve error.
        source: TryReserveError,
    },
    /// The decoder's internal state is inconsistent.
    InvalidState,
}

impl DecodingError {
    pub(crate) fn allocation(
        site: &'static str,
        requested: usize,
        source: TryReserveError,
    ) -> Self {
        Self::Allocation {
            site,
            requested,
            source,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FormatError {
    inner: FormatErrorInner,
}

#[derive(Debug)]
pub(crate) enum FormatErrorInner {
    /// Bad framing.
    CrcMismatch {
        /// Stored CRC32 value
        crc_val: u32,
        /// Calculated CRC32 sum
        crc_sum: u32,
        /// The chunk type that has the CRC mismatch.
        chunk: ChunkType,
    },
    /// Not a PNG, the magic signature is missing.
    InvalidSignature,
    // Errors of chunk level ordering, missing etc.
    /// Fctl must occur if an animated chunk occurs.
    MissingFctl,
    /// Image data that was indicated in IHDR or acTL is missing.
    MissingImageData,
    /// 4.3., Must be first.
    ChunkBeforeIhdr {
        kind: ChunkType,
    },
    /// 4.3., some chunks must be before IDAT.
    AfterIdat {
        kind: ChunkType,
    },
    // 4.3., Some chunks must be after PLTE.
    BeforePlte {
        kind: ChunkType,
    },
    /// 4.3., some chunks must be before PLTE.
    AfterPlte {
        kind: ChunkType,
    },
    /// 4.3., some chunks must be between PLTE and IDAT.
    OutsidePlteIdat {
        kind: ChunkType,
    },
    /// 4.3., some chunks must be unique.
    DuplicateChunk {
        kind: ChunkType,
    },
    /// Specifically for fdat there is an embedded sequence number for chunks.
    ApngOrder {
        /// The sequence number in the chunk.
        present: u32,
        /// The one that should have been present.
        expected: u32,
    },
    // Errors specific to particular chunk data to be validated.
    /// The palette did not even contain a single pixel data.
    ShortPalette {
        expected: usize,
        len: usize,
    },
    /// sBIT chunk size based on color type.
    InvalidSbitChunkSize {
        color_type: ColorType,
        expected: usize,
        len: usize,
    },
    InvalidSbit {
        sample_depth: BitDepth,
        sbit: u8,
    },
    /// A palletized image did not have a palette.
    PaletteRequired,
    /// The color-depth combination is not valid according to Table 11.1.
    InvalidColorBitDepth {
        color_type: ColorType,
        bit_depth: BitDepth,
    },
    ColorWithBadTrns(ColorType),
    /// The image width or height is zero.
    InvalidDimensions,
    InvalidBitDepth(u8),
    InvalidColorType(u8),
    InvalidDisposeOp(u8),
    InvalidBlendOp(u8),
    InvalidUnit(u8),
    /// The rendering intent of the sRGB chunk is invalid.
    InvalidSrgbRenderingIntent(u8),
    UnknownCompressionMethod(u8),
    UnknownFilterMethod(u8),
    UnknownInterlaceMethod(u8),
    /// The subframe is not in bounds of the image.
    /// TODO: fields with relevant data.
    BadSubFrameBounds {},
    // Errors specific to the IDAT/fdAT chunks.
    /// The compression of the data stream was faulty.
    CorruptFlateStream {
        err: crate::sample::embedded_png::fdeflate::DecompressionError,
    },
    /// The image data chunk was too short for the expected pixel count.
    NoMoreImageData,
    /// Bad text encoding
    BadTextEncoding(TextDecodingError),
    /// fdAT shorter than 4 bytes
    FdatShorterThanFourBytes,
    /// "11.2.4 IDAT Image data" section of the PNG spec says: There may be multiple IDAT chunks;
    /// if so, they shall appear consecutively with no other intervening chunks.
    /// `UnexpectedRestartOfDataChunkSequence{kind: IDAT}` indicates that there were "intervening
    /// chunks".
    ///
    /// The APNG spec doesn't directly describe an error similar to `CantInterleaveIdatChunks`,
    /// but we require that a new sequence of consecutive `fdAT` chunks cannot appear unless we've
    /// seen an `fcTL` chunk.
    UnexpectedRestartOfDataChunkSequence {
        kind: ChunkType,
    },
    /// Failure to parse a chunk, because the chunk had the wrong number of bytes.
    ChunkLengthWrong {
        kind: ChunkType,
    },
    UnrecognizedCriticalChunk {
        /// The type of the unrecognized critical chunk.
        type_str: ChunkType,
    },
    BadGammaValue,
}

mod display;
