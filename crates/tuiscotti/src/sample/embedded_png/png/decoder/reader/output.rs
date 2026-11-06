use super::*;

impl<R: BufRead + Seek> Reader<R> {
    /// Returns the color type and the number of bits per sample
    /// of the data returned by [`Reader::next_row`] and [`Reader::next_frame`].

    pub(crate) fn output_color_type(&self) -> Result<(ColorType, BitDepth), DecodingError> {
        use crate::sample::embedded_png::png::common::ColorType::*;
        let t = self.transform;
        let info = self.info()?;
        if t == Transformations::IDENTITY {
            Ok((info.color_type, info.bit_depth))
        } else {
            let bits = match info.bit_depth as u8 {
                16 if t.intersects(Transformations::STRIP_16) => 8,
                n if n < 8
                    && (t.contains(Transformations::EXPAND)
                        || t.contains(Transformations::ALPHA)) =>
                {
                    8
                }
                n => n,
            };
            let color_type =
                if t.contains(Transformations::EXPAND) || t.contains(Transformations::ALPHA) {
                    let has_trns = info.trns.is_some() || t.contains(Transformations::ALPHA);
                    match info.color_type {
                        Grayscale if has_trns => GrayscaleAlpha,
                        Rgb if has_trns => Rgba,
                        Indexed if has_trns => Rgba,
                        Indexed => Rgb,
                        ct => ct,
                    }
                } else {
                    info.color_type
                };
            Ok((
                color_type,
                BitDepth::from_u8(bits).unwrap_or(info.bit_depth),
            ))
        }
    }

    /// Return the number of bytes required to hold a deinterlaced image frame that is decoded
    /// using the given input transformations.
    ///
    /// Returns `None` if the output buffer does not fit into the memory space of the machine,
    /// otherwise returns the byte length in `Some`. The length is smaller than [`isize::MAX`].
    pub(crate) fn output_buffer_size(&self) -> Result<Option<usize>, DecodingError> {
        let (width, height) = self.info()?.size();
        let (color, depth) = self.output_color_type()?;
        // The subtraction should always work, but we do this for consistency. Also note that by
        // calling `checked_raw_row_length` the row buffer is guaranteed to work whereas if we
        // ran other function that didn't include the filter byte that could later fail on an image
        // that is `1xN`...
        let Some(linelen) = color
            .checked_raw_row_length(depth, width)
            .and_then(|len| len.checked_sub(1))
        else {
            return Ok(None);
        };
        let Ok(height) = usize::try_from(height) else {
            return Ok(None);
        };
        let Some(imglen) = linelen.checked_mul(height) else {
            return Ok(None);
        };
        // Ensure that it fits into address space not only `usize` to allocate.
        Ok((imglen <= isize::MAX as usize).then_some(imglen))
    }

    /// Returns the number of bytes required to hold a deinterlaced row.
    pub(crate) fn unguarded_output_line_size(&self, width: u32) -> Result<usize, DecodingError> {
        let (color, depth) = self.output_color_type()?;
        Ok(color.raw_row_length_from_width(depth, width) - 1)
    }

    /// Returns the number of bytes required to hold a deinterlaced row.
    ///
    /// Returns `None` if the output buffer does not fit into the memory space of the machine,
    /// otherwise returns the byte length in `Some`. The length is smaller than [`isize::MAX`].
    pub(crate) fn output_line_size(&self, width: u32) -> Result<Option<usize>, DecodingError> {
        let (color, depth) = self.output_color_type()?;
        let Some(length) = color
            .checked_raw_row_length(depth, width)
            .and_then(|len| len.checked_sub(1))
        else {
            return Ok(None);
        };
        // Ensure that it fits into address space not only `usize` to allocate.
        Ok((length <= isize::MAX as usize).then_some(length))
    }

    /// Unfilter the next raw interlaced row into `self.unfiltering_buffer`.
    pub(super) fn next_raw_interlaced_row(&mut self, rowlen: usize) -> Result<(), DecodingError> {
        // Read image data until we have at least one full row (but possibly more than one).
        while self.unfiltering_buffer.mutable_len_of_curr_row() < rowlen {
            if self.subframe.consumed_and_flushed {
                return Err(DecodingError::Format(
                    FormatErrorInner::NoMoreImageData.into(),
                ));
            }

            assert!(self.unfiltering_buffer.remaining_bytes() > 0);
            let mut limits = self.decoder.limits();
            let completion_status = self
                .unfiltering_buffer
                .with_unfilled_buffer(&mut limits, |buffer| {
                    self.decoder.decode_image_data(Some(buffer))
                });
            self.decoder.set_limits(limits);
            let completion_status = completion_status?;
            match completion_status {
                Ok(ImageDataCompletionStatus::ExpectingMoreData) => (),
                Ok(ImageDataCompletionStatus::Done) => self.mark_subframe_as_consumed_and_flushed(),
                Err(DecodingError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof
                        && self.unfiltering_buffer.readable_len_of_curr_row() >= rowlen =>
                {
                    let mut limits = self.decoder.limits();
                    let result = self
                        .unfiltering_buffer
                        .unfilter_curr_row_using_scratch_buffer(rowlen, self.bpp, &mut limits);
                    self.decoder.set_limits(limits);
                    return result;
                }
                Err(other_error) => return Err(other_error),
            }
        }

        self.unfiltering_buffer
            .unfilter_curr_row_in_place(rowlen, self.bpp)
    }
}
