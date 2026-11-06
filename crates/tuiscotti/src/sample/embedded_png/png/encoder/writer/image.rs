use super::*;

impl<W: Write> Writer<W> {
    /// Check if we should allow writing another image.

    pub(in crate::sample::embedded_png::png::encoder) fn validate_new_image(&self) -> Result<()> {
        if !self.options.validate_sequence {
            return Ok(());
        }

        match self.info.animation_control {
            None => {
                if self.images_written == 0 {
                    Ok(())
                } else {
                    Err(EncodingError::Format(FormatErrorKind::EndReached.into()))
                }
            }
            Some(_) => {
                if self.info.frame_control.is_some() {
                    Ok(())
                } else {
                    Err(EncodingError::Format(FormatErrorKind::EndReached.into()))
                }
            }
        }
    }

    pub(in crate::sample::embedded_png::png::encoder) fn validate_sequence_done(
        &self,
    ) -> Result<()> {
        if !self.options.validate_sequence {
            return Ok(());
        }

        if (self.info.animation_control.is_some() && self.info.frame_control.is_some())
            || self.images_written == 0
        {
            Err(EncodingError::Format(FormatErrorKind::MissingFrames.into()))
        } else {
            Ok(())
        }
    }

    const MAX_IDAT_CHUNK_LEN: u32 = u32::MAX >> 1;
    const MAX_FDAT_CHUNK_LEN: u32 = (u32::MAX >> 1) - 4;

    /// Writes the next image data.
    pub(crate) fn write_image_data(&mut self, data: &[u8]) -> Result<()> {
        if self.info.color_type == ColorType::Indexed && !self.info.has_palette {
            return Err(EncodingError::Format(FormatErrorKind::NoPalette.into()));
        }

        self.validate_new_image()?;

        let width: usize;
        let height: usize;
        if let Some(ref mut fctl) = self.info.frame_control {
            width = fctl.width as usize;
            height = fctl.height as usize;
        } else {
            width = self.info.width as usize;
            height = self.info.height as usize;
        }

        let in_len = self.info.raw_row_length_from_width(width as u32) - 1;
        let data_size = in_len * height;
        if data_size != data.len() {
            return Err(EncodingError::Parameter(
                ParameterErrorKind::ImageBufferSize {
                    expected: data_size,
                    actual: data.len(),
                }
                .into(),
            ));
        }

        let prev = vec![0; in_len];
        let mut prev = prev.as_slice();

        let bpp = self.info.bpp_in_prediction();
        let filter_method = self.options.filter;

        let zlib_encoded = match self.options.compression {
            DeflateCompression::NoCompression => {
                let mut compressor =
                    crate::sample::embedded_png::fdeflate::StoredOnlyCompressor::new(
                        std::io::Cursor::new(Vec::new()),
                    )?;
                for line in data.chunks(in_len) {
                    compressor.write_data(&[0])?;
                    compressor.write_data(line)?;
                }
                compressor.finish()?.into_inner()
            }
            DeflateCompression::FdeflateUltraFast => {
                let mut compressor = crate::sample::embedded_png::fdeflate::Compressor::new(
                    std::io::Cursor::new(Vec::new()),
                )?;

                let mut current = vec![0; in_len + 1];
                for line in data.chunks(in_len) {
                    let filter_type = filter(filter_method, bpp, prev, line, &mut current[1..]);

                    current[0] = filter_type as u8;
                    compressor.write_data(&current)?;
                    prev = line;
                }

                let compressed = compressor.finish()?.into_inner();
                if compressed.len()
                    > crate::sample::embedded_png::fdeflate::StoredOnlyCompressor::<()>::compressed_size((in_len + 1) * height)
                {
                    // Write uncompressed data since the result from fast compression would take
                    // more space than that.
                    //
                    // This is essentially a fallback to NoCompression.
                    let mut compressor =
                        crate::sample::embedded_png::fdeflate::StoredOnlyCompressor::new(std::io::Cursor::new(Vec::new()))?;
                    for line in data.chunks(in_len) {
                        compressor.write_data(&[0])?;
                        compressor.write_data(line)?;
                    }
                    compressor.finish()?.into_inner()
                } else {
                    compressed
                }
            }
            DeflateCompression::Level(level) => {
                let mut current = vec![0; in_len];

                let mut zlib =
                    ZlibEncoder::new(Vec::new(), flate2::Compression::new(u32::from(level)));
                for line in data.chunks(in_len) {
                    let filter_type = filter(filter_method, bpp, prev, line, &mut current);

                    zlib.write_all(&[filter_type as u8])?;
                    zlib.write_all(&current)?;
                    prev = line;
                }
                zlib.finish()?
            }
        };

        match self.info.frame_control {
            None => {
                self.write_zlib_encoded_idat(&zlib_encoded)?;
            }
            Some(_) if self.should_skip_frame_control_on_default_image() => {
                self.write_zlib_encoded_idat(&zlib_encoded)?;
            }
            Some(ref mut fctl) => {
                fctl.encode(&mut self.w)?;
                fctl.sequence_number = fctl.sequence_number.wrapping_add(1);
                self.animation_written += 1;

                // If the default image is the first frame of an animation, it's still an IDAT.
                if self.images_written == 0 {
                    self.write_zlib_encoded_idat(&zlib_encoded)?;
                } else {
                    let buff_size = zlib_encoded.len().min(Self::MAX_FDAT_CHUNK_LEN as usize);
                    let mut alldata = vec![0u8; 4 + buff_size];
                    for chunk in zlib_encoded.chunks(Self::MAX_FDAT_CHUNK_LEN as usize) {
                        alldata[..4].copy_from_slice(&fctl.sequence_number.to_be_bytes());
                        alldata[4..][..chunk.len()].copy_from_slice(chunk);
                        write_chunk(&mut self.w, chunk::FD_AT, &alldata[..4 + chunk.len()])?;
                        fctl.sequence_number = fctl.sequence_number.wrapping_add(1);
                    }
                }
            }
        }

        self.increment_images_written();

        Ok(())
    }

    pub(in crate::sample::embedded_png::png::encoder) fn increment_images_written(&mut self) {
        self.images_written = self.images_written.saturating_add(1);

        if let Some(actl) = self.info.animation_control {
            if actl.num_frames <= self.animation_written {
                // If we've written all animation frames, all following will be normal image chunks.
                self.info.frame_control = None;
            }
        }
    }

    pub(super) fn write_iend(&mut self) -> Result<()> {
        self.iend_written = true;
        self.write_chunk(chunk::IEND, &[])
    }

    pub(in crate::sample::embedded_png::png::encoder) fn should_skip_frame_control_on_default_image(
        &self,
    ) -> bool {
        self.options.sep_def_img && self.images_written == 0
    }

    pub(super) fn write_zlib_encoded_idat(&mut self, zlib_encoded: &[u8]) -> Result<()> {
        for chunk in zlib_encoded.chunks(Self::MAX_IDAT_CHUNK_LEN as usize) {
            self.write_chunk(chunk::IDAT, chunk)?;
        }
        Ok(())
    }
}
