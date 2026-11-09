use super::{DecodingError, FormatError, FormatErrorInner};
use crate::sample::embedded_png::png::text_metadata::TextDecodingError;
use std::{error, fmt, io};

impl error::Error for DecodingError {
    fn cause(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            DecodingError::IoError(err) => Some(err),
            DecodingError::Allocation { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl fmt::Display for DecodingError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::DecodingError::*;
        match self {
            IoError(err) => write!(fmt, "{}", err),
            Parameter(desc) => write!(fmt, "{}", &desc),
            Format(desc) => write!(fmt, "{}", desc),
            LimitsExceeded => write!(fmt, "limits are exceeded"),
            Allocation {
                site,
                requested,
                source,
            } => write!(
                fmt,
                "allocation failed at {site} for {requested} bytes: {source}"
            ),
            InvalidState => write!(fmt, "invalid PNG decoder state"),
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use FormatErrorInner::*;
        match &self.inner {
            CrcMismatch {
                crc_val,
                crc_sum,
                chunk,
                ..
            } => write!(
                fmt,
                "CRC error: expected 0x{:x} have 0x{:x} while decoding {:?} chunk.",
                crc_val, crc_sum, chunk
            ),
            MissingFctl => write!(fmt, "fcTL chunk missing before fdAT chunk."),
            MissingImageData => write!(fmt, "IDAT or fdAT chunk is missing."),
            ChunkBeforeIhdr { kind } => write!(fmt, "{:?} chunk appeared before IHDR chunk", kind),
            AfterIdat { kind } => write!(fmt, "Chunk {:?} is invalid after IDAT chunk.", kind),
            BeforePlte { kind } => write!(fmt, "Chunk {:?} is invalid before PLTE chunk.", kind),
            AfterPlte { kind } => write!(fmt, "Chunk {:?} is invalid after PLTE chunk.", kind),
            OutsidePlteIdat { kind } => write!(
                fmt,
                "Chunk {:?} must appear between PLTE and IDAT chunks.",
                kind
            ),
            DuplicateChunk { kind } => write!(fmt, "Chunk {:?} must appear at most once.", kind),
            ApngOrder { present, expected } => write!(
                fmt,
                "Sequence is not in order, expected #{} got #{}.",
                expected, present,
            ),
            ShortPalette { expected, len } => write!(
                fmt,
                "Not enough palette entries, expect {} got {}.",
                expected, len
            ),
            InvalidSbitChunkSize {
                color_type,
                expected,
                len,
            } => write!(
                fmt,
                "The size of the sBIT chunk should be {} byte(s), but {} byte(s) were provided for the {:?} color type.",
                expected, len, color_type
            ),
            InvalidSbit { sample_depth, sbit } => write!(
                fmt,
                "Invalid sBIT value {}. It must be greater than zero and less than the sample depth {:?}.",
                sbit, sample_depth
            ),
            PaletteRequired => write!(fmt, "Missing palette of indexed image."),
            InvalidDimensions => write!(fmt, "Invalid image dimensions"),
            InvalidColorBitDepth {
                color_type,
                bit_depth,
            } => write!(
                fmt,
                "Invalid color/depth combination in header: {:?}/{:?}",
                color_type, bit_depth,
            ),
            ColorWithBadTrns(color_type) => write!(
                fmt,
                "Transparency chunk found for color type {:?}.",
                color_type
            ),
            InvalidBitDepth(nr) => write!(fmt, "Invalid bit depth {}.", nr),
            InvalidColorType(nr) => write!(fmt, "Invalid color type {}.", nr),
            InvalidDisposeOp(nr) => write!(fmt, "Invalid dispose op {}.", nr),
            InvalidBlendOp(nr) => write!(fmt, "Invalid blend op {}.", nr),
            InvalidUnit(nr) => write!(fmt, "Invalid physical pixel size unit {}.", nr),
            InvalidSrgbRenderingIntent(nr) => write!(fmt, "Invalid sRGB rendering intent {}.", nr),
            UnknownCompressionMethod(nr) => write!(fmt, "Unknown compression method {}.", nr),
            UnknownFilterMethod(nr) => write!(fmt, "Unknown filter method {}.", nr),
            UnknownInterlaceMethod(nr) => write!(fmt, "Unknown interlace method {}.", nr),
            BadSubFrameBounds {} => write!(fmt, "Sub frame is out-of-bounds."),
            InvalidSignature => write!(fmt, "Invalid PNG signature."),
            NoMoreImageData => write!(
                fmt,
                "IDAT or fDAT chunk does not have enough data for image."
            ),
            CorruptFlateStream { err } => {
                write!(fmt, "Corrupt deflate stream. ")?;
                write!(fmt, "{:?}", err)
            }
            // TODO: Wrap more info in the enum variant
            BadTextEncoding(tde) => match tde {
                TextDecodingError::Unrepresentable => {
                    write!(fmt, "Unrepresentable data in tEXt chunk.")
                }
                TextDecodingError::InvalidKeywordSize => {
                    write!(fmt, "Keyword empty or longer than 79 bytes.")
                }
                TextDecodingError::MissingNullSeparator => {
                    write!(fmt, "No null separator in tEXt chunk.")
                }
                TextDecodingError::InflationError => {
                    write!(fmt, "Invalid compressed text data.")
                }
                TextDecodingError::OutOfDecompressionSpace => {
                    write!(fmt, "Out of decompression space. Try with a larger limit.")
                }
                TextDecodingError::InvalidCompressionMethod => {
                    write!(fmt, "Using an unrecognized byte as compression method.")
                }
                TextDecodingError::InvalidCompressionFlag => {
                    write!(
                        fmt,
                        "Using a flag that is not 0 or 255 as a compression flag for iTXt chunk."
                    )
                }
                TextDecodingError::MissingCompressionFlag => {
                    write!(fmt, "No compression flag in the iTXt chunk.")
                }
            },
            FdatShorterThanFourBytes => write!(fmt, "fdAT chunk shorter than 4 bytes"),
            UnexpectedRestartOfDataChunkSequence { kind } => {
                write!(fmt, "Unexpected restart of {:?} chunk sequence", kind)
            }
            ChunkLengthWrong { kind } => {
                write!(fmt, "Chunk length wrong: {:?}", kind)
            }
            UnrecognizedCriticalChunk { type_str } => {
                write!(fmt, "Unrecognized critical chunk: {:?}", type_str)
            }
            BadGammaValue => write!(fmt, "Bad gamma value."),
        }
    }
}

impl From<io::Error> for DecodingError {
    fn from(err: io::Error) -> DecodingError {
        DecodingError::IoError(err)
    }
}

impl From<FormatError> for DecodingError {
    fn from(err: FormatError) -> DecodingError {
        DecodingError::Format(err)
    }
}

impl From<FormatErrorInner> for FormatError {
    fn from(inner: FormatErrorInner) -> Self {
        FormatError { inner }
    }
}

impl From<DecodingError> for io::Error {
    fn from(err: DecodingError) -> io::Error {
        match err {
            DecodingError::IoError(err) => err,
            err => io::Error::new(io::ErrorKind::Other, err.to_string()),
        }
    }
}

impl From<TextDecodingError> for DecodingError {
    fn from(tbe: TextDecodingError) -> Self {
        DecodingError::Format(FormatError {
            inner: FormatErrorInner::BadTextEncoding(tbe),
        })
    }
}
