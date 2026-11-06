use super::*;

impl StreamingDecoder {
    pub(super) fn parse_plte(&mut self) -> Result<(), DecodingError> {
        if info_ref(&self.info)?.palette.is_some() {
            // Only one palette is allowed
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::PLTE }.into(),
            ));
        }

        let palette = self.copy_current_chunk_raw_bytes("PNG PLTE chunk copy")?;
        info_mut(&mut self.info)?.palette = Some(Cow::Owned(palette));
        Ok(())
    }

    pub(super) fn parse_sbit(&mut self) -> Result<(), DecodingError> {
        let info = info_ref(&self.info)?;
        if info.palette.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterPlte { kind: chunk::S_BIT }.into(),
            ));
        }

        if self.have_idat {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::S_BIT }.into(),
            ));
        }

        if info.sbit.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::S_BIT }.into(),
            ));
        }

        let (color_type, bit_depth) = { (info.color_type, info.bit_depth) };
        // The sample depth for color type 3 is fixed at eight bits.
        let sample_depth = if color_type == ColorType::Indexed {
            BitDepth::Eight
        } else {
            bit_depth
        };
        let vec = self.copy_current_chunk_raw_bytes("PNG sBIT chunk copy")?;
        let len = vec.len();

        // expected lenth of the chunk
        let expected = match color_type {
            ColorType::Grayscale => 1,
            ColorType::Rgb | ColorType::Indexed => 3,
            ColorType::GrayscaleAlpha => 2,
            ColorType::Rgba => 4,
        };

        // Check if the sbit chunk size is valid.
        if expected != len {
            return Err(DecodingError::Format(
                FormatErrorInner::InvalidSbitChunkSize {
                    color_type,
                    expected,
                    len,
                }
                .into(),
            ));
        }

        for sbit in &vec {
            if *sbit < 1 || *sbit > sample_depth as u8 {
                return Err(DecodingError::Format(
                    FormatErrorInner::InvalidSbit {
                        sample_depth,
                        sbit: *sbit,
                    }
                    .into(),
                ));
            }
        }
        info_mut(&mut self.info)?.sbit = Some(Cow::Owned(vec));
        Ok(())
    }

    pub(super) fn parse_trns(&mut self) -> Result<(), DecodingError> {
        let info = info_ref(&self.info)?;
        if info.trns.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::PLTE }.into(),
            ));
        }
        let (color_type, bit_depth) = { (info.color_type, info.bit_depth as u8) };
        let mut vec = self.copy_current_chunk_raw_bytes("PNG tRNS chunk copy")?;
        let len = vec.len();
        match color_type {
            ColorType::Grayscale => {
                if len < 2 {
                    return Err(DecodingError::Format(
                        FormatErrorInner::ShortPalette { expected: 2, len }.into(),
                    ));
                }
                if bit_depth < 16 {
                    vec[0] = vec[1];
                    vec.truncate(1);
                }
                info_mut(&mut self.info)?.trns = Some(Cow::Owned(vec));
                Ok(())
            }
            ColorType::Rgb => {
                if len < 6 {
                    return Err(DecodingError::Format(
                        FormatErrorInner::ShortPalette { expected: 6, len }.into(),
                    ));
                }
                if bit_depth < 16 {
                    vec[0] = vec[1];
                    vec[1] = vec[3];
                    vec[2] = vec[5];
                    vec.truncate(3);
                }
                info_mut(&mut self.info)?.trns = Some(Cow::Owned(vec));
                Ok(())
            }
            ColorType::Indexed => {
                // The transparency chunk must be after the palette chunk and
                // before the data chunk.
                if info_ref(&self.info)?.palette.is_none() {
                    return Err(DecodingError::Format(
                        FormatErrorInner::BeforePlte { kind: chunk::T_RNS }.into(),
                    ));
                } else if self.have_idat {
                    return Err(DecodingError::Format(
                        FormatErrorInner::OutsidePlteIdat { kind: chunk::T_RNS }.into(),
                    ));
                }

                info_mut(&mut self.info)?.trns = Some(Cow::Owned(vec));
                Ok(())
            }
            c => Err(DecodingError::Format(
                FormatErrorInner::ColorWithBadTrns(c).into(),
            )),
        }
    }
}
