use super::*;

impl Default for Decompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Decompressor {
    pub(crate) fn new() -> Self {
        Self {
            buffer: 0,
            nbits: 0,
            compression: CompressedBlock {
                litlen_table: [0; 4096],
                dist_table: [0; 512],
                secondary_table: Vec::new(),
                dist_secondary_table: Vec::new(),
                eof_code: 0,
                eof_mask: 0,
                eof_bits: 0,
            },
            header: BlockHeader {
                hlit: 0,
                hdist: 0,
                hclen: 0,
                table: [0; 128],
                num_lengths_read: 0,
                code_lengths: [0; 320],
            },
            uncompressed_bytes_left: 0,
            queued_rle: None,
            queued_backref: None,
            checksum: Adler32::new(),
            state: State::ZlibHeader,
            last_block: false,
            ignore_adler32: false,
            fixed_table: false,
            allocation_error: None,
        }
    }

    pub(crate) fn take_allocation_error(
        &mut self,
    ) -> Option<(usize, std::collections::TryReserveError)> {
        self.allocation_error.take()
    }

    /// Ignore the checksum at the end of the stream.
    pub(crate) fn ignore_adler32(&mut self) {
        self.ignore_adler32 = true;
    }

    pub(super) fn fill_buffer(&mut self, input: &mut &[u8]) {
        if input.len() >= 8 {
            let Ok(bytes) = <[u8; 8]>::try_from(&input[..8]) else {
                return;
            };
            self.buffer |= u64::from_le_bytes(bytes) << self.nbits;
            *input = &input[(63 - self.nbits as usize) / 8..];
            self.nbits |= 56;
        } else {
            let nbytes = input.len().min((63 - self.nbits as usize) / 8);
            let mut input_data = [0; 8];
            input_data[..nbytes].copy_from_slice(&input[..nbytes]);
            self.buffer |= u64::from_le_bytes(input_data)
                .checked_shl(self.nbits as u32)
                .unwrap_or(0);
            self.nbits += nbytes as u8 * 8;
            *input = &input[nbytes..];
        }
    }

    pub(super) fn peak_bits(&mut self, nbits: u8) -> u64 {
        debug_assert!(nbits <= 56 && nbits <= self.nbits);
        self.buffer & ((1u64 << nbits) - 1)
    }
    pub(super) fn consume_bits(&mut self, nbits: u8) {
        debug_assert!(self.nbits >= nbits);
        self.buffer >>= nbits;
        self.nbits -= nbits;
    }

    pub(super) fn read_block_header(
        &mut self,
        remaining_input: &mut &[u8],
    ) -> Result<(), DecompressionError> {
        self.fill_buffer(remaining_input);
        if self.nbits < 10 {
            return Ok(());
        }

        let start = self.peak_bits(3);
        self.last_block = start & 1 != 0;
        match start >> 1 {
            0b00 => {
                let align_bits = (self.nbits - 3) % 8;
                let header_bits = 3 + 32 + align_bits;
                if self.nbits < header_bits {
                    return Ok(());
                }

                let len = (self.peak_bits(align_bits + 19) >> (align_bits + 3)) as u16;
                let nlen = (self.peak_bits(header_bits) >> (align_bits + 19)) as u16;
                if nlen != !len {
                    return Err(DecompressionError::InvalidUncompressedBlockLength);
                }

                self.state = State::UncompressedData;
                self.uncompressed_bytes_left = len;
                self.consume_bits(header_bits);
                Ok(())
            }
            0b01 => {
                self.consume_bits(3);

                // Check for an entirely empty blocks which can happen if there are "partial
                // flushes" in the deflate stream. With fixed huffman codes, the EOF symbol is
                // 7-bits of zeros so we peak ahead and see if the next 7-bits are all zero.
                if self.peak_bits(7) == 0 {
                    self.consume_bits(7);
                    if self.last_block {
                        self.state = State::Checksum;
                        return Ok(());
                    }

                    // At this point we've consumed the entire block and need to read the next block
                    // header. If tail call optimization were guaranteed, we could just recurse
                    // here. But without it, a long sequence of empty fixed-blocks might cause a
                    // stack overflow. Instead, we consume all empty blocks in a loop and then
                    // recurse. This is the only recursive call this function, and thus is safe.
                    while self.nbits >= 10 && self.peak_bits(10) == 0b010 {
                        self.consume_bits(10);
                        self.fill_buffer(remaining_input);
                    }
                    return self.read_block_header(remaining_input);
                }

                // Build decoding tables if the previous block wasn't also a fixed block.
                if !self.fixed_table {
                    self.fixed_table = true;
                    for chunk in self.compression.litlen_table.chunks_exact_mut(512) {
                        chunk.copy_from_slice(&FIXED_LITLEN_TABLE);
                    }
                    for chunk in self.compression.dist_table.chunks_exact_mut(32) {
                        chunk.copy_from_slice(&FIXED_DIST_TABLE);
                    }
                    self.compression.eof_bits = 7;
                    self.compression.eof_code = 0;
                    self.compression.eof_mask = 0x7f;
                }

                self.state = State::CompressedData;
                Ok(())
            }
            0b10 => {
                if self.nbits < 17 {
                    return Ok(());
                }

                self.header.hlit = (self.peak_bits(8) >> 3) as usize + 257;
                self.header.hdist = (self.peak_bits(13) >> 8) as usize + 1;
                self.header.hclen = (self.peak_bits(17) >> 13) as usize + 4;
                if self.header.hlit > 286 {
                    return Err(DecompressionError::InvalidHlit);
                }
                if self.header.hdist > 30 {
                    return Err(DecompressionError::InvalidHdist);
                }

                self.consume_bits(17);
                self.state = State::CodeLengthCodes;
                self.fixed_table = false;
                Ok(())
            }
            0b11 => Err(DecompressionError::InvalidBlockType),
            _ => unreachable!(),
        }
    }

