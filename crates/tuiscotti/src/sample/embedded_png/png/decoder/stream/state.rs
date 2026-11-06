use crate::sample::embedded_png::png::chunk::ChunkType;

/// Kind of `u32` value that is being read via `State::U32`.
#[derive(Debug)]
pub(super) enum U32ValueKind {
    /// First 4 bytes of the PNG signature - see
    /// http://www.libpng.org/pub(crate)/png/spec/1.2/PNG-Structure.html#PNG-file-signature
    Signature1stU32,
    /// Second 4 bytes of the PNG signature - see
    /// http://www.libpng.org/pub(crate)/png/spec/1.2/PNG-Structure.html#PNG-file-signature
    Signature2ndU32,
    /// Chunk length - see
    /// http://www.libpng.org/pub(crate)/png/spec/1.2/PNG-Structure.html#Chunk-layout
    Length,
    /// Chunk type - see
    /// http://www.libpng.org/pub(crate)/png/spec/1.2/PNG-Structure.html#Chunk-layout
    Type { length: u32 },
    /// Chunk checksum - see
    /// http://www.libpng.org/pub(crate)/png/spec/1.2/PNG-Structure.html#Chunk-layout
    Crc(ChunkType),
    /// Sequence number from an `fdAT` chunk - see
    /// https://wiki.mozilla.org/APNG_Specification#.60fdAT.60:_The_Frame_Data_Chunk
    ApngSequenceNumber,
}

#[derive(Debug)]
pub(super) enum State {
    /// In this state we are reading a u32 value from external input.  We start with
    /// `accumulated_count` set to `0`. After reading or accumulating the required 4 bytes we will
    /// call `parse_32` which will then move onto the next state.
    U32 {
        kind: U32ValueKind,
        bytes: [u8; 4],
        accumulated_count: usize,
    },
    /// In this state we are reading chunk data from external input, and appending it to
    /// `ChunkState::raw_bytes`. Then if all data has been read, we parse the chunk.
    ReadChunkData(ChunkType),
    /// In this state we are reading image data from external input and feeding it directly into
    /// `StreamingDecoder::inflater`.
    ImageData(ChunkType),
}

impl State {
    pub(super) fn new_u32(kind: U32ValueKind) -> Self {
        Self::U32 {
            kind,
            bytes: [0; 4],
            accumulated_count: 0,
        }
    }
}
