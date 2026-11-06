use super::*;

impl<R: BufRead + Seek> Reader<R> {
    /// Returns the next processed row of the image (discarding [`InterlaceInfo`]).
    ///
    /// See also [`Reader::read_row`], which reads into a caller-provided buffer.

    pub(crate) fn next_row(&mut self) -> Result<Option<Row<'_>>, DecodingError> {
        self.next_interlaced_row()
            .map(|v| v.map(|v| Row { data: v.data }))
    }

    /// Returns the next processed row of the image.
    ///
    /// See also [`Reader::read_row`], which reads into a caller-provided buffer.
    pub(crate) fn next_interlaced_row(
        &mut self,
    ) -> Result<Option<InterlacedRow<'_>>, DecodingError> {
        let mut output_buffer = mem::take(&mut self.scratch_buffer);
        let width = self.info()?.width;
        let max_line_size = self
            .output_line_size(width)?
            .ok_or(DecodingError::LimitsExceeded)?;
        if max_line_size > output_buffer.len() {
            output_buffer
                .try_reserve_exact(max_line_size - output_buffer.len())
                .map_err(|source| {
                    DecodingError::allocation(
                        "PNG processed output row",
                        max_line_size - output_buffer.len(),
                        source,
                    )
                })?;
        }
        output_buffer.resize(max_line_size, 0u8);
        let result = self.read_row(&mut output_buffer);
        self.scratch_buffer = output_buffer;
        match result? {
            Some(interlace) => {
                let output_line_size = self.output_line_size_for_interlace_info(&interlace)?;
                Ok(Some(InterlacedRow {
                    data: &self.scratch_buffer[..output_line_size],
                    interlace,
                }))
            }
            None => Ok(None),
        }
    }

    /// Reads the next row of the image into the provided `output_buffer`.
    /// `Ok(None)` will be returned if the current image frame has no more rows.
    ///
    /// `output_buffer` needs to be long enough to accommodate [`Reader::output_line_size`] for
    /// [`Info::width`] (initial interlaced rows may need less than that).
    ///
    /// See also [`Reader::next_row`] and [`Reader::next_interlaced_row`], which read into a
    /// `Reader`-owned buffer.
    pub(crate) fn read_row(
        &mut self,
        output_buffer: &mut [u8],
    ) -> Result<Option<InterlaceInfo>, DecodingError> {
        let interlace = match self.subframe.current_interlace_info.as_ref() {
            None => {
                self.finish_decoding()?;
                return Ok(None);
            }
            Some(interlace) => *interlace,
        };
        if interlace.line_number() == 0 {
            self.unfiltering_buffer.reset_prev_row();
        }
        let rowlen = match interlace {
            InterlaceInfo::Null(_) => self.subframe.rowlen,
            InterlaceInfo::Adam7(Adam7Info { samples: width, .. }) => {
                self.info()?.raw_row_length_from_width(width)
            }
        };

        let output_line_size = self.output_line_size_for_interlace_info(&interlace)?;
        let output_buffer = &mut output_buffer[..output_line_size];

        self.next_interlaced_row_impl(rowlen, output_buffer)?;

        Ok(Some(interlace))
    }

    pub(super) fn output_line_size_for_interlace_info(
        &self,
        interlace: &InterlaceInfo,
    ) -> Result<usize, DecodingError> {
        let width = match interlace {
            InterlaceInfo::Adam7(Adam7Info { samples: width, .. }) => *width,
            InterlaceInfo::Null(_) => self.subframe.width,
        };
        self.unguarded_output_line_size(width)
    }

    /// Read the rest of the image and chunks and finish up, including text chunks or others
    /// This will discard the rest of the image if the image is not read already with [`Reader::next_frame`], [`Reader::next_row`] or [`Reader::next_interlaced_row`]
    pub(crate) fn finish(&mut self) -> Result<(), DecodingError> {
        if self.finished {
            return Err(DecodingError::Parameter(
                ParameterErrorKind::PolledAfterEndOfImage.into(),
            ));
        }

        self.remaining_frames = 0;
        self.decoder.read_until_end_of_input()?;

        self.finished = true;
        Ok(())
    }

    /// Fetch the next interlaced row and filter it according to our own transformations.
    pub(super) fn next_interlaced_row_impl(
        &mut self,
        rowlen: usize,
        output_buffer: &mut [u8],
    ) -> Result<(), DecodingError> {
        self.next_raw_interlaced_row(rowlen)?;
        let row = self.unfiltering_buffer.prev_row();
        assert_eq!(row.len(), rowlen - 1);

        // Apply transformations and write resulting data to buffer.
        let transform_fn = {
            if self.transform_fn.is_none() {
                self.transform_fn = Some(create_transform_fn(self.info()?, self.transform)?);
            }
            self.transform_fn
                .as_ref()
                .ok_or(DecodingError::InvalidState)?
        };
        transform_fn.try_apply(row, output_buffer, self.info()?)?;

        self.subframe.current_interlace_info = self.subframe.interlace_info_iter.next();
        Ok(())
    }
}
