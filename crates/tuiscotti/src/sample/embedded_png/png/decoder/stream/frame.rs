use super::*;

impl StreamingDecoder {
    pub(super) fn parse_fctl(&mut self) -> Result<(), DecodingError> {
        let mut buf = &self.current_chunk.raw_bytes[..];
        let next_seq_no = buf.read_be()?;

        // Assuming that fcTL is required before *every* fdAT-sequence
        self.current_seq_no = Some(if let Some(seq_no) = self.current_seq_no {
            if next_seq_no != seq_no + 1 {
                return Err(DecodingError::Format(
                    FormatErrorInner::ApngOrder {
                        expected: seq_no + 1,
                        present: next_seq_no,
                    }
                    .into(),
                ));
            }
            next_seq_no
        } else {
            if next_seq_no != 0 {
                return Err(DecodingError::Format(
                    FormatErrorInner::ApngOrder {
                        expected: 0,
                        present: next_seq_no,
                    }
                    .into(),
                ));
            }
            0
        });
        self.inflater.reset();
        self.ready_for_fdat_chunks = self.have_idat;
        let fc = FrameControl {
            sequence_number: next_seq_no,
            width: buf.read_be()?,
            height: buf.read_be()?,
            x_offset: buf.read_be()?,
            y_offset: buf.read_be()?,
            delay_num: buf.read_be()?,
            delay_den: buf.read_be()?,
            dispose_op: {
                let dispose_op = buf.read_be()?;
                match DisposeOp::from_u8(dispose_op) {
                    Some(dispose_op) => dispose_op,
                    None => {
                        return Err(DecodingError::Format(
                            FormatErrorInner::InvalidDisposeOp(dispose_op).into(),
                        ));
                    }
                }
            },
            blend_op: {
                let blend_op = buf.read_be()?;
                match BlendOp::from_u8(blend_op) {
                    Some(blend_op) => blend_op,
                    None => {
                        return Err(DecodingError::Format(
                            FormatErrorInner::InvalidBlendOp(blend_op).into(),
                        ));
                    }
                }
            },
        };
        info_ref(&self.info)?.validate(&fc)?;
        if !self.have_idat {
            info_ref(&self.info)?.validate_default_image(&fc)?;
        }
        info_mut(&mut self.info)?.frame_control = Some(fc);
        Ok(())
    }

    pub(super) fn parse_actl(&mut self) -> Result<(), DecodingError> {
        let info = info_mut(&mut self.info)?;
        if self.have_idat {
            Err(DecodingError::Format(
                FormatErrorInner::AfterIdat { kind: chunk::AC_TL }.into(),
            ))
        } else if info.animation_control.is_some() {
            Err(DecodingError::Format(
                FormatErrorInner::DuplicateChunk { kind: chunk::AC_TL }.into(),
            ))
        } else {
            let mut buf = &self.current_chunk.raw_bytes[..];
            let actl = AnimationControl {
                num_frames: buf.read_be()?,
                num_plays: buf.read_be()?,
            };
            // The spec says that "0 is not a valid value" for `num_frames`.
            // So let's ignore such malformed `acTL` chunks.
            if actl.num_frames == 0 {
                return Ok(());
            }

            // The spec also says that the number of frames and number of plays should be limited
            // to (2^31)-1. Same as the other condition we enforce it by ignoring the chunk.
            // Another option may be saturation which would lose us some frames but encourage
            // rather dubious handling.
            if actl.num_frames > 0x7FFFFFFF || actl.num_plays > 0x7FFFFFFF {
                return Ok(());
            }

            info.animation_control = Some(actl);
            Ok(())
        }
    }
}
