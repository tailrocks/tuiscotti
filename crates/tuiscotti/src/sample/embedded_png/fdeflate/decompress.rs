use simd_adler32::Adler32;

use crate::sample::embedded_png::fdeflate::{
    huffman::{self, build_table},
    tables::{
        self, CLCL_ORDER, DIST_SYM_TO_DIST_BASE, DIST_SYM_TO_DIST_EXTRA, FIXED_DIST_TABLE,
        FIXED_LITLEN_TABLE, LEN_SYM_TO_LEN_BASE, LEN_SYM_TO_LEN_EXTRA, LITLEN_TABLE_ENTRIES,
    },
};

fn table_result(
    result: Result<bool, huffman::TableAllocationError>,
    allocation_error: &mut Option<(usize, std::collections::TryReserveError)>,
) -> Result<bool, DecompressionError> {
    match result {
        Ok(valid) => Ok(valid),
        Err(error) => {
            *allocation_error = Some((error.requested, error.source));
            Err(DecompressionError::Allocation)
        }
    }
}

/// An error encountered while decompressing a deflate stream.
#[derive(Debug, PartialEq)]
pub(crate) enum DecompressionError {
    /// The zlib header is corrupt.
    BadZlibHeader,
    /// All input was consumed, but the end of the stream hasn't been reached.
    InsufficientInput,
    /// A block header specifies an invalid block type.
    InvalidBlockType,
    /// An uncompressed block's NLEN value is invalid.
    InvalidUncompressedBlockLength,
    /// Too many literals were specified.
    InvalidHlit,
    /// Too many distance codes were specified.
    InvalidHdist,
    /// Attempted to repeat a previous code before reading any codes, or past the end of the code
    /// lengths.
    InvalidCodeLengthRepeat,
    /// The stream doesn't specify a valid huffman tree.
    BadCodeLengthHuffmanTree,
    /// The stream doesn't specify a valid huffman tree.
    BadLiteralLengthHuffmanTree,
    /// The stream doesn't specify a valid huffman tree.
    BadDistanceHuffmanTree,
    /// The stream contains a literal/length code that was not allowed by the header.
    InvalidLiteralLengthCode,
    /// The stream contains a distance code that was not allowed by the header.
    InvalidDistanceCode,
    /// The stream contains contains back-reference as the first symbol.
    InputStartsWithRun,
    /// The stream contains a back-reference that is too far back.
    DistanceTooFarBack,
    /// The deflate stream checksum is incorrect.
    WrongChecksum,
    /// Extra input data.
    ExtraInput,
    /// A dynamic Huffman table could not reserve its secondary storage.
    Allocation,
}

struct BlockHeader {
    hlit: usize,
    hdist: usize,
    hclen: usize,
    num_lengths_read: usize,

    /// Low 3-bits are code length code length, high 5-bits are code length code.
    table: [u32; 128],
    code_lengths: [u8; 320],
}

pub(crate) const LITERAL_ENTRY: u32 = 0x8000;
pub(crate) const EXCEPTIONAL_ENTRY: u32 = 0x4000;
pub(crate) const SECONDARY_TABLE_ENTRY: u32 = 0x2000;

/// The Decompressor state for a compressed block.
#[derive(Eq, PartialEq, Debug)]
struct CompressedBlock {
    litlen_table: [u32; 4096],
    secondary_table: Vec<u16>,

    dist_table: [u32; 512],
    dist_secondary_table: Vec<u16>,

    eof_code: u16,
    eof_mask: u16,
    eof_bits: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum State {
    ZlibHeader,
    BlockHeader,
    CodeLengthCodes,
    CodeLengths,
    CompressedData,
    UncompressedData,
    Checksum,
    Done,
}

/// Decompressor for arbitrary zlib streams.
pub(crate) struct Decompressor {
    /// State for decoding a compressed block.
    compression: CompressedBlock,
    // State for decoding a block header.
    header: BlockHeader,
    // Number of bytes left for uncompressed block.
    uncompressed_bytes_left: u16,

    buffer: u64,
    nbits: u8,

    queued_rle: Option<(u8, usize)>,
    queued_backref: Option<(usize, usize)>,
    last_block: bool,
    fixed_table: bool,

