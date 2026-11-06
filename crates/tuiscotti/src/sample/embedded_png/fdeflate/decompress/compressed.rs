use super::*;

impl Decompressor {
    pub(super) fn read_compressed(
        &mut self,
        remaining_input: &mut &[u8],
        output: &mut [u8],
        mut output_index: usize,
    ) -> Result<usize, DecompressionError> {
        // Fast decoding loop.
        //
        // This loop is optimized for speed and is the main decoding loop for the decompressor,
        // which is used when there are at least 8 bytes of input and output data available. It
        // assumes that the bitbuffer is full (nbits >= 56) and that litlen_entry has been loaded.
        //
        // These assumptions enable a few optimizations:
        // - Nearly all checks for nbits are avoided.
        // - Checking the input size is optimized out in the refill function call.
        // - The litlen_entry for the next loop iteration can be loaded in parallel with refilling
        //   the bit buffer. This is because when the input is non-empty, the bit buffer actually
        //   has 64-bits of valid data (even though nbits will be in 56..=63).
        self.fill_buffer(remaining_input);
        let mut litlen_entry = self.compression.litlen_table[(self.buffer & 0xfff) as usize];
        while self.state == State::CompressedData
            && output_index + 8 <= output.len()
            && remaining_input.len() >= 8
        {
            // First check whether the next symbol is a literal. This code does up to 2 additional
            // table lookups to decode more literals.
            let mut bits;
            let mut litlen_code_bits = litlen_entry as u8;
            if litlen_entry & LITERAL_ENTRY != 0 {
                let litlen_entry2 = self.compression.litlen_table
                    [(self.buffer >> litlen_code_bits & 0xfff) as usize];
                let litlen_code_bits2 = litlen_entry2 as u8;
                let litlen_entry3 = self.compression.litlen_table
                    [(self.buffer >> (litlen_code_bits + litlen_code_bits2) & 0xfff) as usize];
                let litlen_code_bits3 = litlen_entry3 as u8;
                let litlen_entry4 = self.compression.litlen_table[(self.buffer
                    >> (litlen_code_bits + litlen_code_bits2 + litlen_code_bits3)
                    & 0xfff)
                    as usize];

                let advance_output_bytes = ((litlen_entry & 0xf00) >> 8) as usize;
                output[output_index] = (litlen_entry >> 16) as u8;
                output[output_index + 1] = (litlen_entry >> 24) as u8;
                output_index += advance_output_bytes;

                if litlen_entry2 & LITERAL_ENTRY != 0 {
                    let advance_output_bytes2 = ((litlen_entry2 & 0xf00) >> 8) as usize;
                    output[output_index] = (litlen_entry2 >> 16) as u8;
                    output[output_index + 1] = (litlen_entry2 >> 24) as u8;
                    output_index += advance_output_bytes2;

                    if litlen_entry3 & LITERAL_ENTRY != 0 {
                        let advance_output_bytes3 = ((litlen_entry3 & 0xf00) >> 8) as usize;
                        output[output_index] = (litlen_entry3 >> 16) as u8;
                        output[output_index + 1] = (litlen_entry3 >> 24) as u8;
                        output_index += advance_output_bytes3;

                        litlen_entry = litlen_entry4;
                        self.consume_bits(litlen_code_bits + litlen_code_bits2 + litlen_code_bits3);
                        self.fill_buffer(remaining_input);
                        continue;
                    } else {
                        self.consume_bits(litlen_code_bits + litlen_code_bits2);
                        litlen_entry = litlen_entry3;
                        litlen_code_bits = litlen_code_bits3;
                        self.fill_buffer(remaining_input);
                        bits = self.buffer;
                    }
                } else {
                    self.consume_bits(litlen_code_bits);
                    bits = self.buffer;
                    litlen_entry = litlen_entry2;
                    litlen_code_bits = litlen_code_bits2;
                    if self.nbits < 48 {
                        self.fill_buffer(remaining_input);
                    }
                }
            } else {
                bits = self.buffer;
            }

            // The next symbol is either a 13+ bit literal, back-reference, or an EOF symbol.
            let (length_base, length_extra_bits, litlen_code_bits) =
                if litlen_entry & EXCEPTIONAL_ENTRY == 0 {
                    (
                        litlen_entry >> 16,
                        (litlen_entry >> 8) as u8,
                        litlen_code_bits,
                    )
                } else if litlen_entry & SECONDARY_TABLE_ENTRY != 0 {
                    let secondary_table_index =
                        (litlen_entry >> 16) + ((bits >> 12) as u32 & (litlen_entry & 0xff));
                    let secondary_entry =
                        self.compression.secondary_table[secondary_table_index as usize];
                    let litlen_symbol = secondary_entry >> 4;
                    let litlen_code_bits = (secondary_entry & 0xf) as u8;

                    match litlen_symbol {
                        0..=255 => {
                            self.consume_bits(litlen_code_bits);
                            litlen_entry =
                                self.compression.litlen_table[(self.buffer & 0xfff) as usize];
                            self.fill_buffer(remaining_input);
                            output[output_index] = litlen_symbol as u8;
                            output_index += 1;
                            continue;
                        }
                        256 => {
                            self.consume_bits(litlen_code_bits);
                            self.state = match self.last_block {
                                true => State::Checksum,
                                false => State::BlockHeader,
                            };
                            break;
                        }
                        _ => (
                            LEN_SYM_TO_LEN_BASE[litlen_symbol as usize - 257] as u32,
                            LEN_SYM_TO_LEN_EXTRA[litlen_symbol as usize - 257],
                            litlen_code_bits,
                        ),
                    }
                } else if litlen_code_bits == 0 {
                    return Err(DecompressionError::InvalidLiteralLengthCode);
                } else {
                    self.consume_bits(litlen_code_bits);
                    self.state = match self.last_block {
                        true => State::Checksum,
                        false => State::BlockHeader,
                    };
                    break;
                };
            bits >>= litlen_code_bits;

            let length_extra_mask = (1 << length_extra_bits) - 1;
            let length = length_base as usize + (bits & length_extra_mask) as usize;
            bits >>= length_extra_bits;

            let dist_entry = self.compression.dist_table[(bits & 0x1ff) as usize];
            let (dist_base, dist_extra_bits, dist_code_bits) = if dist_entry & LITERAL_ENTRY != 0 {
                (
                    (dist_entry >> 16) as u16,
                    (dist_entry >> 8) as u8 & 0xf,
                    dist_entry as u8,
                )
            } else if dist_entry >> 8 == 0 {
                return Err(DecompressionError::InvalidDistanceCode);
            } else {
                let secondary_table_index =
                    (dist_entry >> 16) + ((bits >> 9) as u32 & (dist_entry & 0xff));
                let secondary_entry =
                    self.compression.dist_secondary_table[secondary_table_index as usize];
                let dist_symbol = (secondary_entry >> 4) as usize;
                if dist_symbol >= 30 {
                    return Err(DecompressionError::InvalidDistanceCode);
                }

                (
                    DIST_SYM_TO_DIST_BASE[dist_symbol],
                    DIST_SYM_TO_DIST_EXTRA[dist_symbol],
                    (secondary_entry & 0xf) as u8,
                )
            };
            bits >>= dist_code_bits;

            let dist = dist_base as usize + (bits & ((1 << dist_extra_bits) - 1)) as usize;
            if dist > output_index {
                return Err(DecompressionError::DistanceTooFarBack);
            }

            self.consume_bits(
                litlen_code_bits + length_extra_bits + dist_code_bits + dist_extra_bits,
            );
            self.fill_buffer(remaining_input);
            litlen_entry = self.compression.litlen_table[(self.buffer & 0xfff) as usize];

            let copy_length = length.min(output.len() - output_index);
            if dist == 1 {
                let last = output[output_index - 1];
                output[output_index..][..copy_length].fill(last);

                if copy_length < length {
                    self.queued_rle = Some((last, length - copy_length));
                    output_index = output.len();
                    break;
                }
            } else if output_index + length + 15 <= output.len() {
                let start = output_index - dist;
                output.copy_within(start..start + 16, output_index);

                if length > 16 || dist < 16 {
                    for i in (0..length).step_by(dist.min(16)).skip(1) {
                        output.copy_within(start + i..start + i + 16, output_index + i);
                    }
                }
            } else {
                if dist < copy_length {
                    for i in 0..copy_length {
                        output[output_index + i] = output[output_index + i - dist];
                    }
                } else {
                    output.copy_within(
                        output_index - dist..output_index + copy_length - dist,
                        output_index,
                    )
                }

                if copy_length < length {
                    self.queued_backref = Some((dist, length - copy_length));
                    output_index = output.len();
                    break;
                }
            }
            output_index += copy_length;
        }

        output_index = self.read_compressed_careful(remaining_input, output, output_index)?;

        if self.state == State::CompressedData
            && self.queued_backref.is_none()
            && self.queued_rle.is_none()
            && self.nbits >= 15
            && self.peak_bits(15) as u16 & self.compression.eof_mask == self.compression.eof_code
        {
            self.consume_bits(self.compression.eof_bits);
            self.state = match self.last_block {
                true => State::Checksum,
                false => State::BlockHeader,
            };
        }

        Ok(output_index)
    }
}
