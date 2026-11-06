use super::*;

impl<W: Write> Writer<W> {
    /// Set the used filter type.
    ///
    /// The default filter is [`Filter::Adaptive`] which automatically selects the best filter
    /// for each row of the image.
    ///
    /// You should only change this if you are after very fast compression,
    /// and either don't care about compression ratio or know exactly what works best for your images.

    pub(crate) fn set_filter(&mut self, filter: Filter) {
        self.options.filter = filter;
    }

    /// Set the fraction of time the following frames are going to be displayed,
    /// in seconds
    ///
    /// If the denominator is 0, it is to be treated as if it were 100
    /// (that is, the numerator then specifies 1/100ths of a second).
    /// If the value of the numerator is 0 the decoder should render the next frame
    /// as quickly as possible, though viewers may impose a reasonable lower bound.
    ///
    /// This method will return an error if the image is not animated.
    pub(crate) fn set_frame_delay(&mut self, numerator: u16, denominator: u16) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.delay_den = denominator;
            fctl.delay_num = numerator;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the dimension of the following frames.
    ///
    /// This function will return an error when:
    /// - The image is not an animated;
    ///
    /// - The selected dimension, considering also the current frame position,
    ///   goes outside the image boundaries;
    ///
    /// - One or both the width and height are 0;
    ///
    // ??? TODO ???
    // - The next frame is the default image
    pub(crate) fn set_frame_dimension(&mut self, width: u32, height: u32) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            if Some(width) > self.info.width.checked_sub(fctl.x_offset)
                || Some(height) > self.info.height.checked_sub(fctl.y_offset)
            {
                return Err(EncodingError::Format(FormatErrorKind::OutOfBounds.into()));
            } else if width == 0 {
                return Err(EncodingError::Format(FormatErrorKind::ZeroWidth.into()));
            } else if height == 0 {
                return Err(EncodingError::Format(FormatErrorKind::ZeroHeight.into()));
            }
            fctl.width = width;
            fctl.height = height;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the position of the following frames.
    ///
    /// An error will be returned if:
    /// - The image is not animated;
    ///
    /// - The selected position, considering also the current frame dimension,
    ///   goes outside the image boundaries;
    ///
    // ??? TODO ???
    // - The next frame is the default image
    pub(crate) fn set_frame_position(&mut self, x: u32, y: u32) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            if Some(x) > self.info.width.checked_sub(fctl.width)
                || Some(y) > self.info.height.checked_sub(fctl.height)
            {
                return Err(EncodingError::Format(FormatErrorKind::OutOfBounds.into()));
            }
            fctl.x_offset = x;
            fctl.y_offset = y;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the frame dimension to occupy all the image, starting from
    /// the current position.
    ///
    /// To reset the frame to the full image size [`reset_frame_position`]
    /// should be called first.
    ///
    /// This method will return an error if the image is not animated.
    ///
    /// [`reset_frame_position`]: Writer::reset_frame_position
    pub(crate) fn reset_frame_dimension(&mut self) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.width = self.info.width - fctl.x_offset;
            fctl.height = self.info.height - fctl.y_offset;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the frame position to (0, 0).
    ///
    /// Equivalent to calling [`set_frame_position(0, 0)`].
    ///
    /// This method will return an error if the image is not animated.
    ///
    /// [`set_frame_position(0, 0)`]: Writer::set_frame_position
    pub(crate) fn reset_frame_position(&mut self) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.x_offset = 0;
            fctl.y_offset = 0;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the blend operation for the following frames.
    ///
    /// The blend operation specifies whether the frame is to be alpha blended
    /// into the current output buffer content, or whether it should completely
    /// replace its region in the output buffer.
    ///
    /// See the [`BlendOp`] documentation for the possible values and their effects.
    ///
    /// *Note that for the first frame the two blend modes are functionally
    /// equivalent due to the clearing of the output buffer at the beginning
    /// of each play.*
    ///
    /// This method will return an error if the image is not animated.
    pub(crate) fn set_blend_op(&mut self, op: BlendOp) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.blend_op = op;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Set the dispose operation for the following frames.
    ///
    /// The dispose operation specifies how the output buffer should be changed
    /// at the end of the delay (before rendering the next frame)
    ///
    /// See the [`DisposeOp`] documentation for the possible values and their effects.
    ///
    /// *Note that if the first frame uses [`DisposeOp::Previous`]
    /// it will be treated as [`DisposeOp::Background`].*
    ///
    /// This method will return an error if the image is not animated.
    pub(crate) fn set_dispose_op(&mut self, op: DisposeOp) -> Result<()> {
        if let Some(ref mut fctl) = self.info.frame_control {
            fctl.dispose_op = op;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Create a stream writer.
    ///
    /// This allows you to create images that do not fit in memory. The default
    /// chunk size is 4K, use `stream_writer_with_size` to set another chunk
    /// size.
    ///
    /// This borrows the writer which allows for manually appending additional
    /// chunks after the image data has been written.
    pub(crate) fn stream_writer(&mut self) -> Result<StreamWriter<'_, W>> {
        self.stream_writer_with_size(DEFAULT_BUFFER_LENGTH)
    }

    /// Create a stream writer with custom buffer size.
    ///
    /// See [`stream_writer`].
    ///
    /// [`stream_writer`]: Self::stream_writer
    pub(crate) fn stream_writer_with_size(&mut self, size: usize) -> Result<StreamWriter<'_, W>> {
        StreamWriter::new(ChunkOutput::Borrowed(self), size)
    }

    /// Turn this into a stream writer for image data.
    ///
    /// This allows you to create images that do not fit in memory. The default
    /// chunk size is 4K, use [`stream_writer_with_size`] to set another chunk
    /// size.
    ///
    /// [`stream_writer_with_size`]: Self::stream_writer_with_size
    pub(crate) fn into_stream_writer(self) -> Result<StreamWriter<'static, W>> {
        self.into_stream_writer_with_size(DEFAULT_BUFFER_LENGTH)
    }

    /// Turn this into a stream writer with custom buffer size.
    ///
    /// See [`into_stream_writer`].
    ///
    /// [`into_stream_writer`]: Self::into_stream_writer
    pub(crate) fn into_stream_writer_with_size(
        self,
        size: usize,
    ) -> Result<StreamWriter<'static, W>> {
        StreamWriter::new(ChunkOutput::Owned(self), size)
    }

    /// Consume the stream writer with validation.
    ///
    /// Unlike a simple drop this ensures that the final chunk was written correctly. When other
    /// validation options (chunk sequencing) had been turned on in the configuration then it will
    /// also do a check on their correctness _before_ writing the final chunk.
    pub(crate) fn finish(mut self) -> Result<()> {
        self.validate_sequence_done()?;
        self.write_iend()?;
        self.w.flush()?;

        // Explicitly drop `self` just for clarity.
        drop(self);
        Ok(())
    }
}

impl<W: Write> Drop for Writer<W> {
    fn drop(&mut self) {
        if !self.iend_written {
            drop(self.write_iend());
        }
    }
}
