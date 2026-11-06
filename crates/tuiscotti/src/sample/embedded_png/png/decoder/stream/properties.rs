use super::FormatError;
use super::*;

impl StreamingDecoder {
    pub(super) fn parse_phys(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::P_HYS }.into(),
            ))
        } else if info.pixel_dims.is_some() {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::P_HYS }.into(),
            ))
        } else {
            let mut buf = &self.current_chunk.raw_bytes[..];
            let xppu = buf.read_be()?;
            let yppu = buf.read_be()?;
            let unit = buf.read_be()?;
            let unit = match Unit::from_u8(unit) {
                Some(unit) => unit,
                None => {
                    return Err(DecodingError::Format(
                        FormatErrorInner::InvalidUnit(unit).into(),
                    ));
                }
            };
            let pixel_dims = PixelDimensions { xppu, yppu, unit };
            info.pixel_dims = Some(pixel_dims);
            Ok(())
        }
    }

    pub(super) fn parse_chrm(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::C_HRM }.into(),
            ))
        } else if info.chrm_chunk.is_some() {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::C_HRM }.into(),
            ))
        } else {
            let mut buf = &self.current_chunk.raw_bytes[..];
            let white_x: u32 = buf.read_be()?;
            let white_y: u32 = buf.read_be()?;
            let red_x: u32 = buf.read_be()?;
            let red_y: u32 = buf.read_be()?;
            let green_x: u32 = buf.read_be()?;
            let green_y: u32 = buf.read_be()?;
            let blue_x: u32 = buf.read_be()?;
            let blue_y: u32 = buf.read_be()?;

            let source_chromaticities = SourceChromaticities {
                white: (
                    ScaledFloat::from_scaled(white_x),
                    ScaledFloat::from_scaled(white_y),
                ),
                red: (
                    ScaledFloat::from_scaled(red_x),
                    ScaledFloat::from_scaled(red_y),
                ),
                green: (
                    ScaledFloat::from_scaled(green_x),
                    ScaledFloat::from_scaled(green_y),
                ),
                blue: (
                    ScaledFloat::from_scaled(blue_x),
                    ScaledFloat::from_scaled(blue_y),
                ),
            };

            info.chrm_chunk = Some(source_chromaticities);
            Ok(())
        }
    }

    pub(super) fn parse_gama(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::G_AMA }.into(),
            ))
        } else if info.gama_chunk.is_some() {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::G_AMA }.into(),
            ))
        } else {
            let mut buf = &self.current_chunk.raw_bytes[..];
            let source_gamma: u32 = buf.read_be()?;
            if source_gamma == 0 {
                return Err(DecodingError::Format(
                    FormatErrorInner::BadGammaValue.into(),
                ));
            }

            let source_gamma = ScaledFloat::from_scaled(source_gamma);
            info.gama_chunk = Some(source_gamma);
            Ok(())
        }
    }

    pub(super) fn parse_srgb(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::S_RGB }.into(),
            ))
        } else if info.srgb.is_some() {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::S_RGB }.into(),
            ))
        } else {
            let mut buf = &self.current_chunk.raw_bytes[..];
            let raw: u8 = buf.read_be()?; // BE is is nonsense for single bytes, but this way the size is checked.
            let rendering_intent = crate::sample::embedded_png::png::SrgbRenderingIntent::from_raw(
                raw,
            )
            .ok_or_else(|| FormatError::from(FormatErrorInner::InvalidSrgbRenderingIntent(raw)))?;

            // Set srgb and override source gamma and chromaticities.
            info.srgb = Some(rendering_intent);
            Ok(())
        }
    }
}
