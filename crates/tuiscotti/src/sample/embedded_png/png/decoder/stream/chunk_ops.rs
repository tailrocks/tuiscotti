use super::FormatError;
use super::*;

impl StreamingDecoder {
    pub(super) fn copy_current_chunk_raw_bytes(
        &mut self,
        site: &'static str,
    ) -> Result<Vec<u8>, DecodingError> {
        let raw_bytes = &self.current_chunk.raw_bytes;
        let requested = raw_bytes.len();
        self.limits.reserve_bytes(requested)?;

        let mut copy = Vec::new();
        copy.try_reserve_exact(requested)
            .map_err(|source| DecodingError::allocation(site, requested, source))?;
        copy.extend_from_slice(raw_bytes);
        Ok(copy)
    }

    pub(super) fn start_chunk(
        &mut self,
        type_str: ChunkType,
        length: u32,
    ) -> Result<State, DecodingError> {
        let target_length = match type_str {
            IHDR => 13..=13,
            chunk::PLTE => 3..=768,
            chunk::IEND => 0..=0,
            chunk::S_BIT => 1..=4,
            chunk::T_RNS => 1..=256,
            chunk::P_HYS => 9..=9,
            chunk::G_AMA => 4..=4,
            chunk::AC_TL => 8..=8,
            chunk::FC_TL => 26..=26,
            chunk::C_HRM => 32..=32,
            chunk::S_RGB => 1..=1,
            chunk::C_ICP => 4..=4,
            chunk::M_DCV => 24..=24,
            chunk::C_LLI => 8..=8,
            chunk::B_KGD => 1..=6,

            // Unbounded size chunks
            chunk::E_XIF => 0..=u32::MAX >> 1, // TODO: allow skipping.
            chunk::I_CCP if !self.decode_options.ignore_iccp_chunk => 0..=u32::MAX >> 1,
            chunk::T_EXT if !self.decode_options.ignore_text_chunk => 0..=u32::MAX >> 1,
            chunk::Z_TXT if !self.decode_options.ignore_text_chunk => 0..=u32::MAX >> 1,
            chunk::I_TXT if !self.decode_options.ignore_text_chunk => 0..=u32::MAX >> 1,

            chunk::IDAT | chunk::FD_AT => unreachable!(),

            _ if is_critical(type_str) => {
                return Err(DecodingError::Format(
                    FormatErrorInner::UnrecognizedCriticalChunk { type_str }.into(),
                ));
            }
            _ => {
                self.current_chunk.action = ChunkAction::Skip;
                return Ok(State::ReadChunkData(type_str));
            }
        };

        if !target_length.contains(&length) {
            // Uncomment to detect unexpected chunk lengths during testing.
            // panic!("chunk type_str={type_str:?} has length={length}, target_length={target_length:?}");
            match type_str {
                IHDR | chunk::PLTE | chunk::IEND | chunk::FC_TL => {
                    return Err(DecodingError::Format(
                        FormatErrorInner::ChunkLengthWrong { kind: type_str }.into(),
                    ));
                }
                _ => {
                    self.current_chunk.action = ChunkAction::Reject;
                }
            }
        } else {
            self.current_chunk.action = ChunkAction::Process;
        }

        Ok(State::ReadChunkData(type_str))
    }

    pub(super) fn parse_chunk(&mut self, type_str: ChunkType) -> Result<Decoded, DecodingError> {
        let mut parse_result = match type_str {
            // Critical non-data chunks.
            IHDR => self.parse_ihdr(),
            chunk::PLTE => self.parse_plte(),
            chunk::IEND => Ok(()), // TODO: Check chunk size.

            // Data chunks handled separately.
            chunk::IDAT => Ok(()),
            chunk::FD_AT => Ok(()),

            // Recognized bounded-size ancillary chunks.
            chunk::S_BIT => self.parse_sbit(),
            chunk::T_RNS => self.parse_trns(),
            chunk::P_HYS => self.parse_phys(),
            chunk::G_AMA => self.parse_gama(),
            chunk::AC_TL => self.parse_actl(),
            chunk::FC_TL => self.parse_fctl(),
            chunk::C_HRM => self.parse_chrm(),
            chunk::S_RGB => self.parse_srgb(),
            chunk::C_ICP => self.parse_cicp(),
            chunk::M_DCV => self.parse_mdcv(),
            chunk::C_LLI => self.parse_clli(),
            chunk::B_KGD => self.parse_bkgd(),

            // Ancillary chunks with unbounded size.
            chunk::E_XIF => self.parse_exif(),
            chunk::I_CCP => self.parse_iccp(),
            chunk::T_EXT => self.parse_text(),
            chunk::Z_TXT => self.parse_ztxt(),
            chunk::I_TXT => self.parse_itxt(),

            // Unrecognized chunks.
            _ => unreachable!(
                "Unrecognized chunk {type_str:?} should have been caught in start_chunk"
            ),
        };

        parse_result = parse_result.map_err(|e| {
            match e {
                // `parse_chunk` is invoked after gathering **all** bytes of a chunk, so
                // `UnexpectedEof` from something like `read_be` is permanent and indicates an
                // invalid PNG that should be represented as a `FormatError`, rather than as a
                // (potentially recoverable) `IoError` / `UnexpectedEof`.
                DecodingError::IoError(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    let fmt_err: FormatError =
                        FormatErrorInner::ChunkLengthWrong { kind: type_str }.into();
                    fmt_err.into()
                }
                e => e,
            }
        });

        match parse_result {
            Ok(()) => Ok(Decoded::ChunkComplete(type_str)),
            Err(DecodingError::Format(_))
                if type_str != chunk::FC_TL && !chunk::is_critical(type_str) =>
            {
                // Ignore benign errors in most auxiliary chunks. `LimitsExceeded`, `Parameter` and
                // other error kinds are *not* treated as benign. We don't ignore errors in `fcTL`
                // chunks because the fallback to the static/non-animated image has to be
                // implemented *on top* of the `StreamingDecoder` API.
                //
                // TODO: Consider supporting a strict mode where even benign errors are reported up.
                // See https://github.com/image-rs/image-png/pull/569#issuecomment-2642062285
                Ok(Decoded::BadAncillaryChunk(type_str))
            }
            Err(e) => Err(e),
        }
    }
}