    pub(super) fn read_code_length_codes(
        &mut self,
        remaining_input: &mut &[u8],
    ) -> Result<(), DecompressionError> {
        self.fill_buffer(remaining_input);
        if self.nbits as usize + remaining_input.len() * 8 < 3 * self.header.hclen {
            return Ok(());
        }

        let mut code_length_lengths = [0; 19];
        for i in 0..self.header.hclen {
            code_length_lengths[CLCL_ORDER[i]] = self.peak_bits(3) as u8;
            self.consume_bits(3);

            // We need to refill the buffer after reading 3 * 18 = 54 bits since the buffer holds
            // between 56 and 63 bits total.
            if i == 17 {
                self.fill_buffer(remaining_input);
            }
        }

        let mut codes = [0; 19];
        if !table_result(
            build_table(
                &code_length_lengths,
                &[],
                &mut codes,
                &mut self.header.table,
                &mut Vec::new(),
                false,
                false,
            ),
            &mut self.allocation_error,
        )? {
            return Err(DecompressionError::BadCodeLengthHuffmanTree);
        }

        self.state = State::CodeLengths;
        self.header.num_lengths_read = 0;
        Ok(())
    }

    pub(super) fn read_code_lengths(
        &mut self,
        remaining_input: &mut &[u8],
    ) -> Result<(), DecompressionError> {
        let total_lengths = self.header.hlit + self.header.hdist;
        while self.header.num_lengths_read < total_lengths {
            self.fill_buffer(remaining_input);
            if self.nbits < 7 {
                return Ok(());
            }

            let code = self.peak_bits(7);
            let entry = self.header.table[code as usize];
            let length = (entry & 0x7) as u8;
            let symbol = (entry >> 16) as u8;

            debug_assert!(length != 0);
            match symbol {
                0..=15 => {
                    self.header.code_lengths[self.header.num_lengths_read] = symbol;
                    self.header.num_lengths_read += 1;
                    self.consume_bits(length);
                }
                16..=18 => {
                    let (base_repeat, extra_bits) = match symbol {
                        16 => (3, 2),
                        17 => (3, 3),
                        18 => (11, 7),
                        _ => unreachable!(),
                    };

                    if self.nbits < length + extra_bits {
                        return Ok(());
                    }

                    let value = match symbol {
                        16 => {
                            self.header.code_lengths[self
                                .header
                                .num_lengths_read
                                .checked_sub(1)
                                .ok_or(DecompressionError::InvalidCodeLengthRepeat)?]
                            // TODO: is this right?
                        }
                        17 => 0,
                        18 => 0,
                        _ => unreachable!(),
                    };

                    let repeat =
                        (self.peak_bits(length + extra_bits) >> length) as usize + base_repeat;
                    if self.header.num_lengths_read + repeat > total_lengths {
                        return Err(DecompressionError::InvalidCodeLengthRepeat);
                    }

                    for i in 0..repeat {
                        self.header.code_lengths[self.header.num_lengths_read + i] = value;
                    }
                    self.header.num_lengths_read += repeat;
                    self.consume_bits(length + extra_bits);
                }
                _ => unreachable!(),
            }
        }

        self.header
            .code_lengths
            .copy_within(self.header.hlit..total_lengths, 288);
        for i in self.header.hlit..288 {
            self.header.code_lengths[i] = 0;
        }
        for i in 288 + self.header.hdist..320 {
            self.header.code_lengths[i] = 0;
        }

        Self::build_tables(
            self.header.hlit,
            &self.header.code_lengths,
            &mut self.compression,
            &mut self.allocation_error,
        )?;
        self.state = State::CompressedData;
        Ok(())
    }

    pub(super) fn build_tables(
        hlit: usize,
        code_lengths: &[u8],
        compression: &mut CompressedBlock,
        allocation_error: &mut Option<(usize, std::collections::TryReserveError)>,
    ) -> Result<(), DecompressionError> {
        // If there is no code assigned for the EOF symbol then the bitstream is invalid.
        if code_lengths[256] == 0 {
            // TODO: Return a dedicated error in this case.
            return Err(DecompressionError::BadLiteralLengthHuffmanTree);
        }

        let mut codes = [0; 288];
        compression.secondary_table.clear();
        if !table_result(
            huffman::build_table(
                &code_lengths[..hlit],
                &LITLEN_TABLE_ENTRIES,
                &mut codes[..hlit],
                &mut compression.litlen_table,
                &mut compression.secondary_table,
                false,
                true,
            ),
            allocation_error,
        )? {
            return Err(DecompressionError::BadCodeLengthHuffmanTree);
        }

        compression.eof_code = codes[256];
        compression.eof_mask = (1 << code_lengths[256]) - 1;
        compression.eof_bits = code_lengths[256];

        // Build the distance code table.
        let lengths = &code_lengths[288..320];
        if lengths == [0; 32] {
            compression.dist_table.fill(0);
        } else {
            let mut dist_codes = [0; 32];
            if !table_result(
                huffman::build_table(
                    lengths,
                    &tables::DISTANCE_TABLE_ENTRIES,
                    &mut dist_codes,
                    &mut compression.dist_table,
                    &mut compression.dist_secondary_table,
                    true,
                    false,
                ),
                allocation_error,
            )? {
                return Err(DecompressionError::BadDistanceHuffmanTree);
            }
        }

        Ok(())
    }
}
