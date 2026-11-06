use borrow::Cow;
use io::{Read, Write};
use ops::{Deref, DerefMut};
use std::{borrow, error, fmt, io, mem, ops, result};

use crc32fast::Hasher as Crc32;
use flate2::write::ZlibEncoder;

use crate::sample::embedded_png::png::DeflateCompression;

mod chunks;
mod config;
mod stream_writer;
mod writer;
use chunks::{ChunkOutput, ChunkWriter, Wrapper};
pub(crate) use config::Encoder;
use config::Options;
pub(crate) use stream_writer::StreamWriter;
use writer::PartialInfo;
pub(crate) use writer::{Writer, write_chunk};
#[cfg(test)]
#[path = "encoder_tests.rs"]
mod tests;
use crate::sample::embedded_png::png::chunk::{self, ChunkType};
use crate::sample::embedded_png::png::common::{
    AnimationControl, BitDepth, BlendOp, BytesPerPixel, ColorType, Compression, DisposeOp,
    FrameControl, Info, ParameterError, ParameterErrorKind, PixelDimensions, ScaledFloat, Unit,
};
use crate::sample::embedded_png::png::filter::{Filter, filter};
use crate::sample::embedded_png::png::text_metadata::{
    EncodableTextChunk, ITXtChunk, TEXtChunk, TextEncodingError, ZTXtChunk, encode_iso_8859_1,
};
use crate::sample::embedded_png::png::traits::WriteBytesExt;

pub(crate) type Result<T> = result::Result<T, EncodingError>;

#[derive(Debug)]
pub(crate) enum EncodingError {
    IoError(io::Error),
    Format(FormatError),
    Parameter(ParameterError),
    LimitsExceeded,
}

#[derive(Debug)]
pub(crate) struct FormatError {
    inner: FormatErrorKind,
}

#[derive(Debug)]
enum FormatErrorKind {
    ZeroWidth,
    ZeroHeight,
    InvalidColorCombination(BitDepth, ColorType),
    NoPalette,
    // TODO: wait, what?
    WrittenTooMuch(usize),
    NotAnimated,
    OutOfBounds,
    EndReached,
    ZeroFrames,
    MissingFrames,
    MissingData(usize),
    Unrecoverable,
    BadTextEncoding(TextEncodingError),
}

impl error::Error for EncodingError {
    fn cause(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            EncodingError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for EncodingError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        use self::EncodingError::*;
        match self {
            IoError(err) => write!(fmt, "{}", err),
            Format(desc) => write!(fmt, "{}", desc),
            Parameter(desc) => write!(fmt, "{}", desc),
            LimitsExceeded => write!(fmt, "Limits are exceeded."),
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        use FormatErrorKind::*;
        match self.inner {
            ZeroWidth => write!(fmt, "Zero width not allowed"),
            ZeroHeight => write!(fmt, "Zero height not allowed"),
            ZeroFrames => write!(fmt, "Zero frames not allowed"),
            InvalidColorCombination(depth, color) => write!(
                fmt,
                "Invalid combination of bit-depth '{:?}' and color-type '{:?}'",
                depth, color
            ),
            NoPalette => write!(fmt, "can't write indexed image without palette"),
            WrittenTooMuch(index) => write!(fmt, "wrong data size, got {} bytes too many", index),
            NotAnimated => write!(fmt, "not an animation"),
            OutOfBounds => write!(
                fmt,
                "the dimension and position go over the frame boundaries"
            ),
            EndReached => write!(fmt, "all the frames have been already written"),
            MissingFrames => write!(fmt, "there are still frames to be written"),
            MissingData(n) => write!(fmt, "there are still {} bytes to be written", n),
            Unrecoverable => write!(
                fmt,
                "a previous error put the writer into an unrecoverable state"
            ),
            BadTextEncoding(tee) => match tee {
                TextEncodingError::Unrepresentable => write!(
                    fmt,
                    "The text metadata cannot be encoded into valid ISO 8859-1"
                ),
                TextEncodingError::InvalidKeywordSize => write!(fmt, "Invalid keyword size"),
                TextEncodingError::CompressionError => {
                    write!(fmt, "Unable to compress text metadata")
                }
            },
        }
    }
}

impl From<io::Error> for EncodingError {
    fn from(err: io::Error) -> EncodingError {
        EncodingError::IoError(err)
    }
}

impl From<EncodingError> for io::Error {
    fn from(err: EncodingError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}

// Private impl.
impl From<FormatErrorKind> for FormatError {
    fn from(kind: FormatErrorKind) -> Self {
        FormatError { inner: kind }
    }
}

impl From<TextEncodingError> for EncodingError {
    fn from(tee: TextEncodingError) -> Self {
        EncodingError::Format(FormatError {
            inner: FormatErrorKind::BadTextEncoding(tee),
        })
    }
}
