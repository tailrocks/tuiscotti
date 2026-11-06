use std::borrow::Cow;
use std::cmp::min;
use std::collections::TryReserveError;
use std::error;
use std::fmt;
use std::io;

use crc32fast::Hasher as Crc32;

use super::zlib::{UnfilterBuf, ZlibStream};
use crate::sample::embedded_png::png::chunk::is_critical;
use crate::sample::embedded_png::png::chunk::{self, ChunkType, IDAT, IEND, IHDR};
use crate::sample::embedded_png::png::common::{
    AnimationControl, BitDepth, BlendOp, ColorType, ContentLightLevelInfo, DisposeOp, FrameControl,
    Info, MasteringDisplayColorVolume, ParameterError, ParameterErrorKind, PixelDimensions,
    ScaledFloat, SourceChromaticities, Unit,
};
use crate::sample::embedded_png::png::text_metadata::{
    ITXtChunk, TEXtChunk, TextDecodingError, ZTXtChunk,
};
use crate::sample::embedded_png::png::traits::ReadBytesExt;
use crate::sample::embedded_png::png::{CodingIndependentCodePoints, Limits};

mod chunk_ops;
mod color;
mod errors;
mod frame;
mod framing;
mod input;
mod metadata;
mod options;
mod palette;
mod properties;
mod state;
mod validate;

#[cfg(test)]
mod tests;

pub(super) use errors::FormatError;
pub(crate) use errors::{Decoded, DecodingError, FormatErrorInner};
pub(crate) use options::DecodeOptions;
use state::{State, U32ValueKind};

pub(crate) const CHUNK_BUFFER_SIZE: usize = 128;

/// Determines if checksum checks should be disabled globally.
///
/// This is used only in fuzzing. `afl` automatically adds `--cfg fuzzing` to RUSTFLAGS which can
/// be used to detect that build.
const CHECKSUM_DISABLED: bool = cfg!(fuzzing);

/// PNG StreamingDecoder (low-level interface)
///
/// By default, the decoder does not verify Adler-32 checksum computation. To
/// enable checksum verification, set it with [`StreamingDecoder::set_ignore_adler32`]
/// before starting decompression.
pub(crate) struct StreamingDecoder {
    state: Option<State>,
    current_chunk: ChunkState,
    pub(super) inflater: ZlibStream,
    pub(crate) info: Option<Info<'static>>,
    pub(crate) limits: Limits,
    pub(super) ready_for_idat_chunks: bool,
    pub(super) ready_for_fdat_chunks: bool,
    pub(super) have_idat: bool,
    pub(super) current_seq_no: Option<u32>,
    pub(super) decode_options: DecodeOptions,
    pub(super) have_iccp: bool,
}

fn info_ref<'a>(info: &'a Option<Info<'static>>) -> Result<&'a Info<'static>, DecodingError> {
    info.as_ref().ok_or(DecodingError::InvalidState)
}

fn info_mut<'a>(
    info: &'a mut Option<Info<'static>>,
) -> Result<&'a mut Info<'static>, DecodingError> {
    info.as_mut().ok_or(DecodingError::InvalidState)
}

struct ChunkState {
    crc: Crc32,
    remaining: u32,
    raw_bytes: Vec<u8>,
    type_: ChunkType,
    action: ChunkAction,
}

#[derive(Debug, PartialEq, Eq)]
enum ChunkAction {
    Process,
    Skip,
    Reject,
}

impl Default for StreamingDecoder {
    fn default() -> Self {
        Self::new()
    }
}
