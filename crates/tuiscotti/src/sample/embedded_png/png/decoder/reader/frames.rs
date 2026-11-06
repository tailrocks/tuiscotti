use super::*;

impl<R: BufRead + Seek> Reader<R> {
    /// Advances to the start of the next animation frame and
    /// returns a reference to the [`FrameControl`] info that describes it.
    /// Skips and discards the image data of the previous frame if necessary.
    ///
    /// Returns a [`ParameterError`] when there are no more animation frames.
    /// To avoid this the caller can check if [`Info::animation_control`] exists
    /// and consult [`AnimationControl::num_frames`].
    ///
    /// [`ParameterError`]: crate::sample::embedded_png::png::ParameterError
    /// [`AnimationControl::num_frames`]: crate::sample::embedded_png::png::AnimationControl::num_frames
    pub(crate) fn next_frame_info(&mut self) -> Result<&FrameControl, DecodingError> {
        let remaining_frames = if self.subframe.consumed_and_flushed {
            self.remaining_frames
        } else {
            // One remaining frame will be consumed by the `finish_decoding` call below.
            self.remaining_frames - 1
        };
        if remaining_frames == 0 {
            return Err(DecodingError::Parameter(
                ParameterErrorKind::PolledAfterEndOfImage.into(),
            ));
        }

        if !self.subframe.consumed_and_flushed {
            self.subframe.current_interlace_info = None;
            self.finish_decoding()?;
        }
        self.read_until_image_data()?;

        self.info()?
            .frame_control
            .as_ref()
            .ok_or_else(|| DecodingError::Format(FormatErrorInner::MissingFctl.into()))
    }

    /// Reads all meta data until the next frame data starts.
    /// Requires IHDR before the IDAT and fcTL before fdAT.
    pub(in crate::sample::embedded_png::png::decoder) fn read_until_image_data(
        &mut self,
    ) -> Result<(), DecodingError> {
        self.decoder.read_until_image_data()?;

        let info = self.decoder.info().ok_or(DecodingError::InvalidState)?;
        self.subframe = SubframeInfo::new(info);
        self.bpp = info.bpp_in_prediction();

        let frame_bytes = if info.interlaced {
            let mut bytes = 0u64;
            for pass in crate::sample::embedded_png::png::adam7::PassConstants::PASSES {
                bytes += info.raw_row_length_from_width(pass.count_samples(self.subframe.width))
                    as u64
                    * pass.count_lines(self.subframe.height) as u64;
            }
            bytes
        } else {
            (self.subframe.rowlen as u64) * self.subframe.height as u64
        };
        self.unfiltering_buffer.start_frame(frame_bytes);

        // Allocate output buffer.
        let buflen = self.unguarded_output_line_size(self.subframe.width)?;
        self.decoder.reserve_bytes(buflen)?;

        Ok(())
    }

    /// Get information on the image.
    ///
    /// The structure will change as new frames of an animated image are decoded.
    pub(crate) fn info(&self) -> Result<&Info<'static>, DecodingError> {
        self.decoder.info().ok_or(DecodingError::InvalidState)
    }

    /// Decodes the next frame into `buf`.
    ///
    /// Note that this decodes raw subframes that need to be mixed according to blend-op and
    /// dispose-op by the caller.
    ///
    /// The caller must always provide a buffer large enough to hold a complete frame (the APNG
    /// specification restricts subframes to the dimensions given in the image header). The region
    /// that has been written be checked afterwards by calling [`Reader::info`] after a successful call and
    /// inspecting the `frame_control` data. This requirement may be lifted in a later version of
    /// `png`.
    ///
    /// Output lines will be written in row-major, packed matrix with width and height of the read
    /// frame (or subframe), all samples are in big endian byte order where this matters.
    pub(crate) fn next_frame(&mut self, buf: &mut [u8]) -> Result<OutputInfo, DecodingError> {
        if self.remaining_frames == 0 {
            return Err(DecodingError::Parameter(
                ParameterErrorKind::PolledAfterEndOfImage.into(),
            ));
        } else if self.subframe.consumed_and_flushed {
            // Advance until the next `fdAT`
            // (along the way we should encounter the fcTL for this frame).
            self.read_until_image_data()?;
        }

        // Note that we only check if the buffer size calculation holds in a call to decoding the
        // frame. Consequently, we can represent the `Info` and frameless decoding even when the
        // target architecture's address space is too small for a frame. However reading the actual
        let required_len = self
            .output_buffer_size()?
            .ok_or(DecodingError::LimitsExceeded)?;

        if buf.len() < required_len {
            return Err(DecodingError::Parameter(
                ParameterErrorKind::ImageBufferSize {
                    expected: required_len,
                    actual: buf.len(),
                }
                .into(),
            ));
        }

        let (color_type, bit_depth) = self.output_color_type()?;
        let output_info = OutputInfo {
            width: self.subframe.width,
            height: self.subframe.height,
            color_type,
            bit_depth,
            line_size: self.unguarded_output_line_size(self.subframe.width)?,
        };

        let info = self.info()?;
        if info.interlaced {
            let stride = self.unguarded_output_line_size(info.width)?;
            let samples = color_type.samples() as u8;
            let bits_pp = samples * (bit_depth as u8);
            let expand = crate::sample::embedded_png::png::adam7::expand_pass;

            while let Some(InterlacedRow {
                data: row,
                interlace,
                ..
            }) = self.next_interlaced_row()?
            {
                let adam7info = interlace
                    .get_adam7_info()
                    .ok_or(DecodingError::InvalidState)?;
                expand(buf, stride, row, adam7info, bits_pp);
            }
        } else {
            let current_interlace_info = self.subframe.current_interlace_info.as_ref();
            let already_done_rows = current_interlace_info
                .map(|info| info.line_number())
                .unwrap_or(self.subframe.height);

            for row in buf
                .chunks_exact_mut(output_info.line_size)
                .take(self.subframe.height as usize)
                .skip(already_done_rows as usize)
            {
                self.next_interlaced_row_impl(self.subframe.rowlen, row)?;
            }
        }

        // Advance over the rest of data for this (sub-)frame.
        self.finish_decoding()?;

        Ok(output_info)
    }

    pub(super) fn mark_subframe_as_consumed_and_flushed(&mut self) {
        assert!(self.remaining_frames > 0);
        self.remaining_frames -= 1;

        self.subframe.consumed_and_flushed = true;
    }

    /// Advance over the rest of data for this (sub-)frame.
    /// Called after decoding the last row of a frame.
    pub(super) fn finish_decoding(&mut self) -> Result<(), DecodingError> {
        // Double-check that all rows of this frame have been decoded (i.e. that the potential
        // `finish_decoding` call below won't be discarding any data).
        assert!(self.subframe.current_interlace_info.is_none());

        // Discard the remaining data in the current sequence of `IDAT` or `fdAT` chunks.
        if !self.subframe.consumed_and_flushed {
            self.decoder.finish_decoding_image_data()?;
            self.mark_subframe_as_consumed_and_flushed();
        }

        Ok(())
    }
}
