use super::*;

/// Streaming PNG writer
///
/// This may silently fail in the destructor, so it is a good idea to call
/// [`finish`] or [`flush`] before dropping.
///
/// [`finish`]: Self::finish
/// [`flush`]: Write::flush

pub(crate) struct StreamWriter<'a, W: Write> {
    /// The option here is needed in order to access the inner `ChunkWriter` in-between
    /// each frame, which is needed for writing the fcTL chunks between each frame
    writer: Wrapper<'a, W>,
    prev_buf: Vec<u8>,
    curr_buf: Vec<u8>,
    filtered_buf: Vec<u8>,
    /// Amount of data already written
    index: usize,
    /// length of the current scanline
    line_len: usize,
    /// size of the frame (width * height * sample_size)
    to_write: usize,

    width: u32,
    height: u32,

    bpp: BytesPerPixel,
    filter: Filter,
    fctl: Option<FrameControl>,
    compression: DeflateCompression,
}

impl<'a, W: Write> StreamWriter<'a, W> {
    pub(in crate::sample::embedded_png::png::encoder) fn new(
        writer: ChunkOutput<'a, W>,
        buf_len: usize,
    ) -> Result<StreamWriter<'a, W>> {
        let PartialInfo {
            width,
            height,
            frame_control: fctl,
            ..
        } = writer.info;

        let bpp = writer.info.bpp_in_prediction();
        let in_len = writer.info.raw_row_length() - 1;
        let filter = writer.options.filter;
        let compression = writer.options.compression;
        let prev_buf = vec![0; in_len];
        let curr_buf = vec![0; in_len];
        let filtered_buf = vec![0; in_len];

        let mut chunk_writer = ChunkWriter::new(writer, buf_len);
        let (line_len, to_write) = chunk_writer.next_frame_info();
        chunk_writer.write_header()?;

        Ok(StreamWriter {
            writer: Wrapper::from_level(chunk_writer, compression)?,
            index: 0,
            prev_buf,
            curr_buf,
            filtered_buf,
            bpp,
            filter,
            width,
            height,
            line_len,
            to_write,
            fctl,
            compression,
        })
    }

    /// Set the used filter type.
    ///
    /// The default filter is [`Filter::Adaptive`] which automatically selects the best filter
    /// for each row of the image.
    ///
    /// You should only change this if you are after very fast compression,
    /// and either don't care about compression ratio or know exactly what works best for your images.
    pub(crate) fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
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
        if let Some(ref mut fctl) = self.fctl {
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
    pub(crate) fn set_frame_dimension(&mut self, width: u32, height: u32) -> Result<()> {
        if let Some(ref mut fctl) = self.fctl {
            if Some(width) > self.width.checked_sub(fctl.x_offset)
                || Some(height) > self.height.checked_sub(fctl.y_offset)
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
    pub(crate) fn set_frame_position(&mut self, x: u32, y: u32) -> Result<()> {
        if let Some(ref mut fctl) = self.fctl {
            if Some(x) > self.width.checked_sub(fctl.width)
                || Some(y) > self.height.checked_sub(fctl.height)
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
        if let Some(ref mut fctl) = self.fctl {
            fctl.width = self.width - fctl.x_offset;
            fctl.height = self.height - fctl.y_offset;
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
        if let Some(ref mut fctl) = self.fctl {
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
        if let Some(ref mut fctl) = self.fctl {
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
        if let Some(ref mut fctl) = self.fctl {
            fctl.dispose_op = op;
            Ok(())
        } else {
            Err(EncodingError::Format(FormatErrorKind::NotAnimated.into()))
        }
    }

    /// Consume the stream writer with validation.
    ///
    /// Unlike a simple drop this ensures that the all data was written correctly. When other
    /// validation options (chunk sequencing) had been turned on in the configuration of inner
    /// [`Writer`], then it will also do a check on their correctness. Differently from
    /// [`Writer::finish`], this just `flush`es, returns error if some data is abandoned.
    pub(crate) fn finish(mut self) -> Result<()> {
        self.finish_mut()
    }

    /// Internal helper that can be called both from `fn finish(mut self)`
    /// and from `fn drop(&mut self)`.
    fn finish_mut(&mut self) -> Result<()> {
        if self.to_write > 0 {
            let err = FormatErrorKind::MissingData(self.to_write).into();
            return Err(EncodingError::Format(err));
        }

        self.flush()?;
        match self.writer.take() {
            Wrapper::Chunk(wrt) => {
                wrt.writer.validate_sequence_done()?;
            }
            Wrapper::FDeflate(wrt) => {
                wrt.finish()?;
            }
            Wrapper::Flate2(wrt) => {
                wrt.finish()?;
            }
            Wrapper::None => unreachable!(),
            Wrapper::Unrecoverable => {
                let err = FormatErrorKind::Unrecoverable.into();
                return Err(EncodingError::Format(err));
            }
        }

        Ok(())
    }

    /// Flushes the buffered chunk, checks if it was the last frame,
    /// writes the next frame header and gets the next frame scanline size
    /// and image size.
    /// NOTE: This method must only be called when the writer is the variant Chunk(_)
    fn new_frame(&mut self) -> Result<()> {
        let wrt = match &mut self.writer {
            Wrapper::Chunk(wrt) => wrt,
            Wrapper::Unrecoverable => {
                let err = FormatErrorKind::Unrecoverable.into();
                return Err(EncodingError::Format(err));
            }
            Wrapper::Flate2(_) | Wrapper::FDeflate(_) => {
                unreachable!("never called on a half-finished frame")
            }
            Wrapper::None => unreachable!(),
        };
        wrt.flush()?;
        wrt.writer.validate_new_image()?;

        if let Some(fctl) = self.fctl {
            wrt.set_fctl(fctl);
        }
        let (scansize, size) = wrt.next_frame_info();
        self.line_len = scansize;
        self.to_write = size;

        wrt.write_header()?;
        wrt.writer.increment_images_written();

        // now it can be taken because the next statements cannot cause any errors
        match self.writer.take() {
            Wrapper::Chunk(wrt) => match Wrapper::from_level(wrt, self.compression) {
                Ok(writer) => self.writer = writer,
                Err(err) => {
                    self.writer = Wrapper::Unrecoverable;
                    return Err(err.into());
                }
            },
            _ => unreachable!(),
        };

        Ok(())
    }
}

mod write;
