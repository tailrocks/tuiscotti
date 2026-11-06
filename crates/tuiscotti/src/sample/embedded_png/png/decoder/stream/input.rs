use super::*;

impl StreamingDecoder {
    /// Creates a new StreamingDecoder
    ///
    /// Allocates the internal buffers.
    pub(crate) fn new() -> StreamingDecoder {
        StreamingDecoder::new_with_options(DecodeOptions::default())
    }

    pub(crate) fn new_with_options(decode_options: DecodeOptions) -> StreamingDecoder {
        let mut inflater = ZlibStream::new();
        inflater.set_ignore_adler32(decode_options.ignore_adler32);

        StreamingDecoder {
            state: Some(State::new_u32(U32ValueKind::Signature1stU32)),
            current_chunk: ChunkState {
                type_: ChunkType([0; 4]),
                crc: Crc32::new(),
                remaining: 0,
                // Start empty so bounded decoding can report allocator failure at
                // the first checked growth instead of panicking here.
                raw_bytes: Vec::new(),
                action: ChunkAction::Process,
            },
            inflater,
            info: None,
            current_seq_no: None,
            have_idat: false,
            have_iccp: false,
            ready_for_idat_chunks: true,
            ready_for_fdat_chunks: false,
            decode_options,
            limits: Limits { bytes: usize::MAX },
        }
    }

    /// Resets the StreamingDecoder
    pub(crate) fn reset(&mut self) {
        self.state = Some(State::new_u32(U32ValueKind::Signature1stU32));
        self.current_chunk.crc = Crc32::new();
        self.current_chunk.remaining = 0;
        self.current_chunk.raw_bytes.clear();
        self.inflater.reset();
        self.info = None;
        self.current_seq_no = None;
        self.have_idat = false;
    }

    /// Provides access to the inner `info` field
    pub(crate) fn info(&self) -> Option<&Info<'static>> {
        self.info.as_ref()
    }

    pub(crate) fn set_ignore_text_chunk(&mut self, ignore_text_chunk: bool) {
        self.decode_options.set_ignore_text_chunk(ignore_text_chunk);
    }

    pub(crate) fn set_ignore_iccp_chunk(&mut self, ignore_iccp_chunk: bool) {
        self.decode_options.set_ignore_iccp_chunk(ignore_iccp_chunk);
    }

    /// Return whether the decoder is set to ignore the Adler-32 checksum.
    pub(crate) fn ignore_adler32(&self) -> bool {
        self.inflater.ignore_adler32()
    }

    /// Set whether to compute and verify the Adler-32 checksum during
    /// decompression. Return `true` if the flag was successfully set.
    ///
    /// The decoder defaults to `true`.
    ///
    /// This flag cannot be modified after decompression has started until the
    /// [`StreamingDecoder`] is reset.
    pub(crate) fn set_ignore_adler32(&mut self, ignore_adler32: bool) -> bool {
        self.inflater.set_ignore_adler32(ignore_adler32)
    }

    /// Set whether to compute and verify the Adler-32 checksum during
    /// decompression.
    ///
    /// The decoder defaults to `false`.
    pub(crate) fn set_ignore_crc(&mut self, ignore_crc: bool) {
        self.decode_options.set_ignore_crc(ignore_crc)
    }

    /// Ignore ancillary chunks if CRC fails
    ///
    /// Defaults to `true`
    pub(crate) fn set_skip_ancillary_crc_failures(&mut self, skip_ancillary_crc_failures: bool) {
        self.decode_options
            .set_skip_ancillary_crc_failures(skip_ancillary_crc_failures)
    }
}
