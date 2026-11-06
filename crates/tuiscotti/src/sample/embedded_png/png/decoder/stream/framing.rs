use super::*;

impl StreamingDecoder {
    pub(crate) fn update(
        &mut self,
        mut buf: &[u8],
        mut image_data: Option<&mut UnfilterBuf<'_>>,
    ) -> Result<(usize, Decoded), DecodingError> {
        if self.state.is_none() {
            return Err(DecodingError::Parameter(
                ParameterErrorKind::PolledAfterFatalError.into(),
            ));
        }

        let len = buf.len();
        while !buf.is_empty() {
            let image_data = image_data.as_deref_mut();

            match self.next_state(buf, image_data) {
                Ok((bytes, Decoded::Nothing)) => buf = &buf[bytes..],
                Ok((bytes, result)) => {
                    buf = &buf[bytes..];
                    return Ok((len - buf.len(), result));
                }
                Err(err) => {
                    debug_assert!(self.state.is_none());
                    return Err(err);
                }
            }
        }
        Ok((len - buf.len(), Decoded::Nothing))
    }

    fn next_state(
        &mut self,
        buf: &[u8],
        image_data: Option<&mut UnfilterBuf<'_>>,
    ) -> Result<(usize, Decoded), DecodingError> {
        use super::State::*;

        // Driver should ensure that state is never None
        let state = self.state.take().ok_or(DecodingError::InvalidState)?;

        match state {
            U32 {
                kind,
                mut bytes,
                mut accumulated_count,
            } => {
                debug_assert!(accumulated_count <= 4);
                if accumulated_count == 0 && buf.len() >= 4 {
                    // Handling these `accumulated_count` and `buf.len()` values in a separate `if`
                    // branch is not strictly necessary - the `else` statement below is already
                    // capable of handling these values.  The main reason for special-casing these
                    // values is that they occur fairly frequently and special-casing them results
                    // in performance gains.
                    const CONSUMED_BYTES: usize = 4;
                    self.parse_u32(kind, &buf[0..4], image_data, CONSUMED_BYTES)
                } else {
                    let remaining_count = 4 - accumulated_count;
                    let consumed_bytes = {
                        let available_count = min(remaining_count, buf.len());
                        bytes[accumulated_count..accumulated_count + available_count]
                            .copy_from_slice(&buf[0..available_count]);
                        accumulated_count += available_count;
                        available_count
                    };

                    if accumulated_count < 4 {
                        self.state = Some(U32 {
                            kind,
                            bytes,
                            accumulated_count,
                        });
                        Ok((consumed_bytes, Decoded::Nothing))
                    } else {
                        debug_assert_eq!(accumulated_count, 4);
                        self.parse_u32(kind, &bytes, image_data, consumed_bytes)
                    }
                }
            }
            ReadChunkData(type_str) => {
                debug_assert!(type_str != IDAT && type_str != chunk::FD_AT);
                if self.current_chunk.remaining == 0 {
                    self.state = Some(State::new_u32(U32ValueKind::Crc(type_str)));
                    Ok((0, Decoded::Nothing))
                } else {
                    let ChunkState {
                        crc,
                        remaining,
                        raw_bytes,
                        type_: _,
                        action,
                    } = &mut self.current_chunk;

                    let buf_avail = raw_bytes.capacity() - raw_bytes.len();
                    let bytes_avail = min(buf.len(), buf_avail);
                    let n = min(*remaining, bytes_avail as u32);
                    let buf = &buf[..n as usize];

                    if !self.decode_options.ignore_crc {
                        crc.update(buf);
                    }

                    if *action == ChunkAction::Process {
                        if raw_bytes.len() == raw_bytes.capacity() {
                            if self.limits.bytes == 0 {
                                return Err(DecodingError::LimitsExceeded);
                            }

                            // Double the size of the Vec, but not beyond the allocation limit.
                            let reserve_size = raw_bytes
                                .capacity()
                                .max(CHUNK_BUFFER_SIZE)
                                .min(self.limits.bytes);

                            self.limits.reserve_bytes(reserve_size)?;
                            raw_bytes
                                .try_reserve_exact(reserve_size)
                                .map_err(|source| {
                                    DecodingError::allocation(
                                        "PNG raw chunk buffer",
                                        reserve_size,
                                        source,
                                    )
                                })?;
                        }
                        raw_bytes.extend_from_slice(buf);
                    }

                    *remaining -= n;
                    if *remaining == 0 {
                        debug_assert!(type_str != IDAT && type_str != chunk::FD_AT);
                        self.state = Some(State::new_u32(U32ValueKind::Crc(type_str)));
                    } else {
                        self.state = Some(ReadChunkData(type_str));
                    }
                    Ok((n as usize, Decoded::Nothing))
                }
            }
            ImageData(type_str) => {
                debug_assert!(type_str == IDAT || type_str == chunk::FD_AT);
                let len = std::cmp::min(buf.len(), self.current_chunk.remaining as usize);
                let buf = &buf[..len];

                let consumed = if let Some(image_data) = image_data {
                    self.inflater.decompress(buf, image_data)?
                } else {
                    len
                };

                if !self.decode_options.ignore_crc {
                    self.current_chunk.crc.update(&buf[..consumed]);
                }

                self.current_chunk.remaining -= consumed as u32;
                if self.current_chunk.remaining == 0 {
                    self.state = Some(State::new_u32(U32ValueKind::Crc(type_str)));
                } else {
                    self.state = Some(ImageData(type_str));
                }
                Ok((consumed, Decoded::ImageData))
            }
        }
    }