    state: State,
    checksum: Adler32,
    ignore_adler32: bool,
    allocation_error: Option<(usize, std::collections::TryReserveError)>,
}

mod careful;
mod compressed;
mod header;
mod read;
#[cfg(test)]
#[path = "decompress_tests.rs"]
mod tests;

/// Decompress the given data.
pub(crate) fn decompress_to_vec(input: &[u8]) -> Result<Vec<u8>, DecompressionError> {
    match decompress_to_vec_bounded_unmetered(input, usize::MAX) {
        Ok(output) => Ok(output),
        Err(BoundedDecompressionError::DecompressionError { inner }) => Err(inner),
        Err(BoundedDecompressionError::OutputTooLarge { .. }) => {
            unreachable!("Impossible to allocate more than isize::MAX bytes")
        }
        Err(BoundedDecompressionError::LimitsExceeded) => unreachable!("unmetered decoder"),
        Err(BoundedDecompressionError::Allocation { .. }) => Err(DecompressionError::Allocation),
    }
}

/// An error encountered while decompressing a deflate stream given a bounded maximum output.
#[derive(Debug)]
pub(crate) enum BoundedDecompressionError {
    /// The input is not a valid deflate stream.
    DecompressionError {
        /// The underlying error.
        inner: DecompressionError,
    },

    /// The output is too large.
    OutputTooLarge {
        /// The output decoded so far.
        partial_output: Vec<u8>,
    },

    /// The caller's byte budget could not cover the next output allocation.
    LimitsExceeded,

    /// A checked output or Huffman table allocation failed.
    Allocation {
        /// The allocation site.
        site: &'static str,
        /// The additional number of requested bytes.
        requested: usize,
        /// The allocator's reserve error.
        source: std::collections::TryReserveError,
    },
}
impl From<DecompressionError> for BoundedDecompressionError {
    fn from(inner: DecompressionError) -> Self {
        BoundedDecompressionError::DecompressionError { inner }
    }
}

/// Decompress the given data, returning an error if the output is larger than
/// `maxlen` bytes.
pub(crate) fn decompress_to_vec_bounded(
    input: &[u8],
    maxlen: usize,
    reserve_bytes: impl FnMut(usize) -> bool,
) -> Result<Vec<u8>, BoundedDecompressionError> {
    decompress_to_vec_bounded_with_reservation(input, maxlen, reserve_bytes)
}

/// Bounded decompression adapter for text accessors that run after PNG metadata parsing and have
/// no decoder `Limits` counter to debit. Sample validation does not call these accessors.
pub(crate) fn decompress_to_vec_bounded_unmetered(
    input: &[u8],
    maxlen: usize,
) -> Result<Vec<u8>, BoundedDecompressionError> {
    decompress_to_vec_bounded_with_reservation(input, maxlen, |_| true)
}

fn decompress_to_vec_bounded_with_reservation(
    input: &[u8],
    maxlen: usize,
    mut reserve_bytes: impl FnMut(usize) -> bool,
) -> Result<Vec<u8>, BoundedDecompressionError> {
    let mut decoder = Decompressor::new();
    let mut output = Vec::new();
    reserve_output(&mut output, 1024.min(maxlen), &mut reserve_bytes)?;
    let mut input_index = 0;
    let mut output_index = 0;
    loop {
        let (consumed, produced) =
            match decoder.read(&input[input_index..], &mut output, output_index, true) {
                Ok(read) => read,
                Err(DecompressionError::Allocation) => {
                    if let Some((requested, source)) = decoder.take_allocation_error() {
                        return Err(BoundedDecompressionError::Allocation {
                            site: "fdeflate Huffman table",
                            requested,
                            source,
                        });
                    }
                    return Err(DecompressionError::Allocation.into());
                }
                Err(error) => return Err(error.into()),
            };
        input_index += consumed;
        output_index += produced;
        if decoder.is_done() || output_index == maxlen {
            break;
        }
        let next_len = output_index.saturating_add(32 * 1024).min(maxlen);
        reserve_output(&mut output, next_len, &mut reserve_bytes)?;
    }
    output.resize(output_index, 0);

    if decoder.is_done() {
        Ok(output)
    } else {
        Err(BoundedDecompressionError::OutputTooLarge {
            partial_output: output,
        })
    }
}

fn reserve_output(
    output: &mut Vec<u8>,
    new_len: usize,
    reserve_bytes: &mut impl FnMut(usize) -> bool,
) -> Result<(), BoundedDecompressionError> {
    let additional = new_len.saturating_sub(output.len());
    if !reserve_bytes(additional) {
        return Err(BoundedDecompressionError::LimitsExceeded);
    }
    output.try_reserve_exact(additional).map_err(|source| {
        BoundedDecompressionError::Allocation {
            site: "fdeflate bounded output",
            requested: additional,
            source,
        }
    })?;
    output.resize(new_len, 0);
    Ok(())
}
