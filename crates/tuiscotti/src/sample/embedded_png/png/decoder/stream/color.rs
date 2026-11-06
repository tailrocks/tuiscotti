use super::*;

impl StreamingDecoder {
    pub(super) fn parse_cicp(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;

        // The spec requires that the cICP chunk MUST come before the PLTE and IDAT chunks.
        if info.coding_independent_code_points.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::C_ICP }.into(),
            ));
        } else if info.palette.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterPlte { kind: chunk::C_ICP }.into(),
            ));
        } else if self.have_idat {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::C_ICP }.into(),
            ));
        }

        let mut buf = &*self.current_chunk.raw_bytes;
        let color_primaries: u8 = buf.read_be()?;
        let transfer_function: u8 = buf.read_be()?;
        let matrix_coefficients: u8 = buf.read_be()?;
        let is_video_full_range_image = {
            let flag: u8 = buf.read_be()?;
            match flag {
                0 => false,
                1 => true,
                _ => {
                    return Err(DecodingError::IoError(
                        std::io::ErrorKind::InvalidData.into(),
                    ));
                }
            }
        };

        // RGB is currently the only supported color model in PNG, and as
        // such Matrix Coefficients shall be set to 0.
        if matrix_coefficients != 0 {
            return Err(DecodingError::IoError(
                std::io::ErrorKind::InvalidData.into(),
            ));
        }

        if !buf.is_empty() {
            return Err(DecodingError::IoError(
                std::io::ErrorKind::InvalidData.into(),
            ));
        }

        info.coding_independent_code_points = Some(CodingIndependentCodePoints {
            color_primaries,
            transfer_function,
            matrix_coefficients,
            is_video_full_range_image,
        });

        Ok(())
    }

    pub(super) fn parse_mdcv(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;

        // The spec requires that the mDCV chunk MUST come before the PLTE and IDAT chunks.
        if info.mastering_display_color_volume.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::M_DCV }.into(),
            ));
        } else if info.palette.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterPlte { kind: chunk::M_DCV }.into(),
            ));
        } else if self.have_idat {
            return Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::M_DCV }.into(),
            ));
        }

        let mut buf = &*self.current_chunk.raw_bytes;
        let red_x: u16 = buf.read_be()?;
        let red_y: u16 = buf.read_be()?;
        let green_x: u16 = buf.read_be()?;
        let green_y: u16 = buf.read_be()?;
        let blue_x: u16 = buf.read_be()?;
        let blue_y: u16 = buf.read_be()?;
        let white_x: u16 = buf.read_be()?;
        let white_y: u16 = buf.read_be()?;
        fn scale(chunk: u16) -> ScaledFloat {
            // `ScaledFloat::SCALING` is hardcoded to 100_000, which works
            // well for the `cHRM` chunk where the spec says that "a value
            // of 0.3127 would be stored as the integer 31270".  In the
            // `mDCV` chunk the spec says that "0.708, 0.292)" is stored as
            // "{ 35400, 14600 }", using a scaling factor of 50_000, so we
            // multiply by 2 before converting.
            ScaledFloat::from_scaled((chunk as u32) * 2)
        }
        let chromaticities = SourceChromaticities {
            white: (scale(white_x), scale(white_y)),
            red: (scale(red_x), scale(red_y)),
            green: (scale(green_x), scale(green_y)),
            blue: (scale(blue_x), scale(blue_y)),
        };
        let max_luminance: u32 = buf.read_be()?;
        let min_luminance: u32 = buf.read_be()?;
        if !buf.is_empty() {
            return Err(DecodingError::IoError(
                std::io::ErrorKind::InvalidData.into(),
            ));
        }
        info.mastering_display_color_volume = Some(MasteringDisplayColorVolume {
            chromaticities,
            max_luminance,
            min_luminance,
        });

        Ok(())
    }

    pub(super) fn parse_clli(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if info.content_light_level.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::C_LLI }.into(),
            ));
        }

        let mut buf = &*self.current_chunk.raw_bytes;
        let max_content_light_level: u32 = buf.read_be()?;
        let max_frame_average_light_level: u32 = buf.read_be()?;
        if !buf.is_empty() {
            return Err(DecodingError::IoError(
                std::io::ErrorKind::InvalidData.into(),
            ));
        }
        info.content_light_level = Some(ContentLightLevelInfo {
            max_content_light_level,
            max_frame_average_light_level,
        });

        Ok(())
    }

    pub(super) fn parse_exif(&mut self) -> Result<(), DecodingError> {
        let info = info_ref(&self.info)?;
        if info.exif_metadata.is_some() {
            return Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::E_XIF }.into(),
            ));
        }

        let metadata = self.copy_current_chunk_raw_bytes("PNG eXIf chunk copy")?;
        info_mut(&mut self.info)?.exif_metadata = Some(metadata.into());
        Ok(())
    }

    pub(super) fn parse_iccp(&mut self) -> Result<(), DecodingError> {
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::I_CCP }.into(),
            ))
        } else if self.have_iccp {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::I_CCP }.into(),
            ))
        } else {
            self.have_iccp = true;
            drop(self.parse_iccp_raw());
            Ok(())
        }
    }

    pub(super) fn parse_iccp_raw(&mut self) -> Result<(), DecodingError> {
        let mut buf = &self.current_chunk.raw_bytes[..];

        // read profile name
        for len in 0..=80 {
            let raw: u8 = buf.read_be()?;
            if (raw == 0 && len == 0) || (raw != 0 && len == 80) {
                return Err(DecodingError::from(TextDecodingError::InvalidKeywordSize));
            }
            if raw == 0 {
                break;
            }
        }

        match buf.read_be()? {
            // compression method
            0u8 => (),
            n => {
                return Err(DecodingError::Format(
                    FormatErrorInner::UnknownCompressionMethod(n).into(),
                ));
            }
        }

        let mut limits = self.limits;
        let result = crate::sample::embedded_png::fdeflate::decompress_to_vec_bounded(
            buf,
            limits.bytes,
            |requested| limits.reserve_bytes(requested).is_ok(),
        );
        self.limits = limits;

        match result {
            Ok(profile) => {
                info_mut(&mut self.info)?.icc_profile = Some(Cow::Owned(profile));
            }
            Err(crate::sample::embedded_png::fdeflate::BoundedDecompressionError::DecompressionError { inner: err }) => {
                return Err(DecodingError::Format(
                    FormatErrorInner::CorruptFlateStream { err }.into(),
                ))
            }
            Err(crate::sample::embedded_png::fdeflate::BoundedDecompressionError::OutputTooLarge { .. }
                | crate::sample::embedded_png::fdeflate::BoundedDecompressionError::LimitsExceeded) => {
                return Err(DecodingError::LimitsExceeded);
            }
            Err(crate::sample::embedded_png::fdeflate::BoundedDecompressionError::Allocation {
                site,
                requested,
                source,
            }) => return Err(DecodingError::allocation(site, requested, source)),
        }

        Ok(())
    }
}