    fn parse_u32(
        &mut self,
        kind: U32ValueKind,
        u32_be_bytes: &[u8],
        image_data: Option<&mut UnfilterBuf<'_>>,
        consumed_bytes: usize,
    ) -> Result<(usize, Decoded), DecodingError> {
        debug_assert_eq!(u32_be_bytes.len(), 4);
        let Ok(bytes) = <[u8; 4]>::try_from(u32_be_bytes) else {
            return Err(DecodingError::InvalidState);
        };
        let val = u32::from_be_bytes(bytes);

        match kind {
            U32ValueKind::Signature1stU32 => {
                if bytes == [137, 80, 78, 71] {
                    self.state = Some(State::new_u32(U32ValueKind::Signature2ndU32));
                    Ok((consumed_bytes, Decoded::Nothing))
                } else {
                    Err(DecodingError::Format(
                        FormatErrorInner::InvalidSignature.into(),
                    ))
                }
            }
            U32ValueKind::Signature2ndU32 => {
                if bytes == [13, 10, 26, 10] {
                    self.state = Some(State::new_u32(U32ValueKind::Length));
                    Ok((consumed_bytes, Decoded::Nothing))
                } else {
                    Err(DecodingError::Format(
                        FormatErrorInner::InvalidSignature.into(),
                    ))
                }
            }
            U32ValueKind::Length => {
                self.state = Some(State::new_u32(U32ValueKind::Type { length: val }));
                Ok((consumed_bytes, Decoded::Nothing))
            }
            U32ValueKind::Type { length } => {
                let type_str = ChunkType(bytes);
                if self.info.is_none() && type_str != IHDR {
                    return Err(DecodingError::Format(
                        FormatErrorInner::ChunkBeforeIhdr { kind: type_str }.into(),
                    ));
                }
                if type_str != self.current_chunk.type_
                    && (self.current_chunk.type_ == IDAT
                        || self.current_chunk.type_ == chunk::FD_AT)
                {
                    let finished = match image_data {
                        Some(image_data) => self.inflater.finish(image_data)?,
                        None => true,
                    };

                    // We ended up handling IDAT/fdAT data rather than the chunk
                    // type header atually received. Thus rewind `self.state` to
                    // what it was before this function was called.
                    self.state = Some(State::U32 {
                        kind,
                        bytes,
                        accumulated_count: 4 - consumed_bytes,
                    });

                    if finished {
                        // We've processed all the image data necessary. Update
                        // `current_chunk.type_`so this codepath isn't taken
                        // again next time.
                        self.current_chunk.type_ = type_str;
                        self.ready_for_idat_chunks = false;
                        self.ready_for_fdat_chunks = false;
                        return Ok((0, Decoded::ImageDataFlushed));
                    } else {
                        // Report that we processed some image data without
                        // consuming any input. This gives the caller a chance
                        // to grow the output buffer and call us again.
                        return Ok((0, Decoded::ImageData));
                    }
                }

                self.current_chunk.type_ = type_str;
                if !self.decode_options.ignore_crc {
                    self.current_chunk.crc.reset();
                    self.current_chunk.crc.update(&type_str.0);
                }
                self.current_chunk.remaining = length;
                self.current_chunk.raw_bytes.clear();

                self.state = match type_str {
                    chunk::FD_AT => {
                        if !self.ready_for_fdat_chunks {
                            return Err(DecodingError::Format(
                                FormatErrorInner::UnexpectedRestartOfDataChunkSequence {
                                    kind: chunk::FD_AT,
                                }
                                .into(),
                            ));
                        }
                        if length < 4 {
                            return Err(DecodingError::Format(
                                FormatErrorInner::FdatShorterThanFourBytes.into(),
                            ));
                        }
                        self.current_chunk.action = ChunkAction::Process;
                        Some(State::new_u32(U32ValueKind::ApngSequenceNumber))
                    }
                    IDAT => {
                        if !self.ready_for_idat_chunks {
                            return Err(DecodingError::Format(
                                FormatErrorInner::UnexpectedRestartOfDataChunkSequence {
                                    kind: IDAT,
                                }
                                .into(),
                            ));
                        }
                        self.have_idat = true;
                        self.current_chunk.action = ChunkAction::Process;
                        Some(State::ImageData(type_str))
                    }
                    _ => Some(self.start_chunk(type_str, length)?),
                };
                Ok((consumed_bytes, Decoded::ChunkBegin(length, type_str)))
            }
            U32ValueKind::Crc(type_str) => {
                // If ignore_crc is set, do not calculate CRC. We set
                // sum=val so that it short-circuits to true in the next
                // if-statement block
                let sum = if self.decode_options.ignore_crc {
                    val
                } else {
                    self.current_chunk.crc.clone().finalize()
                };

                if val == sum || CHECKSUM_DISABLED {
                    match self.current_chunk.action {
                        ChunkAction::Process => {
                            // A fatal error in chunk parsing leaves the decoder in state 'None' to enforce
                            // that parsing can't continue after an error.
                            debug_assert!(self.state.is_none());
                            let decoded = self.parse_chunk(type_str)?;

                            if type_str != IEND {
                                self.state = Some(State::new_u32(U32ValueKind::Length));
                            }
                            Ok((consumed_bytes, decoded))
                        }
                        ChunkAction::Skip => {
                            self.state = Some(State::new_u32(U32ValueKind::Length));
                            Ok((
                                consumed_bytes,
                                Decoded::SkippedAncillaryChunk(self.current_chunk.type_),
                            ))
                        }
                        ChunkAction::Reject => {
                            self.state = Some(State::new_u32(U32ValueKind::Length));
                            Ok((consumed_bytes, Decoded::BadAncillaryChunk(type_str)))
                        }
                    }
                } else if self.decode_options.skip_ancillary_crc_failures
                    && !chunk::is_critical(type_str)
                {
                    // Ignore ancillary chunk with invalid CRC
                    self.state = Some(State::new_u32(U32ValueKind::Length));
                    Ok((consumed_bytes, Decoded::BadAncillaryChunk(type_str)))
                } else {
                    Err(DecodingError::Format(
                        FormatErrorInner::CrcMismatch {
                            crc_val: val,
                            crc_sum: sum,
                            chunk: type_str,
                        }
                        .into(),
                    ))
                }
            }
            U32ValueKind::ApngSequenceNumber => {
                debug_assert_eq!(self.current_chunk.type_, chunk::FD_AT);
                let next_seq_no = val;

                // Should be verified by the FdatShorterThanFourBytes check earlier.
                debug_assert!(self.current_chunk.remaining >= 4);
                self.current_chunk.remaining -= 4;

                if let Some(seq_no) = self.current_seq_no {
                    if next_seq_no != seq_no + 1 {
                        return Err(DecodingError::Format(
                            FormatErrorInner::ApngOrder {
                                present: next_seq_no,
                                expected: seq_no + 1,
                            }
                            .into(),
                        ));
                    }
                    self.current_seq_no = Some(next_seq_no);
                } else {
                    return Err(DecodingError::Format(FormatErrorInner::MissingFctl.into()));
                }

                if !self.decode_options.ignore_crc {
                    let data = next_seq_no.to_be_bytes();
                    self.current_chunk.crc.update(&data);
                }

                self.state = Some(State::ImageData(chunk::FD_AT));
                Ok((consumed_bytes, Decoded::Nothing))
            }
        }
    }
}
