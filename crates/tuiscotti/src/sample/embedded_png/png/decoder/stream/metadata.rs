use super::*;

impl StreamingDecoder {
    pub(super) fn parse_ihdr(&mut self) -> Result<(), DecodingError> {
        if self.info.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: IHDR }.into(),
            ));
        }
        let mut buf = &self.current_chunk.raw_bytes[..];
        let width = buf.read_be()?;
        let height = buf.read_be()?;
        if width == 0 || height == 0 {
            return Err(DecodingError::Format(
                FormatErrorInner::InvalidDimensions.into(),
            ));
        }
        let bit_depth = buf.read_be()?;
        let bit_depth = match BitDepth::from_u8(bit_depth) {
            Some(bits) => bits,
            None => {
                return Err(DecodingError::Format(
                    FormatErrorInner::InvalidBitDepth(bit_depth).into(),
                ));
            }
        };
        let color_type = buf.read_be()?;
        let color_type = match ColorType::from_u8(color_type) {
            Some(color_type) => {
                if color_type.is_combination_invalid(bit_depth) {
                    return Err(DecodingError::Format(
                        FormatErrorInner::InvalidColorBitDepth {
                            color_type,
                            bit_depth,
                        }
                        .into(),
                    ));
                } else {
                    color_type
                }
            }
            None => {
                return Err(DecodingError::Format(
                    FormatErrorInner::InvalidColorType(color_type).into(),
                ));
            }
        };
        match buf.read_be()? {
            // compression method
            0u8 => (),
            n => {
                return Err(DecodingError::Format(
                    FormatErrorInner::UnknownCompressionMethod(n).into(),
                ));
            }
        }
        match buf.read_be()? {
            // filter method
            0u8 => (),
            n => {
                return Err(DecodingError::Format(
                    FormatErrorInner::UnknownFilterMethod(n).into(),
                ));
            }
        }
        let interlaced = match buf.read_be()? {
            0u8 => false,
            1 => true,
            n => {
                return Err(DecodingError::Format(
                    FormatErrorInner::UnknownInterlaceMethod(n).into(),
                ));
            }
        };

        self.info = Some(Info {
            width,
            height,
            bit_depth,
            color_type,
            interlaced,
            ..Default::default()
        });

        Ok(())
    }

    fn split_keyword(buf: &[u8]) -> Result<(&[u8], &[u8]), DecodingError> {
        let null_byte_index = buf
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| DecodingError::from(TextDecodingError::MissingNullSeparator))?;

        if null_byte_index == 0 || null_byte_index > 79 {
            return Err(DecodingError::from(TextDecodingError::InvalidKeywordSize));
        }

        Ok((&buf[..null_byte_index], &buf[null_byte_index + 1..]))
    }

    pub(super) fn parse_text(&mut self) -> Result<(), DecodingError> {
        let buf = &self.current_chunk.raw_bytes[..];
        self.limits.reserve_bytes(buf.len())?;

        let (keyword_slice, value_slice) = Self::split_keyword(buf)?;
        let value = TEXtChunk::decode(keyword_slice, value_slice, &mut self.limits)?;
        let entries = &mut info_mut(&mut self.info)?.uncompressed_latin1_text;
        let growth = std::mem::size_of::<TEXtChunk>();
        self.limits.reserve_bytes(growth)?;
        entries.try_reserve_exact(1).map_err(|source| {
            DecodingError::allocation("PNG tEXt metadata list", growth, source)
        })?;
        entries.push(value);

        Ok(())
    }

    pub(super) fn parse_ztxt(&mut self) -> Result<(), DecodingError> {
        let buf = &self.current_chunk.raw_bytes[..];
        self.limits.reserve_bytes(buf.len())?;

        let (keyword_slice, value_slice) = Self::split_keyword(buf)?;

        let compression_method = *value_slice
            .first()
            .ok_or_else(|| DecodingError::from(TextDecodingError::InvalidCompressionMethod))?;

        let text_slice = &value_slice[1..];

        let value = ZTXtChunk::decode(
            keyword_slice,
            compression_method,
            text_slice,
            &mut self.limits,
        )?;
        let entries = &mut info_mut(&mut self.info)?.compressed_latin1_text;
        let growth = std::mem::size_of::<ZTXtChunk>();
        self.limits.reserve_bytes(growth)?;
        entries.try_reserve_exact(1).map_err(|source| {
            DecodingError::allocation("PNG zTXt metadata list", growth, source)
        })?;
        entries.push(value);

        Ok(())
    }

    pub(super) fn parse_itxt(&mut self) -> Result<(), DecodingError> {
        let buf = &self.current_chunk.raw_bytes[..];
        self.limits.reserve_bytes(buf.len())?;

        let (keyword_slice, value_slice) = Self::split_keyword(buf)?;

        let compression_flag = *value_slice
            .first()
            .ok_or_else(|| DecodingError::from(TextDecodingError::MissingCompressionFlag))?;

        let compression_method = *value_slice
            .get(1)
            .ok_or_else(|| DecodingError::from(TextDecodingError::InvalidCompressionMethod))?;

        let second_null_byte_index = value_slice[2..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| DecodingError::from(TextDecodingError::MissingNullSeparator))?
            + 2;

        let language_tag_slice = &value_slice[2..second_null_byte_index];

        let third_null_byte_index = value_slice[second_null_byte_index + 1..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| DecodingError::from(TextDecodingError::MissingNullSeparator))?
            + (second_null_byte_index + 1);

        let translated_keyword_slice =
            &value_slice[second_null_byte_index + 1..third_null_byte_index];

        let text_slice = &value_slice[third_null_byte_index + 1..];

        let value = ITXtChunk::decode(
            keyword_slice,
            compression_flag,
            compression_method,
            language_tag_slice,
            translated_keyword_slice,
            text_slice,
            &mut self.limits,
        )?;
        let entries = &mut info_mut(&mut self.info)?.utf8_text;
        let growth = std::mem::size_of::<ITXtChunk>();
        self.limits.reserve_bytes(growth)?;
        entries.try_reserve_exact(1).map_err(|source| {
            DecodingError::allocation("PNG iTXt metadata list", growth, source)
        })?;
        entries.push(value);

        Ok(())
    }

    pub(super) fn parse_bkgd(&mut self) -> Result<(), DecodingError> {
        let info = info_ref(&self.info)?;
        if info.bkgd.is_some() {
            // Only one bKGD chunk is allowed
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::B_KGD }.into(),
            ));
        } else if self.have_idat {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::B_KGD }.into(),
            ));
        }

        let expected = match info.color_type {
            ColorType::Indexed => {
                if info.palette.is_none() {
                    return Err(DecodingError::IoError(
                        std::io::ErrorKind::InvalidData.into(),
                    ));
                };
                1
            }
            ColorType::Grayscale | ColorType::GrayscaleAlpha => 2,
            ColorType::Rgb | ColorType::Rgba => 6,
        };
        let vec = self.copy_current_chunk_raw_bytes("PNG bKGD chunk copy")?;
        if vec.len() != expected {
            return Err(DecodingError::Format(
                FormatErrorInner::ChunkLengthWrong { kind: chunk::B_KGD }.into(),
            ));
        }

        info_mut(&mut self.info)?.bkgd = Some(Cow::Owned(vec));
        Ok(())
    }
}
