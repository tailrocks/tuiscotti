use super::*;

impl Decompressor {
    /// Decompresses a chunk of data.
    ///
    /// Returns the number of bytes read from `input` and the number of bytes written to `output`,
    /// or an error if the deflate stream is not valid. `input` is the compressed data. `output` is
    /// the buffer to write the decompressed data to, starting at index `output_position`.
    /// `end_of_input` indicates whether more data may be available in the future.
    ///
    /// The contents of `output` after `output_position` are ignored. However, this function may
    /// write additional data to `output` past what is indicated by the return value.
    ///
    /// When this function returns `Ok`, at least one of the following is true:
    /// - The input is fully consumed.
    /// - The output is full but there are more bytes to output.
    /// - The deflate stream is complete (and `is_done` will return true).
    ///
    /// # Panics
    ///
    /// This function will panic if `output_position` is out of bounds.

    pub(crate) fn read(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        output_position: usize,
        end_of_input: bool,
    ) -> Result<(usize, usize), DecompressionError> {
        if let State::Done = self.state {
            return Ok((0, 0));
        }

        assert!(output_position <= output.len());

        let mut remaining_input = input;
        let mut output_index = output_position;

        if let Some((data, len)) = self.queued_rle.take() {
            let n = len.min(output.len() - output_index);
            output[output_index..][..n].fill(data);
            output_index += n;
            if n < len {
                self.queued_rle = Some((data, len - n));
                return Ok((0, n));
            }
        }
        if let Some((dist, len)) = self.queued_backref.take() {
            let n = len.min(output.len() - output_index);
            for i in 0..n {
                output[output_index + i] = output[output_index + i - dist];
            }
            output_index += n;
            if n < len {
                self.queued_backref = Some((dist, len - n));
                return Ok((0, n));
            }
        }

        // Main decoding state machine.
        let mut last_state = None;
        while last_state != Some(self.state) {
            last_state = Some(self.state);
            match self.state {
                State::ZlibHeader => {
                    self.fill_buffer(&mut remaining_input);
                    if self.nbits < 16 {
                        break;
                    }

                    let input0 = self.peak_bits(8);
                    let input1 = self.peak_bits(16) >> 8 & 0xff;
                    if input0 & 0x0f != 0x08
                        || (input0 & 0xf0) > 0x70
                        || input1 & 0x20 != 0
                        || (input0 << 8 | input1) % 31 != 0
                    {
                        return Err(DecompressionError::BadZlibHeader);
                    }

                    self.consume_bits(16);
                    self.state = State::BlockHeader;
                }
                State::BlockHeader => {
                    self.read_block_header(&mut remaining_input)?;
                }
                State::CodeLengthCodes => {
                    self.read_code_length_codes(&mut remaining_input)?;
                }
                State::CodeLengths => {
                    self.read_code_lengths(&mut remaining_input)?;
                }
                State::CompressedData => {
                    output_index =
                        self.read_compressed(&mut remaining_input, output, output_index)?
                }
                State::UncompressedData => {
                    // Drain any bytes from our buffer.
                    debug_assert_eq!(self.nbits % 8, 0);
                    while self.nbits > 0
                        && self.uncompressed_bytes_left > 0
                        && output_index < output.len()
                    {
                        output[output_index] = self.peak_bits(8) as u8;
                        self.consume_bits(8);
                        output_index += 1;
                        self.uncompressed_bytes_left -= 1;
                    }
                    // Buffer may contain one additional byte. Clear it to avoid confusion.
                    if self.nbits == 0 {
                        self.buffer = 0;
                    }

                    // Copy subsequent bytes directly from the input.
                    let copy_bytes = (self.uncompressed_bytes_left as usize)
                        .min(remaining_input.len())
                        .min(output.len() - output_index);
                    output[output_index..][..copy_bytes]
                        .copy_from_slice(&remaining_input[..copy_bytes]);
                    remaining_input = &remaining_input[copy_bytes..];
                    output_index += copy_bytes;
                    self.uncompressed_bytes_left -= copy_bytes as u16;

                    if self.uncompressed_bytes_left == 0 {
                        self.state = if self.last_block {
                            State::Checksum
                        } else {
                            State::BlockHeader
                        };
                    }
                }
                State::Checksum => {
                    self.fill_buffer(&mut remaining_input);

                    let align_bits = self.nbits % 8;
                    if self.nbits >= 32 + align_bits {
                        self.checksum.write(&output[output_position..output_index]);
                        if align_bits != 0 {
                            self.consume_bits(align_bits);
                        }
                        if !self.ignore_adler32
                            && (self.peak_bits(32) as u32).swap_bytes() != self.checksum.finish()
                        {
                            return Err(DecompressionError::WrongChecksum);
                        }
                        self.state = State::Done;
                        self.consume_bits(32);
                        break;
                    }
                }
                State::Done => unreachable!(),
            }
        }

        if !self.ignore_adler32 && self.state != State::Done {
            self.checksum.write(&output[output_position..output_index]);
        }

        if self.state == State::Done || !end_of_input || output_index == output.len() {
            let input_left = remaining_input.len();
            Ok((input.len() - input_left, output_index - output_position))
        } else {
            Err(DecompressionError::InsufficientInput)
        }
    }

    /// Returns true if the decompressor has finished decompressing the input.
    pub(crate) fn is_done(&self) -> bool {
        self.state == State::Done
    }
}
