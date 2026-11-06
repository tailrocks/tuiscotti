use super::*;

impl Decompressor {
    pub(super) fn read_compressed_careful(
        &mut self,
        remaining_input: &mut &[u8],
        output: &mut [u8],
        mut output_index: usize,
    ) -> Result<usize, DecompressionError> {
        // Careful decoding loop.
        //
        // This loop processes the remaining input when we're too close to the end of the input or
        // output to use the fast loop.
        while let State::CompressedData = self.state {
            self.fill_buffer(remaining_input);
            if output_index == output.len() {
                break;
            }

            let mut bits = self.buffer;
            let litlen_entry = self.compression.litlen_table[(bits & 0xfff) as usize];
            let litlen_code_bits = litlen_entry as u8;

            if litlen_entry & LITERAL_ENTRY != 0 {
                // Fast path: the next symbol is <= 12 bits and a literal, the table specifies the
                // output bytes and we can directly write them to the output buffer.
                let advance_output_bytes = ((litlen_entry & 0xf00) >> 8) as usize;

                if self.nbits < litlen_code_bits {
                    break;
                } else if output_index + 1 < output.len() {
                    output[output_index] = (litlen_entry >> 16) as u8;
                    output[output_index + 1] = (litlen_entry >> 24) as u8;
                    output_index += advance_output_bytes;
                    self.consume_bits(litlen_code_bits);
                    continue;
                } else if output_index + advance_output_bytes == output.len() {
                    debug_assert_eq!(advance_output_bytes, 1);
                    output[output_index] = (litlen_entry >> 16) as u8;
                    output_index += 1;
                    self.consume_bits(litlen_code_bits);
                    break;
                } else {
                    debug_assert_eq!(advance_output_bytes, 2);
                    output[output_index] = (litlen_entry >> 16) as u8;
                    self.queued_rle = Some(((litlen_entry >> 24) as u8, 1));
                    output_index += 1;
                    self.consume_bits(litlen_code_bits);
                    break;
                }
            }

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

                    if self.nbits < litlen_code_bits {
                        break;
                    } else if litlen_symbol < 256 {
                        self.consume_bits(litlen_code_bits);
                        output[output_index] = litlen_symbol as u8;
                        output_index += 1;
                        continue;
                    } else if litlen_symbol == 256 {
                        self.consume_bits(litlen_code_bits);
                        self.state = match self.last_block {
                            true => State::Checksum,
                            false => State::BlockHeader,
                        };
                        break;
                    }

                    (
                        LEN_SYM_TO_LEN_BASE[litlen_symbol as usize - 257] as u32,
                        LEN_SYM_TO_LEN_EXTRA[litlen_symbol as usize - 257],
                        litlen_code_bits,
                    )
                } else if litlen_code_bits == 0 {
                    return Err(DecompressionError::InvalidLiteralLengthCode);
                } else {
                    if self.nbits < litlen_code_bits {
                        break;
                    }
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
            } else if self.nbits > litlen_code_bits + length_extra_bits + 9 {
                if dist_entry >> 8 == 0 {
                    return Err(DecompressionError::InvalidDistanceCode);
                }

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
            } else {
                break;
            };
            bits >>= dist_code_bits;

            let dist = dist_base as usize + (bits & ((1 << dist_extra_bits) - 1)) as usize;
            let total_bits =
                litlen_code_bits + length_extra_bits + dist_code_bits + dist_extra_bits;

            if self.nbits < total_bits {
                break;
            } else if dist > output_index {
                return Err(DecompressionError::DistanceTooFarBack);
            }

            self.consume_bits(total_bits);

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

        Ok(output_index)
    }
}
