use super::*;

impl<W: Write> Writer<W> {
    pub(in crate::sample::embedded_png::png::encoder) fn new(
        w: W,
        info: PartialInfo,
        options: Options,
    ) -> Writer<W> {
        Writer {
            w,
            info,
            options,
            images_written: 0,
            animation_written: 0,
            iend_written: false,
        }
    }

    pub(in crate::sample::embedded_png::png::encoder) fn init(
        mut self,
        info: &Info<'_>,
    ) -> Result<Self> {
        if self.info.width == 0 {
            return Err(EncodingError::Format(FormatErrorKind::ZeroWidth.into()));
        }

        if self.info.height == 0 {
            return Err(EncodingError::Format(FormatErrorKind::ZeroHeight.into()));
        }

        if self
            .info
            .color_type
            .is_combination_invalid(self.info.bit_depth)
        {
            return Err(EncodingError::Format(
                FormatErrorKind::InvalidColorCombination(self.info.bit_depth, self.info.color_type)
                    .into(),
            ));
        }

        self.encode_header(info)?;

        Ok(self)
    }

    /// Encode PNG signature, IHDR, and then chunks that were added to the `Info`
    pub(super) fn encode_header(&mut self, info: &Info<'_>) -> Result<()> {
        self.w.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?; // PNG signature

        // Encode the IHDR chunk
        let mut data = [0; 13];
        data[..4].copy_from_slice(&info.width.to_be_bytes());
        data[4..8].copy_from_slice(&info.height.to_be_bytes());
        data[8] = info.bit_depth as u8;
        data[9] = info.color_type as u8;
        data[12] = info.interlaced as u8;
        self.write_chunk(chunk::IHDR, &data)?;

        // Encode the pHYs chunk
        if let Some(pd) = info.pixel_dims {
            let mut phys_data = [0; 9];
            phys_data[0..4].copy_from_slice(&pd.xppu.to_be_bytes());
            phys_data[4..8].copy_from_slice(&pd.yppu.to_be_bytes());
            match pd.unit {
                Unit::Meter => phys_data[8] = 1,
                Unit::Unspecified => phys_data[8] = 0,
            }
            self.write_chunk(chunk::P_HYS, &phys_data)?;
        }

        // If specified, the sRGB information overrides the source gamma and chromaticities.
        if let Some(srgb) = &info.srgb {
            srgb.encode(&mut self.w)?;

            // gAMA and cHRM are optional, for backwards compatibility
            let srgb_gamma = crate::sample::embedded_png::png::srgb::substitute_gamma();
            if Some(srgb_gamma) == info.source_gamma {
                srgb_gamma.encode_gama(&mut self.w)?
            }
            let srgb_chromaticities =
                crate::sample::embedded_png::png::srgb::substitute_chromaticities();
            if Some(srgb_chromaticities) == info.source_chromaticities {
                srgb_chromaticities.encode(&mut self.w)?;
            }
        } else {
            if let Some(gma) = info.source_gamma {
                gma.encode_gama(&mut self.w)?
            }
            if let Some(chrms) = info.source_chromaticities {
                chrms.encode(&mut self.w)?;
            }
            if let Some(iccp) = &info.icc_profile {
                self.write_iccp_chunk("_", iccp)?
            }
        }

        if let Some(exif) = &info.exif_metadata {
            self.write_chunk(chunk::E_XIF, exif)?;
        }

        if let Some(actl) = info.animation_control {
            actl.encode(&mut self.w)?;
        }

        // The position of the PLTE chunk is important, it must come before the tRNS chunk and after
        // many of the other metadata chunks.
        if let Some(p) = &info.palette {
            self.write_chunk(chunk::PLTE, p)?;
        };

        if let Some(t) = &info.trns {
            self.write_chunk(chunk::T_RNS, t)?;
        }

        for text_chunk in &info.uncompressed_latin1_text {
            self.write_text_chunk(text_chunk)?;
        }

        for text_chunk in &info.compressed_latin1_text {
            self.write_text_chunk(text_chunk)?;
        }

        for text_chunk in &info.utf8_text {
            self.write_text_chunk(text_chunk)?;
        }

        Ok(())
    }

    /// Write a raw chunk of PNG data.
    ///
    /// This function calculates the required CRC sum so this should not be included in the input
    /// `data`, otherwise the data is not filtered in any way. This function returns an error if
    /// the length of `data` can't be parsed as a `u32` though the length of the chunk data should
    /// not exceed `i32::MAX` or 2,147,483,647.
    pub(crate) fn write_chunk(&mut self, name: ChunkType, data: &[u8]) -> Result<()> {
        use std::convert::TryFrom;

        if u32::try_from(data.len()).map_or(true, |length| length > i32::MAX as u32) {
            let kind = FormatErrorKind::WrittenTooMuch(data.len() - i32::MAX as usize);
            return Err(EncodingError::Format(kind.into()));
        }

        write_chunk(&mut self.w, name, data)
    }

    pub(crate) fn write_text_chunk<T: EncodableTextChunk>(&mut self, text_chunk: &T) -> Result<()> {
        text_chunk.encode(&mut self.w)
    }

    pub(super) fn write_iccp_chunk(
        &mut self,
        profile_name: &str,
        icc_profile: &[u8],
    ) -> Result<()> {
        let profile_name = encode_iso_8859_1(profile_name)?;
        if profile_name.is_empty() || profile_name.len() > 79 {
            return Err(TextEncodingError::InvalidKeywordSize.into());
        }

        let estimated_compressed_size = icc_profile.len() * 3 / 4;
        let chunk_size = profile_name
            .len()
            .checked_add(2) // string NUL + compression type. Checked add optimizes out later Vec reallocations.
            .and_then(|s| s.checked_add(estimated_compressed_size))
            .ok_or(EncodingError::LimitsExceeded)?;

        let mut data = Vec::new();
        data.try_reserve_exact(chunk_size)
            .map_err(|_| EncodingError::LimitsExceeded)?;

        data.extend(profile_name.into_iter().chain([0, 0]));

        let mut encoder = ZlibEncoder::new(data, flate2::Compression::default());
        encoder.write_all(icc_profile)?;

        self.write_chunk(chunk::I_CCP, &encoder.finish()?)
    }
}
